use std::process::Stdio;

use anyhow::{anyhow, bail, Context, Result};
use tokio::io::AsyncWriteExt;

use crate::discover::scoped;
use crate::model::View;

/// fzf reads the candidates from a pipe but draws on /dev/tty, so the prompt
/// still works when `phone` is itself inside a pipeline. With no terminal at all
/// — a script, an agent, a cron job — there is nothing to draw on, and asking
/// gets `inappropriate ioctl for device` instead of an answer. The ambiguity has
/// to come back as an error that names the way out of it.
pub async fn pick(views: &[View], prompt: &str) -> Result<usize> {
    if views.is_empty() {
        bail!("no device to choose from");
    }

    if views.len() == 1 {
        return Ok(0);
    }

    if std::fs::File::open("/dev/tty").is_err() {
        bail!("{}", ambiguity(views));
    }

    let rows: String = views
        .iter()
        .enumerate()
        .map(|(i, v)| {
            format!(
                "{i}\t{:<20} {:<20} {:<14} {}\n",
                v.device.label,
                v.device.model,
                v.reach.label(),
                v.device.platform
            )
        })
        .collect();

    let mut child = tokio::process::Command::new("fzf")
        .args([
            "--delimiter=\t",
            "--with-nth=2..",
            &format!("--prompt={prompt}> "),
            "--header=select a device",
            "--height=40%",
            "--reverse",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("running fzf")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(rows.as_bytes()).await?;
        stdin.shutdown().await?;
    }

    let out = child.wait_with_output().await?;

    if !out.status.success() {
        bail!("cancelled");
    }

    String::from_utf8_lossy(&out.stdout)
        .split('\t')
        .next()
        .and_then(|i| i.trim().parse::<usize>().ok())
        .ok_or_else(|| anyhow!("unreadable selection"))
}

fn ambiguity(views: &[View]) -> String {
    let rows: Vec<String> = views
        .iter()
        .map(|v| {
            let serial = v
                .reach
                .serial()
                .map(|s| scoped(&v.server, s))
                .unwrap_or_else(|| "-".into());

            let row = format!(
                "  {:<32} {:<20} {:<16} {serial}",
                v.device.id,
                v.device.label,
                v.reach.label()
            );

            row.trim_end().to_string()
        })
        .collect();

    format!(
        "{} devices match, and there is no terminal to choose on; \
         name one by the id on the left, or set PHONE_TARGET:\n{}",
        views.len(),
        rows.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adb::Server;
    use crate::model::{Device, Platform, Reach};

    #[test]
    fn an_unanswerable_prompt_says_which_of_the_rows_is_running_and_where() {
        let mac = Server::Remote {
            host: "mac".into(),
            port: 5038,
        };

        let views = [
            View::new(
                Device::new("android_id:1111aaaa", "pixel_7-api36", Platform::Emulator),
                Reach::Attached {
                    serial: "emulator-5554".into(),
                    wireless: false,
                },
            )
            .on(mac),
            View::new(
                Device::new("android_id:2222bbbb", "pixel_7-api36", Platform::Emulator),
                Reach::Known,
            ),
        ];

        let text = ambiguity(&views);
        let lines: Vec<&str> = text.lines().skip(1).collect();

        assert!(lines[0].contains("attached/emu") && lines[0].ends_with("mac/emulator-5554"));
        assert!(lines[1].contains("known") && lines[1].ends_with(" -"));
    }
}
