use std::net::{IpAddr, SocketAddr};
use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;

/// Android hands adbd an ephemeral port, so wireless debugging lands somewhere
/// in the kernel's local port range rather than on a fixed number.
pub const EPHEMERAL: RangeInclusive<u16> = 32768..=61000;

pub struct Sweep {
    pub concurrency: usize,
    pub timeout: Duration,
}

impl Default for Sweep {
    fn default() -> Self {
        // over the tailnet each probe costs a WireGuard round trip, so the win
        // comes from width, not from a shorter timeout.
        Self {
            concurrency: 512,
            timeout: Duration::from_millis(900),
        }
    }
}

impl Sweep {
    /// Every port in `range` that accepts a TCP connection, lowest first. A hit
    /// is only a candidate: adbd is not the sole listener a phone can have, so
    /// callers still have to confirm with `adb connect`.
    pub async fn scan<F>(
        &self,
        host: &str,
        range: RangeInclusive<u16>,
        mut on_progress: F,
    ) -> Result<Vec<u16>>
    where
        F: FnMut(usize, usize),
    {
        let ip: IpAddr = host
            .parse()
            .with_context(|| format!("sweep needs a literal IP, got {host}"))?;

        let total = (*range.end() as usize).saturating_sub(*range.start() as usize) + 1;
        let sem = Arc::new(Semaphore::new(self.concurrency));
        let done = Arc::new(AtomicUsize::new(0));
        let timeout = self.timeout;

        let mut tasks = tokio::task::JoinSet::new();

        for port in range {
            let sem = sem.clone();
            let done = done.clone();

            tasks.spawn(async move {
                let _permit = sem.acquire_owned().await.ok()?;

                let hit =
                    tokio::time::timeout(timeout, TcpStream::connect(SocketAddr::new(ip, port)))
                        .await
                        .is_ok_and(|r| r.is_ok());

                done.fetch_add(1, Ordering::Relaxed);

                hit.then_some(port)
            });
        }

        let mut open = Vec::new();
        let mut reported = 0;

        while let Some(res) = tasks.join_next().await {
            if let Ok(Some(port)) = res {
                open.push(port);
            }

            let seen = done.load(Ordering::Relaxed);

            // one callback per percent; the TUI redraws on it
            if seen * 100 / total.max(1) != reported * 100 / total.max(1) {
                reported = seen;
                on_progress(seen, total);
            }
        }

        on_progress(total, total);
        open.sort_unstable();

        Ok(open)
    }
}

/// Single-port reachability check, used to rank remembered endpoints before
/// spending a five second `adb connect` on each of them.
pub async fn probe(host: &str, port: u16, timeout: Duration) -> bool {
    let addr = format!("{host}:{port}");

    match tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}
