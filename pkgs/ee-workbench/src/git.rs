use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

fn git(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(root);
    command
}

pub fn is_repo(root: &Path) -> bool {
    root.join(".git").exists()
}

pub fn init_repo(root: &Path) -> Result<()> {
    let status = git(root)
        .args(["init", "--quiet"])
        .status()
        .context("running `git init`: is git on PATH?")?;

    if !status.success() {
        bail!("`git init` failed in {}", root.display());
    }

    Ok(())
}

/// Transparent passthrough, like `pass git`: the operator's own git, run in
/// the workbench repository, with no arguments added and nothing implied.
pub fn passthrough(root: &Path, args: &[String]) -> Result<i32> {
    if !is_repo(root) {
        bail!(
            "{} is not a Git repository: run `ee repo init`",
            root.display()
        );
    }

    let status = git(root)
        .args(args)
        .status()
        .context("running git: is git on PATH?")?;

    Ok(match status.code() {
        Some(code) => code,
        None => 128 + status.signal().unwrap_or(0),
    })
}

/// `git status --porcelain` lines, repository-relative. `None` means the data
/// root is not a Git repository yet.
pub fn porcelain(root: &Path) -> Result<Option<Vec<String>>> {
    if !is_repo(root) {
        return Ok(None);
    }

    let output = git(root)
        .args(["status", "--porcelain"])
        .output()
        .context("running `git status`: is git on PATH?")?;

    if !output.status.success() {
        bail!(
            "`git status` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
    ))
}

pub fn head_branch(root: &Path) -> Result<Option<String>> {
    if !is_repo(root) {
        return Ok(None);
    }

    let output = git(root)
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .output()
        .context("running `git symbolic-ref`: is git on PATH?")?;

    if !output.status.success() {
        return Ok(None);
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok((!branch.is_empty()).then_some(branch))
}
