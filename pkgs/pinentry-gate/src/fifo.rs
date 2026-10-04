//! Writing a decision on a request's fifo: never creating it, and never
//! waiting on one whose reader is gone.

use std::fs::File;
use std::io::Write;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

use rustix::fs::{Mode, OFlags};

fn open(fifo: &Path) -> Option<File> {
    let fd = rustix::fs::open(fifo, OFlags::WRONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW, Mode::empty()).ok()?;
    let file = File::from(fd);
    file.metadata().is_ok_and(|meta| meta.file_type().is_fifo()).then_some(file)
}

pub fn live(fifo: &Path) -> bool {
    open(fifo).is_some()
}

pub fn deliver(fifo: &Path, line: &str) -> bool {
    let Some(mut file) = open(fifo) else {
        return false;
    };
    let _ = rustix::fs::fcntl_setfl(&file, OFlags::empty());
    file.write_all(line.as_bytes()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pinentry-gate-fifo-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn mkfifo(path: &Path) {
        rustix::fs::mkfifoat(rustix::fs::CWD, path, Mode::from_raw_mode(0o600)).unwrap();
    }

    fn reader(path: &Path) -> File {
        OpenOptions::new().read(true).write(true).custom_flags(OFlags::NONBLOCK.bits() as i32).open(path).unwrap()
    }

    #[test]
    fn a_reader_gets_the_line() {
        let path = scratch("reader").join("req");
        mkfifo(&path);
        let mut held = reader(&path);
        assert!(live(&path));
        assert!(deliver(&path, "secret\n"));
        let mut buf = [0u8; 32];
        let n = held.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"secret\n");
    }

    #[test]
    fn a_fifo_nobody_reads_is_not_waited_on() {
        let path = scratch("dead").join("req");
        mkfifo(&path);
        let started = Instant::now();
        assert!(!live(&path));
        assert!(!deliver(&path, "secret\n"));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_regular_file_is_never_written() {
        let path = scratch("regular").join("req");
        std::fs::write(&path, "kept").unwrap();
        assert!(!live(&path));
        assert!(!deliver(&path, "secret\n"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "kept");
    }

    #[test]
    fn a_missing_path_is_never_created() {
        let path = scratch("missing").join("req");
        assert!(!deliver(&path, "secret\n"));
        assert!(!path.exists());
    }
}
