use anyhow::Result;

use crate::project::Project;
use crate::{actions, apps};
use crate::config::{self, Pick};
use crate::lease::{self, sticky::Last, Book, Standing};
use crate::model::{self, Platform, Reach, Unix, View};
use crate::registry::Registry;

#[derive(Clone, Debug)]
pub struct Seat {
    pub view: View,
    pub standing: Standing,
    pub pick: Pick,
    pub pooled: bool,
    pub warm: bool,
    pub installed: bool,
    pub current: bool,
    pub clones: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    Use(usize, Option<String>),
    Boot(usize),
    Clone(usize),
    Refuse(String),
}

impl Seat {
    fn free(&self) -> bool {
        matches!(self.standing, Standing::Free(_))
    }

    fn mine(&self) -> bool {
        matches!(self.standing, Standing::Mine(_))
    }

    fn running(&self) -> bool {
        actions::running(&self.view.reach)
    }

    fn bootable(&self) -> bool {
        self.view.reach == Reach::Off
            && matches!(self.view.device.platform, Platform::Emulator | Platform::Simulator)
    }

    fn pool(&self) -> bool {
        self.free() && self.pooled && matches!(self.pick, Pick::First | Pick::Normal)
    }

    fn order(&self) -> (Pick, bool, bool, bool) {
        (self.pick, !self.warm, !self.installed, !self.current)
    }

    fn reachable(&self, boot: bool) -> bool {
        self.running() || boot && self.bootable()
    }

    fn cloneable(&self) -> bool {
        self.clones && self.pooled && self.pick != Pick::Never && self.bootable()
    }

    fn last_resort(&self) -> bool {
        (self.free() || self.mine()) && self.pick == Pick::Last && self.running()
    }
}

fn best(seats: &[Seat], fit: impl Fn(&Seat) -> bool) -> Option<usize> {
    seats
        .iter()
        .enumerate()
        .filter(|(_, s)| fit(s))
        .min_by_key(|(_, s)| s.order())
        .map(|(i, _)| i)
}

pub fn plan(seats: &[Seat], sticky: Option<&str>, boot: bool) -> Plan {
    if let Some(i) = best(seats, |s| s.mine() && s.pick.sticks() && s.reachable(boot)) {
        return match seats[i].running() {
            true => Plan::Use(i, None),
            false => Plan::Boot(i),
        };
    }

    let sticky = sticky.and_then(|id| {
        best(seats, |s| s.view.device.id == id && s.free() && s.pick.sticks())
    });

    let warm = sticky
        .filter(|&i| seats[i].running())
        .or_else(|| best(seats, |s| s.pool() && s.running()));

    if let Some(i) = warm {
        return Plan::Use(i, None);
    }

    let cold = sticky
        .filter(|&i| seats[i].bootable())
        .or_else(|| best(seats, |s| s.pool() && s.bootable()));

    match boot {
        true => booted(seats, cold),
        false => last(seats, cold),
    }
}

fn booted(seats: &[Seat], cold: Option<usize>) -> Plan {
    match cold.map(Plan::Boot).or_else(|| best(seats, Seat::cloneable).map(Plan::Clone)) {
        Some(plan) => plan,
        None => last(seats, None),
    }
}

fn last(seats: &[Seat], cold: Option<usize>) -> Plan {
    let Some(i) = best(seats, Seat::last_resort).filter(|_| cold.is_none()) else {
        return Plan::Refuse(refused(seats, cold.map(|i| &seats[i])));
    };

    let why = format!(
        "{} is a last resort, picked because {}",
        seats[i].view.device.label,
        busy(seats)
    );

    Plan::Use(i, Some(why))
}

fn held(seats: &[Seat], now: Unix) -> Vec<String> {
    seats
        .iter()
        .filter_map(|s| match &s.standing {
            Standing::Held(lease) => Some(format!("{} by {}", s.view.device.label, lease.describe(now))),
            _ => None,
        })
        .collect()
}

fn busy(seats: &[Seat]) -> String {
    let held = held(seats, model::now());

    match held.is_empty() {
        true => "no pooled device is free and running".to_string(),
        false => format!("every pooled device is held or off: {}", held.join(", ")),
    }
}

fn refused(seats: &[Seat], cold: Option<&Seat>) -> String {
    if seats.is_empty() {
        return "no device reachable to allocate (check: phone device list, tailscale status)".to_string();
    }

    let held = held(seats, model::now());
    let mut said = match held.is_empty() {
        true => "no device is free to allocate".to_string(),
        false => format!("no free device: {}", held.join(", ")),
    };

    said.push_str(&match cold {
        Some(s) => format!("; {}, or `phone up` allocates one", offer(s)),
        None => "; a holder's `phone release` frees one, `--take` takes one".to_string(),
    });

    said
}

fn offer(seat: &Seat) -> String {
    let label = &seat.view.device.label;
    let target = crate::quoted(label);

    match seat.running() {
        true => format!("{label} is free: `-t {target}`"),
        false => format!("{label} is free (off): `phone device boot -t {target}`"),
    }
}

pub fn alternative(seats: &[Seat]) -> Option<String> {
    best(seats, |s| s.pool() && s.running())
        .or_else(|| best(seats, |s| s.pool() && s.bootable()))
        .or_else(|| best(seats, Seat::last_resort))
        .map(|i| offer(&seats[i]))
}

fn recalled(seats: &[Seat], last: &Last) -> bool {
    seats
        .iter()
        .find(|s| s.view.device.id == last.device)
        .is_none_or(|s| s.pick.sticks())
}

pub fn displaced(seats: &[Seat], last: &Last) -> Option<String> {
    let seat = seats.iter().find(|s| s.view.device.id == last.device)?;

    match &seat.standing {
        Standing::Held(lease) => Some(format!(
            "your {} was taken by {} after {} idle",
            last.label,
            lease.label,
            lease::span(lease.since - last.seen)
        )),
        _ => None,
    }
}

pub fn stranded(seats: &[Seat], last: &Last, boot: bool) -> Option<String> {
    if boot || seats.iter().any(|s| s.mine() && s.running()) {
        return None;
    }

    let note = displaced(seats, last)?;

    Some(match alternative(seats) {
        Some(instead) => format!("{note}; {instead}"),
        None => format!("{note}; no other device is free to allocate"),
    })
}

pub enum Chosen {
    Use(View),
    Boot(View),
    Clone(View),
}

pub async fn choose(
    views: &[View],
    reg: &Registry,
    project: Option<&Project>,
    oses: &[&'static str],
    boot: bool,
) -> Result<Chosen> {
    let seats = seated(views, reg, project, oses).await;
    let last = lease::sticky::last().filter(|l| recalled(&seats, l));

    if let Some(why) = last.as_ref().and_then(|l| stranded(&seats, l, boot)) {
        lease::sticky::forget();

        return Err(crate::Refused(why).into());
    }

    if let Some(note) = last.as_ref().and_then(|l| displaced(&seats, l)) {
        eprintln!("phone: {note}");
    }

    let picked = |i: usize| seats[i].view.clone();

    Ok(match plan(&seats, last.as_ref().map(|l| l.device.as_str()), boot) {
        Plan::Use(i, why) => {
            if let Some(why) = why {
                eprintln!("phone: {why}");
            }

            Chosen::Use(picked(i))
        }
        Plan::Boot(i) => Chosen::Boot(picked(i)),
        Plan::Clone(i) => Chosen::Clone(picked(i)),
        Plan::Refuse(why) => return Err(crate::Refused(why).into()),
    })
}

async fn seated(views: &[View], reg: &Registry, project: Option<&Project>, oses: &[&'static str]) -> Vec<Seat> {
    let (books, tree) = tokio::join!(lease::books(views), lease::here_tree());
    let wants = Wants {
        oses,
        tree: tree.as_deref(),
        project,
    };

    seats(views, &books, reg, &wants)
}

pub async fn instead(views: &[View], reg: &Registry, project: Option<&Project>, oses: &[&'static str]) -> Option<String> {
    alternative(&seated(views, reg, project, oses).await)
}

struct Wants<'a> {
    oses: &'a [&'static str],
    tree: Option<&'a str>,
    project: Option<&'a Project>,
}

fn seats(views: &[View], books: &[Book], reg: &Registry, wants: &Wants) -> Vec<Seat> {
    let config = config::get();

    views
        .iter()
        .filter(|v| wants.oses.contains(&v.device.platform.os()))
        .filter(|v| config.pick(&v.device) != Pick::Never)
        .filter_map(|v| {
            let book = lease::book_of(books, &v.device)?;
            let key = lease::key(&v.device).id;
            let app = wants
                .project
                .and_then(|p| p.manifest.build.get(v.device.platform.os()))
                .and_then(|b| apps::app_id(&b.app).ok().map(str::to_string));

            Some(Seat {
                view: v.clone(),
                standing: book.standing(&v.device, actions::running(&v.reach)),
                pick: config.pick(&v.device),
                pooled: config.pooled(&v.device),
                warm: book.usage.of(&v.device, wants.tree).project.is_some(),
                installed: app.is_some_and(|app| {
                    book.ledger.installed.get(&key).is_some_and(|set| set.contains(&app))
                }),
                current: reg.current.as_deref() == Some(v.device.id.as_str()),
                clones: config.clones(v.device.host.as_deref()),
            })
        })
        .collect()
}

pub fn clone_name(views: &[View], source: &View) -> String {
    let taken = |name: &str| {
        views
            .iter()
            .any(|v| v.device.host == source.device.host && v.device.label.eq_ignore_ascii_case(name))
    };

    (2..)
        .map(|n| format!("{}-{n}", source.device.label))
        .find(|name| !taken(name))
        .expect("an unbounded range has a free name")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Agent, Harness};
    use crate::lease::Lease;
    use crate::usage::tests::on_rose;
    use std::time::Duration;

    fn attached() -> Reach {
        Reach::Attached {
            serial: "emulator-5554".into(),
            wireless: false,
        }
    }

    fn holder() -> Lease {
        let other = Agent {
            id: "other".into(),
            label: "other".into(),
            host: "nyx".into(),
            harness: Harness::Claude,
            pid: None,
            start: None,
            shared: false,
        };

        Lease::of(&other, Duration::from_secs(1200), model::now())
    }

    fn seat(name: &str, reach: Reach) -> Seat {
        Seat {
            view: on_rose(&format!("avd:rose/{name}"), name, Platform::Emulator, reach),
            standing: Standing::Free(None),
            pick: Pick::Normal,
            pooled: true,
            warm: false,
            installed: false,
            current: false,
            clones: false,
        }
    }

    fn taken(mut s: Seat) -> Seat {
        s.standing = Standing::Held(holder());
        s
    }

    fn label(seats: &[Seat], plan: &Plan) -> String {
        match plan {
            Plan::Use(i, _) | Plan::Boot(i) | Plan::Clone(i) => seats[*i].view.device.label.clone(),
            Plan::Refuse(why) => why.clone(),
        }
    }

    #[test]
    fn the_sticky_device_comes_back_when_it_is_free() {
        let seats = [seat("a", attached()), seat("b", attached())];
        let plan = plan(&seats, Some("avd:rose/b"), false);

        assert_eq!(plan, Plan::Use(1, None));
    }

    #[test]
    fn a_device_already_held_beats_the_sticky_one() {
        let mut seats = [seat("a", attached()), seat("b", attached())];
        seats[0].standing = Standing::Mine(holder());

        assert_eq!(plan(&seats, Some("avd:rose/b"), false), Plan::Use(0, None));
    }

    #[test]
    fn a_held_last_resort_yields_to_a_free_pool_device() {
        let mut seats = [seat("faraday", attached()), seat("a", attached())];
        seats[0].pick = Pick::Last;
        seats[0].standing = Standing::Mine(holder());

        assert_eq!(label(&seats, &plan(&seats, None, false)), "a");

        seats[1] = taken(seats[1].clone());

        assert_eq!(label(&seats, &plan(&seats, None, false)), "faraday");
    }

    #[test]
    fn a_free_running_device_warm_for_the_project_wins_then_one_with_the_app() {
        let mut seats = [seat("cold", attached()), seat("app", attached()), seat("warm", attached())];
        seats[1].installed = true;
        seats[2].warm = true;

        assert_eq!(label(&seats, &plan(&seats, None, false)), "warm");

        seats[2].standing = Standing::Held(holder());

        assert_eq!(label(&seats, &plan(&seats, None, false)), "app");
    }

    #[test]
    fn a_taken_sticky_falls_through_to_the_pool() {
        let seats = [taken(seat("a", attached())), seat("b", attached())];

        assert_eq!(label(&seats, &plan(&seats, Some("avd:rose/a"), false)), "b");
    }

    #[test]
    fn with_nothing_running_free_an_off_pool_device_is_booted() {
        let seats = [taken(seat("a", attached())), seat("b", Reach::Off)];

        assert_eq!(plan(&seats, None, true), Plan::Boot(1));
    }

    #[test]
    fn a_verb_that_does_not_boot_names_the_device_it_could() {
        let seats = [taken(seat("a", attached())), seat("b", Reach::Off)];
        let Plan::Refuse(why) = plan(&seats, None, false) else {
            panic!("refused");
        };

        assert!(why.contains("a by other"), "{why}");
        assert!(why.contains("b is free (off): `phone device boot -t b`"), "{why}");
    }

    #[test]
    fn a_last_resort_waits_while_a_pool_device_is_only_off() {
        let mut seats = [
            seat("faraday", attached()),
            taken(seat("a", attached())),
            seat("b", Reach::Off),
        ];
        seats[0].pick = Pick::Last;

        let Plan::Refuse(why) = plan(&seats, None, false) else {
            panic!("refused while b is off");
        };

        assert!(why.contains("`phone device boot -t b`"), "{why}");
        assert!(!why.contains("faraday"), "{why}");
        assert_eq!(plan(&seats, None, true), Plan::Boot(2));
    }

    #[test]
    fn a_last_resort_is_never_recalled_as_the_sticky_device() {
        let mut seats = [
            seat("faraday", attached()),
            taken(seat("a", attached())),
            seat("b", Reach::Off),
        ];
        seats[0].pick = Pick::Last;

        let last = Last {
            device: "avd:rose/faraday".into(),
            label: "faraday".into(),
            seen: model::now(),
        };

        assert!(!recalled(&seats, &last));
        assert!(matches!(plan(&seats, Some("avd:rose/faraday"), false), Plan::Refuse(_)));
        assert_eq!(plan(&seats, Some("avd:rose/faraday"), true), Plan::Boot(2));

        seats[0].pick = Pick::Normal;

        assert!(recalled(&seats, &last));
        assert_eq!(plan(&seats, Some("avd:rose/faraday"), false), Plan::Use(0, None));
    }

    #[test]
    fn the_alternative_offers_running_then_off_pool_then_a_last_resort() {
        let mut seats = [
            seat("faraday", attached()),
            seat("b", Reach::Off),
            seat("c", attached()),
        ];
        seats[0].pick = Pick::Last;

        let mut offered = Vec::new();

        for i in [2, 1, 0] {
            offered.push(alternative(&seats));
            seats[i] = taken(seats[i].clone());
        }

        offered.push(alternative(&seats));

        assert_eq!(
            offered,
            [
                Some("c is free: `-t c`".to_string()),
                Some("b is free (off): `phone device boot -t b`".to_string()),
                Some("faraday is free: `-t faraday`".to_string()),
                None,
            ]
        );
    }

    #[test]
    fn a_clone_happens_only_on_a_host_that_allows_it() {
        let mut seats = [taken(seat("a", Reach::Off)), taken(seat("b", attached()))];

        assert!(matches!(plan(&seats, None, true), Plan::Refuse(_)));

        seats[0].clones = true;

        assert_eq!(plan(&seats, None, true), Plan::Clone(0));
    }

    #[test]
    fn a_last_resort_is_picked_only_after_the_pool_and_says_why() {
        let mut seats = [seat("faraday", attached()), seat("a", attached())];
        seats[0].pick = Pick::Last;

        assert_eq!(plan(&seats, None, false), Plan::Use(1, None));

        seats[1].standing = Standing::Held(holder());

        let Plan::Use(0, Some(why)) = plan(&seats, None, false) else {
            panic!("faraday as the last resort");
        };

        assert!(why.contains("last resort"), "{why}");
        assert!(why.contains("a by other"), "{why}");
    }

    #[test]
    fn a_never_device_is_not_allocated_even_when_sticky() {
        let mut seats = [seat("never", attached())];
        seats[0].pick = Pick::Never;

        assert!(matches!(plan(&seats, Some("avd:rose/never"), true), Plan::Refuse(_)));
    }

    #[test]
    fn a_first_device_beats_a_warm_normal_one() {
        let mut seats = [seat("warm", attached()), seat("first", attached())];
        seats[0].warm = true;
        seats[1].pick = Pick::First;

        assert_eq!(label(&seats, &plan(&seats, None, false)), "first");
    }

    #[test]
    fn a_taken_sticky_device_is_reported_with_how_long_it_sat() {
        let mut lease = holder();
        lease.since = 1000 + 7 * 60;

        let mut seats = [seat("a", attached())];
        seats[0].standing = Standing::Held(lease);

        let last = Last {
            device: "avd:rose/a".into(),
            label: "a".into(),
            seen: 1000,
        };

        assert_eq!(
            displaced(&seats, &last).as_deref(),
            Some("your a was taken by other after 7m idle")
        );
    }

    #[test]
    fn a_verb_whose_sticky_device_was_taken_stops_and_names_a_free_one() {
        let mut seats = [taken(seat("a", attached())), seat("b", attached())];
        let last = Last {
            device: "avd:rose/a".into(),
            label: "a".into(),
            seen: model::now(),
        };

        let why = stranded(&seats, &last, false).unwrap();

        assert!(why.starts_with("your a was taken by other"), "{why}");
        assert!(why.contains("b is free: `-t b`"), "{why}");
        assert_eq!(stranded(&seats, &last, true), None, "up reallocates and converges");

        seats[1].standing = Standing::Mine(holder());

        assert_eq!(stranded(&seats, &last, false), None, "it already holds another device");
    }

    #[test]
    fn a_clone_is_named_after_its_source_without_colliding() {
        let views = [
            on_rose("avd:rose/p", "p", Platform::Emulator, Reach::Off),
            on_rose("avd:rose/p-2", "p-2", Platform::Emulator, Reach::Off),
        ];

        assert_eq!(clone_name(&views, &views[0]), "p-3");
    }
}
