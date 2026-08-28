//! The one declarative file: `phone.toml`, beside the code it drives.
//!
//! There is no machine-wide manifest on purpose. A device only means something
//! next to the project pointed at it — `pixel_7-api36` is a Cuenta Cero
//! emulator this week and something else the next — so what is declared is
//! declared in the repository, travels with it, and reviews with it.
//!
//! Nothing here knows what Expo is. `stale` is a command whose output is
//! hashed and `run` is a command; which build system produces them is the
//! manifest's business, not this program's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

pub const FILE: &str = "phone.toml";

/// How far up the ladder a device is meant to be. Ordered, and every level
/// implies the ones before it — which is the whole reason the enum derives
/// `Ord` rather than carrying a `satisfies()` of its own.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Running on its host.
    Booted,
    /// Running, and reachable from here over a transport.
    Attached,
    /// Attached, with its forwards open and its settings as declared.
    Ready,
    /// Ready, carrying a build no older than the sources, and in the app.
    #[default]
    Prepared,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Booted => "booted",
            Level::Attached => "attached",
            Level::Ready => "ready",
            Level::Prepared => "prepared",
        }
    }
}

/// Something to run on the host, and the question of whether it needs running.
///
/// `stale` is run first and its stdout hashed; the hash is remembered, and
/// `run` happens when it moves. A task with no `stale` has no way to be fresh
/// and runs every time, which is almost never what is wanted — so it is worth
/// saying out loud rather than defaulting to it silently.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub stale: Option<String>,
    pub run: String,
}

/// A `Task` that also produces something installable, so freshness has a second
/// question to answer: the artifact may be current and the device still not
/// have it.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// Package name on Android, bundle id on iOS.
    pub app: String,
    pub stale: Option<String>,
    pub run: String,
    /// The app's own `argv`, for a runtime that reads where to connect from it
    /// rather than from a url. Only a simulator can carry them: `am start` sends
    /// an intent rather than spawning a process, so an Android build declaring
    /// any is refused here rather than having them quietly dropped.
    #[serde(default)]
    pub args: Vec<String>,
    /// A url to send the app once it is up, for an app that needs telling more
    /// than that it should start. A dev client launched by its icon shows its
    /// own menu and waits to be told which bundler to use; the deep link is
    /// what answers that, and it is sent on every run rather than only after a
    /// build, because the answer does not survive the app being closed.
    pub open: Option<String>,
}

impl Build {
    pub fn task(&self) -> Task {
        Task {
            stale: self.stale.clone(),
            run: self.run.clone(),
        }
    }
}

fn metro_port() -> u16 {
    8081
}

/// The dev server the app talks to. Held apart from `build` because it outlives
/// any one build: a bundler is started once and answers every later `up`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundler {
    pub run: String,
    #[serde(default = "metro_port")]
    pub port: u16,
}

/// A value as `settings put` will spell it. Kept to the three TOML scalars a
/// device setting is ever written as, so a typo'd table or array is a parse
/// error here rather than a string like `[1, 2]` landing on the device.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Scalar {
    Int(i64),
    Bool(bool),
    Str(String),
}

impl std::fmt::Display for Scalar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scalar::Int(v) => write!(f, "{v}"),
            // `settings` stores everything as text and Android reads these back
            // as ints; `true` would be read as 0
            Scalar::Bool(v) => write!(f, "{}", u8::from(*v)),
            Scalar::Str(v) => f.write_str(v),
        }
    }
}

/// The three namespaces `settings` divides a device's own configuration into.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default)]
    pub global: BTreeMap<String, Scalar>,
    #[serde(default)]
    pub system: BTreeMap<String, Scalar>,
    #[serde(default)]
    pub secure: BTreeMap<String, Scalar>,
}

impl Settings {
    /// Every declared setting as `(namespace, key, value)`, in one list because
    /// every caller wants them together and none wants them by namespace.
    pub fn each(&self) -> Vec<(&'static str, &str, String)> {
        let mut out = Vec::new();

        for (ns, map) in [
            ("global", &self.global),
            ("system", &self.system),
            ("secure", &self.secure),
        ] {
            for (key, value) in map {
                out.push((ns, key.as_str(), value.to_string()));
            }
        }

        out
    }

    pub fn is_empty(&self) -> bool {
        self.global.is_empty() && self.system.is_empty() && self.secure.is_empty()
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    #[serde(default)]
    pub state: Level,
    /// Ports on the device that should reach the same port on the device's own
    /// host — which is where the bundler runs.
    #[serde(default)]
    pub reverse: Vec<u16>,
    #[serde(default)]
    pub settings: Settings,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub devices: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// The machine that holds the tree, the toolchain and the devices. Absent
    /// means this one.
    pub host: Option<String>,
    /// Where the tree sits on `host`. Every command runs with this as its cwd.
    pub dir: Option<String>,
    /// The device unqualified commands target inside this project.
    pub default: Option<String>,

    pub deps: Option<Task>,
    pub bundler: Option<Bundler>,

    /// Keyed by platform: `android`, `ios`.
    #[serde(default)]
    pub build: BTreeMap<String, Build>,
    #[serde(default)]
    pub devices: BTreeMap<String, Spec>,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Clone, Debug)]
pub struct Project {
    /// The directory holding `phone.toml` on this machine.
    pub root: PathBuf,
    pub manifest: Manifest,
}

impl Project {
    /// The nearest `phone.toml` at or above `from`. `None` rather than an error:
    /// most verbs work perfectly well outside a project and must not start
    /// demanding one.
    pub fn find(from: &Path) -> Result<Option<Project>> {
        for dir in from.ancestors() {
            let path = dir.join(FILE);

            if path.is_file() {
                return Ok(Some(Self::load(&path)?));
            }
        }

        Ok(None)
    }

    pub fn here() -> Result<Option<Project>> {
        let cwd = std::env::current_dir().context("reading the working directory")?;

        Self::find(&cwd)
    }

    pub fn load(path: &Path) -> Result<Project> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;

        let manifest = Self::parse(&text).with_context(|| format!("in {}", path.display()))?;

        Ok(Project {
            root: path.parent().unwrap_or(Path::new(".")).to_path_buf(),
            manifest,
        })
    }

    pub fn parse(text: &str) -> Result<Manifest> {
        let manifest: Manifest = toml::from_str(text)?;

        manifest.check()?;

        Ok(manifest)
    }

    pub fn host(&self) -> Option<&str> {
        self.manifest.host.as_deref().filter(|h| !h.is_empty())
    }

    /// The cwd every declared command runs in. A remote project says where it
    /// lives; a local one is simply where its manifest is.
    pub fn dir(&self) -> String {
        match &self.manifest.dir {
            Some(dir) => dir.clone(),
            None => self.root.display().to_string(),
        }
    }

    /// What the remembered hashes are filed under. The local path, because that
    /// is the one thing that is the same across `up`, `status` and `down` and
    /// different between two checkouts of the same repository.
    pub fn key(&self) -> String {
        self.root.display().to_string()
    }

    pub fn name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.key())
    }

    /// The devices a run covers, in declaration order.
    ///
    /// A profile is a named subset, and naming one that does not exist is an
    /// error rather than an empty run: an agent that mistypes `--profile` would
    /// otherwise be told everything converged.
    /// A run with no profile named covers everything, which is what keeps `up`
    /// and `down` the inverse of each other: neither has a subset of its own to
    /// fall back to, so what one brings up the other takes down.
    pub fn devices(&self, profile: Option<&str>) -> Result<Vec<(&str, &Spec)>> {
        self.named(profile)
    }

    /// Every device the manifest declares, whatever profile was asked for.
    ///
    /// What `down` means, and the one place a profile is deliberately
    /// ignored: it exists so that a run does not build for a simulator nobody
    /// asked about, and there is no matching cost in shutting one down. A
    /// teardown that left it running would be the stray the next `status`
    /// reports.
    pub fn every_device(&self) -> Vec<(&str, &Spec)> {
        self.manifest
            .devices
            .iter()
            .map(|(name, spec)| (name.as_str(), spec))
            .collect()
    }

    fn named(&self, profile: Option<&str>) -> Result<Vec<(&str, &Spec)>> {
        let Some(profile) = profile else {
            return Ok(self
                .manifest
                .devices
                .iter()
                .map(|(name, spec)| (name.as_str(), spec))
                .collect());
        };

        let Some(chosen) = self.manifest.profiles.get(profile) else {
            let known: Vec<&str> = self.manifest.profiles.keys().map(String::as_str).collect();

            bail!(
                "no profile '{profile}' in {FILE}{}",
                match known.is_empty() {
                    true => String::new(),
                    false => format!(" (it has {})", known.join(", ")),
                }
            );
        };

        let mut out = Vec::new();

        for name in &chosen.devices {
            let spec = self.manifest.devices.get(name).ok_or_else(|| {
                anyhow::anyhow!("profile '{profile}' names {name}, which no [devices] entry covers")
            })?;

            out.push((name.as_str(), spec));
        }

        Ok(out)
    }
}

impl Manifest {
    /// What serde cannot say: a manifest can be well-formed TOML and still
    /// describe something that cannot be run.
    fn check(&self) -> Result<()> {
        if self.host.is_some() && self.dir.is_none() {
            bail!("a project on a remote host needs `dir`, the path to the tree there");
        }

        for platform in self.build.keys() {
            if !matches!(platform.as_str(), "android" | "ios") {
                bail!("[build.{platform}]: builds are keyed by platform, `android` or `ios`");
            }
        }

        for (platform, build) in &self.build {
            if let Some(open) = &build.open {
                crate::apps::url(open).with_context(|| format!("[build.{platform}]: `open`"))?;
            }

            if !build.args.is_empty() && platform != "ios" {
                bail!(
                    "[build.{platform}]: `args` reach an app as its argv, which only a simulator hands one; say it in `open` instead"
                );
            }
        }

        for (name, profile) in &self.profiles {
            if profile.devices.is_empty() {
                bail!("profile '{name}' names no devices");
            }
        }

        if let Some(default) = &self.default {
            if !self.devices.contains_key(default) {
                bail!("default is {default}, which no [devices] entry covers");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEVASTOPOL: &str = r#"
host = "rose"
dir = "~/Projects/github.com/cuentacero/sevastopol"
default = "pixel_7-api36"

[deps]
stale = "cksum package-lock.json"
run = "npm install"

[bundler]
run = "npx expo start --dev-client"

[build.android]
app = "app.cuentacero.dev"
stale = "npx @expo/fingerprint fingerprint:generate --platform android"
run = "npx expo run:android --no-bundler"

[devices."pixel_7-api36"]
state = "prepared"
reverse = [8081]
settings.global = { window_animation_scale = 1, transition_animation_scale = 1 }

[devices."iPhone 17 Pro Max"]
state = "ready"

[profiles.dev]
devices = ["pixel_7-api36"]
"#;

    fn manifest() -> Manifest {
        Project::parse(SEVASTOPOL).expect("the reference manifest parses")
    }

    fn project() -> Project {
        Project {
            root: PathBuf::from("/tmp/sevastopol"),
            manifest: manifest(),
        }
    }

    #[test]
    fn the_reference_manifest_reads_back_what_it_says() {
        let m = manifest();

        assert_eq!(m.host.as_deref(), Some("rose"));
        assert_eq!(m.default.as_deref(), Some("pixel_7-api36"));
        assert_eq!(m.build["android"].app, "app.cuentacero.dev");
        assert_eq!(m.devices["pixel_7-api36"].reverse, [8081]);
        assert_eq!(m.devices["pixel_7-api36"].state, Level::Prepared);
        assert_eq!(m.devices["iPhone 17 Pro Max"].state, Level::Ready);
    }

    /// The ladder is an ordering, not a set of flags: `prepared` is what says a
    /// device must also be booted, attached and ready.
    #[test]
    fn a_level_implies_every_level_below_it() {
        assert!(Level::Booted < Level::Attached);
        assert!(Level::Attached < Level::Ready);
        assert!(Level::Ready < Level::Prepared);
        assert_eq!(Level::default(), Level::Prepared);
    }

    /// A device with no `state` is one someone bothered to declare, so the
    /// useful reading of the omission is "all the way up", not "barely on".
    #[test]
    fn a_device_that_says_nothing_is_taken_all_the_way_up() {
        let m = Project::parse("[devices.pixel]\n").unwrap();

        assert_eq!(m.devices["pixel"].state, Level::Prepared);
        assert!(m.devices["pixel"].reverse.is_empty());
        assert!(m.devices["pixel"].settings.is_empty());
    }

    #[test]
    fn settings_come_out_flat_and_namespaced() {
        let m = manifest();
        let each = m.devices["pixel_7-api36"].settings.each();

        assert_eq!(
            each,
            [
                ("global", "transition_animation_scale", "1".to_string()),
                ("global", "window_animation_scale", "1".to_string()),
            ]
        );
    }

    /// `settings put` writes text and Android reads these back as integers, so
    /// a bool has to arrive as 1 or 0 rather than as the word.
    #[test]
    fn a_boolean_setting_is_written_as_a_number() {
        let m = Project::parse(
            "[devices.pixel]\nsettings.global = { stay_on_while_plugged_in = true }\n",
        )
        .unwrap();

        assert_eq!(
            m.devices["pixel"].settings.each(),
            [("global", "stay_on_while_plugged_in", "1".to_string())]
        );
    }

    #[test]
    fn a_profile_is_a_named_subset_in_the_order_it_names_them() {
        let p = project();

        assert_eq!(
            p.devices(Some("dev")).unwrap().len(),
            1,
            "the dev profile names one of the two devices"
        );
        assert_eq!(p.devices(None).unwrap().len(), 2);
    }

    /// A build without `open` is the ordinary case and must stay legal: not
    /// every app needs telling where to connect.
    #[test]
    fn a_build_need_not_declare_a_url_to_open() {
        assert_eq!(manifest().build["android"].open, None);
    }

    /// Appended to the build table rather than to the text, because the last
    /// table in the fixture is a profile and a key after it belongs to that.
    fn opening(url: &str) -> String {
        SEVASTOPOL.replace(
            "run = \"npx expo run:android --no-bundler\"",
            &format!("run = \"npx expo run:android --no-bundler\"\nopen = \"{url}\""),
        )
    }

    #[test]
    fn a_declared_url_to_open_is_kept_as_written() {
        let url = "exp+cc://expo-development-client/?url=http%3A%2F%2Flocalhost%3A8081";

        assert_eq!(
            Project::parse(&opening(url)).unwrap().build["android"]
                .open
                .as_deref(),
            Some(url)
        );
    }

    /// The url is only ever sent to a device, so a typo would otherwise surface
    /// as a launch that quietly did nothing at the end of a long build.
    #[test]
    fn a_url_to_open_with_no_scheme_is_refused_at_parse_time() {
        let err = Project::parse(&opening("expo-development-client")).unwrap_err();
        let err = format!("{err:#}");

        assert!(err.contains("build.android"), "{err}");
        assert!(err.contains("no scheme"), "{err}");
    }

    /// `am start` sends an intent and never spawns a process, so arguments
    /// declared for Android would be dropped without a word — and the app would
    /// sit on its menu with the manifest looking like it said otherwise.
    #[test]
    fn launch_arguments_on_an_android_build_are_refused() {
        let text = SEVASTOPOL.replace(
            "[build.android]",
            "[build.android]\nargs = [\"--initialUrl\", \"http://localhost:8081\"]",
        );
        let err = format!("{:#}", Project::parse(&text).unwrap_err());

        assert!(err.contains("build.android"), "{err}");
        assert!(err.contains("argv"), "{err}");
    }

    #[test]
    fn launch_arguments_on_an_ios_build_are_kept_in_order() {
        let text = format!(
            "{SEVASTOPOL}\n[build.ios]\napp = \"app.cuentacero.dev\"\nrun = \"true\"\nargs = [\"--initialUrl\", \"http://localhost:8081\"]\n"
        );

        assert_eq!(
            Project::parse(&text).unwrap().build["ios"].args,
            ["--initialUrl", "http://localhost:8081"]
        );
    }

    #[test]
    fn a_build_declaring_no_arguments_has_none_rather_than_failing_to_parse() {
        assert!(manifest().build["android"].args.is_empty());
    }

    /// The pair of bugs this closes: a bare `up` left the simulator down and a
    /// bare `down` left it running, because a manifest could name a subset that
    /// only one of the two honoured.
    #[test]
    fn a_bare_run_and_a_teardown_cover_the_same_devices() {
        let project = project();

        let up: Vec<&str> = project
            .devices(None)
            .unwrap()
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        let down: Vec<&str> = project.every_device().into_iter().map(|(n, _)| n).collect();

        assert_eq!(up, ["iPhone 17 Pro Max", "pixel_7-api36"]);
        assert_eq!(up, down);
    }

    /// A mistyped profile that ran against nothing would report success, which
    /// is the one answer that must never come back from a converge.
    #[test]
    fn a_profile_that_does_not_exist_is_refused_with_the_ones_that_do() {
        let err = project().devices(Some("e2e")).unwrap_err().to_string();

        assert!(err.contains("no profile 'e2e'"), "{err}");
        assert!(err.contains("dev"), "{err}");
    }

    #[test]
    fn a_profile_naming_an_undeclared_device_is_refused() {
        let text = "[devices.pixel]\n\n[profiles.dev]\ndevices = [\"ghost\"]\n";
        let p = Project {
            root: PathBuf::from("/tmp/x"),
            manifest: Project::parse(text).unwrap(),
        };

        let err = p.devices(Some("dev")).unwrap_err().to_string();

        assert!(err.contains("ghost"), "{err}");
    }

    #[test]
    fn a_remote_project_has_to_say_where_its_tree_is() {
        let err = Project::parse("host = \"rose\"\n").unwrap_err().to_string();

        assert!(err.contains("dir"), "{err}");
    }

    #[test]
    fn a_local_project_runs_where_its_manifest_is() {
        let p = Project {
            root: PathBuf::from("/tmp/sevastopol"),
            manifest: Project::parse("[devices.pixel]\n").unwrap(),
        };

        assert_eq!(p.host(), None);
        assert_eq!(p.dir(), "/tmp/sevastopol");
    }

    #[test]
    fn a_build_is_keyed_by_a_platform_that_exists() {
        let err = Project::parse("[build.web]\napp = \"x\"\nrun = \"y\"\n")
            .unwrap_err()
            .to_string();

        assert!(err.contains("android"), "{err}");
    }

    #[test]
    fn a_default_naming_no_declared_device_is_refused() {
        let err = Project::parse("default = \"ghost\"\n[devices.pixel]\n")
            .unwrap_err()
            .to_string();

        assert!(err.contains("ghost"), "{err}");
    }

    /// A key nobody reads is a key someone believed in. The whole manifest is
    /// `deny_unknown_fields` so a typo fails loudly instead of being ignored.
    #[test]
    fn a_misspelled_key_is_refused_rather_than_ignored() {
        assert!(Project::parse("[devices.pixel]\nreverse_ports = [8081]\n").is_err());
        assert!(Project::parse("hosts = \"rose\"\n").is_err());
    }

    #[test]
    fn the_bundler_port_has_the_usual_default() {
        let m = Project::parse("[bundler]\nrun = \"npx expo start\"\n").unwrap();

        assert_eq!(m.bundler.unwrap().port, 8081);
    }

    #[test]
    fn the_manifest_is_found_by_walking_up_from_a_subdirectory() {
        let dir = std::env::temp_dir().join(format!("phone-project-{}", std::process::id()));
        let deep = dir.join("src/app/routes");

        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(dir.join(FILE), "[devices.pixel]\n").unwrap();

        let found = Project::find(&deep).unwrap().expect("walks up to the root");

        assert_eq!(found.root, dir);
        assert!(found.manifest.devices.contains_key("pixel"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_manifest_anywhere_is_not_an_error() {
        assert!(Project::find(Path::new("/")).unwrap().is_none());
    }
}
