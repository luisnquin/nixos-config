use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::a11y::{self, Keyboard, Node, Row, Screen, Signature, Target};
use crate::actions::{Settled, Settling};

const STEP: Duration = Duration::from_millis(350);

#[derive(Debug, PartialEq)]
pub enum How {
    Moved,
    Unmoved,
    Restless,
}

pub struct Change {
    pub screen: Screen,
    pub how: How,
    pub appeared: Vec<usize>,
    pub gone: Vec<String>,
}

#[derive(Default, PartialEq)]
struct Look {
    rows: Vec<Signature>,
    keyboard: Option<Keyboard>,
}

fn look(screen: &Screen) -> Look {
    Look {
        rows: screen.nodes.iter().map(Node::signature).collect(),
        keyboard: screen.keyboard,
    }
}

pub async fn after(t: &Target, before: &Screen) -> Result<Change> {
    let started = Instant::now();
    let first = a11y::dump(t).await?;
    let mut settling = Settling::with(Some(look(before)), look(&first), |a, b| a == b);

    loop {
        tokio::time::sleep(STEP).await;

        let screen = a11y::dump(t).await?;

        let how = match settling.see(look(&screen), started.elapsed()) {
            Settled::Still => continue,
            Settled::Done(_) => How::Moved,
            Settled::Unmoved(_) => How::Unmoved,
            Settled::Restless(_) => How::Restless,
        };

        return Ok(compare(before, screen, how));
    }
}

pub fn compare(before: &Screen, screen: Screen, how: How) -> Change {
    let was = listed(before);
    let now = listed(&screen);
    let appeared = fresh(&now, &was).iter().map(|r| r.node.index).collect();
    let gone = fresh(&was, &now).iter().map(|r| r.label.clone()).collect();

    Change {
        screen,
        how,
        appeared,
        gone,
    }
}

type Identity<'a> = (&'a str, &'a str, &'a str, &'a str);

fn identity<'a>(r: &Row<'a>) -> Identity<'a> {
    (&r.node.res_id, &r.node.class, &r.node.text, &r.node.desc)
}

fn listed(screen: &Screen) -> Vec<Row<'_>> {
    a11y::rows(&screen.nodes)
        .into_iter()
        .filter(|r| r.within.is_none())
        .collect()
}

/// Rows of `from` that `against` has no counterpart for. Where a row sits is
/// left out, so a scroll reports what came into view rather than every row.
fn fresh<'a, 'b>(from: &'b [Row<'a>], against: &[Row<'a>]) -> Vec<&'b Row<'a>> {
    let mut left: HashMap<Identity, usize> = HashMap::new();

    for n in against {
        *left.entry(identity(n)).or_default() += 1;
    }

    from.iter()
        .filter(|n| match left.get_mut(&identity(n)) {
            Some(count) if *count > 0 => {
                *count -= 1;
                false
            }
            _ => true,
        })
        .collect()
}

pub fn number(shown: &mut Vec<Signature>, rows: &[&Node]) -> Vec<usize> {
    rows.iter()
        .map(|n| {
            let row = n.signature();

            shown.iter().position(|s| *s == row).unwrap_or_else(|| {
                shown.push(row);
                shown.len() - 1
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(xml: &str) -> Screen {
        Screen {
            nodes: a11y::parse(xml).unwrap(),
            keyboard: None,
        }
    }

    fn row(text: &str, clickable: bool, y: i32) -> String {
        format!(
            r#"<node class="android.widget.TextView" bounds="[40,{y}][1040,{}]" clickable="{clickable}" text="{text}" content-desc="" resource-id=""/>"#,
            y + 80
        )
    }

    fn hierarchy(rows: &[String]) -> String {
        format!(
            r#"<hierarchy rotation="0"><node class="android.widget.FrameLayout" bounds="[0,0][1080,2400]" clickable="false" text="" content-desc="" resource-id="">{}</node></hierarchy>"#,
            rows.concat()
        )
    }

    const TABS: [&str; 4] = ["Inicio", "Operaciones", "Préstamos", "Configuración"];

    fn home() -> Screen {
        let mut rows = vec![
            row("Operaciones en curso", false, 360),
            row("Viaje QA Cusco, colecta fija", true, 740),
            row("Tus préstamos", false, 1180),
            row("Ver Con meta fija", true, 2090),
        ];
        rows.extend(TABS.iter().map(|t| row(t, true, 2200)));

        screen(&hierarchy(&rows))
    }

    fn form() -> Screen {
        let mut rows = vec![
            row("Nueva colecta", false, 200),
            row("Nombre", true, 500),
            row("Meta", true, 700),
            row("Continuar", true, 2090),
        ];
        rows.extend(TABS.iter().map(|t| row(t, true, 2200)));

        screen(&hierarchy(&rows))
    }

    #[test]
    fn a_tap_that_opens_a_screen_names_what_came_and_what_went() {
        let change = compare(&home(), form(), How::Moved);
        let labels = |at: &[usize]| -> Vec<String> {
            at.iter().map(|&i| change.screen.nodes[i].label()).collect()
        };

        assert_eq!(
            labels(&change.appeared),
            ["Nueva colecta", "Nombre", "Meta", "Continuar"]
        );
        assert_eq!(
            change.gone,
            [
                "Operaciones en curso",
                "Viaje QA Cusco, colecta fija",
                "Tus préstamos",
                "Ver Con meta fija"
            ]
        );
    }

    #[test]
    fn a_scroll_reports_the_rows_that_came_into_view_not_the_ones_that_moved() {
        let before = screen(&hierarchy(&[row("A", false, 100), row("B", false, 300)]));
        let after = screen(&hierarchy(&[row("B", false, 100), row("C", false, 300)]));

        let change = compare(&before, after, How::Moved);

        assert_eq!(change.appeared, [1]);
        assert_eq!(change.gone, ["A"]);
    }

    #[test]
    fn a_second_row_of_one_name_counts_as_new() {
        let before = screen(&hierarchy(&[row("Item", false, 100)]));
        let after = screen(&hierarchy(&[
            row("Item", false, 100),
            row("Item", false, 300),
        ]));

        assert_eq!(compare(&before, after, How::Moved).appeared, [1]);
    }

    #[test]
    fn new_rows_are_numbered_after_the_last_snapshot_and_old_ones_keep_theirs() {
        let home = home();
        let form = form();
        let mut shown: Vec<Signature> = home.nodes.iter().map(Node::signature).collect();

        let back: Vec<&Node> = vec![&home.nodes[3], &form.nodes[0]];
        let at = number(&mut shown, &back);

        assert_eq!(at, [3, home.nodes.len()]);
        assert_eq!(shown.len(), home.nodes.len() + 1);
        assert_eq!(
            a11y::pick_in(&form.nodes, &format!("@{}", at[1]), Some(&shown))
                .unwrap()
                .label(),
            "Nueva colecta"
        );
    }
}
