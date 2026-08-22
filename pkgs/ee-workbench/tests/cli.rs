use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// A workbench per test, wired through the same environment variables an
/// operator would use, so the tests exercise the real path resolution.
struct Bench {
    home: PathBuf,
}

impl Bench {
    fn new(name: &str) -> Self {
        let home = std::env::temp_dir().join(format!(
            "ee-workbench-test-{}-{name}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));

        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();

        let bench = Self { home };

        bench.ok(&["repo", "init"]);

        bench
    }

    fn data(&self) -> PathBuf {
        self.home.join("data")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ee"))
            .args(args)
            .env("HOME", &self.home)
            .env("EE_WORKBENCH_DATA", self.data())
            .env("XDG_STATE_HOME", self.home.join("state"))
            .env("XDG_CONFIG_HOME", self.home.join("config"))
            .env("XDG_CACHE_HOME", self.home.join("cache"))
            .output()
            .expect("running ee")
    }

    fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args);

        assert!(
            output.status.success(),
            "ee {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        String::from_utf8(output.stdout).unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let mut args = args.to_vec();
        args.push("--json");

        serde_json::from_str(&self.ok(&args)).unwrap()
    }

    fn fails(&self, args: &[&str]) -> String {
        let output = self.run(args);

        assert!(
            !output.status.success(),
            "ee {args:?} unexpectedly succeeded"
        );

        String::from_utf8(output.stderr).unwrap()
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();

    names.sort();
    names
}

#[test]
fn init_lays_out_the_repository_without_committing() {
    let bench = Bench::new("init");

    for path in [
        ".ee-workbench",
        "projects",
        "inventory/parts",
        "inventory/events",
        "experiments",
        "measurements",
        ".git",
    ] {
        assert!(bench.data().join(path).exists(), "missing {path}");
    }

    // `git init` and nothing else: the first commit stays the operator's.
    let log = bench.run(&["git", "log", "--oneline"]);
    assert!(!log.status.success());

    let status = bench.json(&["repo", "status"]);
    assert_eq!(status["counts"]["projects"], 0);
    // Git tracks no empty directory, so a fresh workbench is exactly one
    // untracked path: the marker that makes the directory a workbench.
    assert_eq!(
        status["dirty"].as_array().unwrap(),
        &vec![Value::from("?? .ee-workbench")]
    );

    assert_eq!(
        bench.ok(&["repo", "path"]).trim(),
        bench.data().display().to_string()
    );
}

#[test]
fn the_ledger_is_append_only_and_stock_is_summed_from_it() {
    let bench = Bench::new("ledger");

    bench.ok(&["project", "new", "bench-psu", "--title", "Bench PSU"]);
    bench.ok(&["inventory", "part", "add", "lm317", "--name", "LM317"]);
    bench.ok(&["inventory", "receive", "lm317", "--qty", "10"]);
    bench.ok(&[
        "inventory",
        "consume",
        "lm317",
        "--qty",
        "3",
        "--project",
        "bench-psu",
    ]);

    let events = bench.data().join("inventory/events");
    let year = files_in(&events).pop().expect("a year directory");
    let names = files_in(&events.join(&year));

    assert_eq!(names.len(), 2, "each movement is its own file: {names:?}");
    assert_ne!(names[0], names[1], "event names are unique");

    let stock = bench.json(&["inventory", "stock"]);
    assert_eq!(stock[0]["part"], "lm317");
    assert_eq!(stock[0]["on_hand"], 7);
    assert_eq!(stock[0]["events"], 2);

    // Nothing rewrites a recorded event: the second receive adds a file.
    bench.ok(&["inventory", "receive", "lm317", "--qty", "1"]);
    assert_eq!(files_in(&events.join(&year)).len(), 3);
    assert_eq!(bench.json(&["inventory", "stock"])[0]["on_hand"], 8);
}

#[test]
fn experiments_and_measurements_hang_off_a_project() {
    let bench = Bench::new("experiments");

    bench.ok(&["project", "new", "bench-psu"]);
    bench.ok(&[
        "experiment",
        "new",
        "bench-psu/ripple",
        "--title",
        "Ripple at 1A",
    ]);

    bench.ok(&[
        "measurement",
        "record",
        "--project",
        "bench-psu",
        "--experiment",
        "ripple",
        "--quantity",
        "ripple",
        "--value",
        "4.2",
        "--unit",
        "mVpp",
    ]);

    let view = bench.json(&["experiment", "show", "bench-psu/ripple"]);
    assert_eq!(view["status"], "planned");
    assert_eq!(view["measurements"][0]["value"], 4.2);

    bench.ok(&["experiment", "set", "bench-psu/ripple", "--status", "done"]);
    assert_eq!(
        bench.json(&["experiment", "show", "bench-psu/ripple"])["status"],
        "done"
    );

    // A reading against an experiment that does not exist is refused before
    // it can become an immutable orphan.
    let error = bench.fails(&[
        "measurement",
        "record",
        "--project",
        "bench-psu",
        "--experiment",
        "nope",
        "--quantity",
        "ripple",
        "--value",
        "1",
        "--unit",
        "mVpp",
    ]);

    assert!(error.contains("unknown experiment"), "{error}");
}

#[test]
fn references_and_slugs_are_validated_before_anything_is_written() {
    let bench = Bench::new("validation");

    assert!(
        bench
            .fails(&["project", "new", "Bench_PSU"])
            .contains("invalid project slug")
    );
    assert!(
        bench
            .fails(&["experiment", "new", "ripple"])
            .contains("<project>/<slug>")
    );
    assert!(
        bench
            .fails(&["inventory", "receive", "lm317"])
            .contains("unknown part")
    );

    bench.ok(&["project", "new", "bench-psu"]);
    assert!(
        bench
            .fails(&["project", "new", "bench-psu"])
            .contains("already exists")
    );
}

#[test]
fn check_catches_hand_edits_that_break_the_ledger() {
    let bench = Bench::new("check");

    bench.ok(&["project", "new", "bench-psu"]);
    bench.ok(&["inventory", "part", "add", "lm317"]);
    bench.ok(&["inventory", "receive", "lm317", "--qty", "2"]);

    let report = bench.json(&["repo", "check"]);
    assert_eq!(report["problems"].as_array().unwrap().len(), 0);

    // Git surgery is the supported way to move data between machines, so a
    // dangling reference is a realistic state, not an impossible one.
    std::fs::remove_file(bench.data().join("inventory/parts/lm317.toml")).unwrap();

    let output = bench.run(&["repo", "check", "--json"]);
    assert!(
        !output.status.success(),
        "check must fail on a broken ledger"
    );

    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let problems = report["problems"].as_array().unwrap();

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0]["kind"], "dangling-part");
}

#[test]
fn git_runs_in_the_workbench_and_forwards_its_exit_code() {
    let bench = Bench::new("git");

    bench.ok(&["project", "new", "bench-psu"]);

    let inside = bench.ok(&["git", "rev-parse", "--show-toplevel"]);
    assert_eq!(
        std::fs::canonicalize(inside.trim()).unwrap(),
        std::fs::canonicalize(bench.data()).unwrap()
    );

    assert!(
        bench
            .ok(&["git", "status", "--porcelain"])
            .contains("projects/")
    );

    let output = bench.run(&["git", "cat-file", "-p", "does-not-exist"]);
    assert_eq!(output.status.code(), Some(128));
}

#[test]
fn project_checkouts_stay_machine_local() {
    let bench = Bench::new("link");

    bench.ok(&["project", "new", "bench-psu"]);

    let checkout = bench.home.join("checkout");
    std::fs::create_dir_all(&checkout).unwrap();

    bench.ok(&[
        "project",
        "link",
        "bench-psu",
        "--path",
        checkout.to_str().unwrap(),
    ]);

    let view = bench.json(&["project", "show", "bench-psu"]);
    assert_eq!(
        std::fs::canonicalize(view["checkout"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&checkout).unwrap()
    );

    // The mapping is keyed by project id and lives in XDG state, never in the
    // repository: a clone on another machine carries no paths with it.
    assert!(
        bench
            .home
            .join("state/ee-workbench/checkouts.toml")
            .is_file()
    );
    assert!(
        bench
            .ok(&["git", "status", "--porcelain"])
            .lines()
            .all(|line| { !line.contains("checkouts") })
    );

    let stored =
        std::fs::read_to_string(bench.home.join("state/ee-workbench/checkouts.toml")).unwrap();
    assert!(stored.contains(view["id"].as_str().unwrap()));
}
