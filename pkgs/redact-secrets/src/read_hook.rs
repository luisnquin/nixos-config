use std::io::{self, Read};
use std::path::Path;

fn holds_secrets(path: &str) -> bool {
    let Some(name) = Path::new(path).file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    let template = [".example", ".sample", ".template", ".dist"]
        .iter()
        .any(|suffix| name.ends_with(suffix));
    if template {
        return false;
    }
    name == ".env"
        || name.starts_with(".env.")
        || name.ends_with(".env")
        || matches!(
            name.as_str(),
            ".senv" | ".netrc" | ".pgpass" | ".npmrc" | ".pypirc" | ".git-credentials" | ".secrets"
        )
        || name.starts_with("id_") && !name.ends_with(".pub")
        || [".pem", ".key", ".p12", ".pfx", ".keystore", ".jks"]
            .iter()
            .any(|ext| name.ends_with(ext))
        || name.starts_with("credentials")
        || name.starts_with("secrets.") && !name.ends_with(".nix")
}

pub fn run() -> io::Result<()> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let Ok(event) = serde_json::from_str::<serde_json::Value>(&input) else {
        return Ok(());
    };
    let Some(path) = event["tool_input"]["file_path"].as_str() else {
        return Ok(());
    };
    if !holds_secrets(path) {
        return Ok(());
    }
    let decision = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": format!(
                "{path} likely holds secrets. Use `cat` through Bash instead, whose output is redacted, or blindfold for a value you need."
            ),
        }
    });
    println!("{decision}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::holds_secrets;

    #[test]
    fn secret_files() {
        for path in [
            "/p/.env",
            ".env.local",
            "prod.env",
            "/home/u/.senv",
            "~/.ssh/id_ed25519",
            "tls.pem",
            "/x/.aws/credentials",
            "secrets.yaml",
            ".netrc",
        ] {
            assert!(holds_secrets(path), "{path}");
        }
    }

    #[test]
    fn ordinary_files() {
        for path in [
            ".env.example",
            "id_ed25519.pub",
            "secrets.nix",
            "src/env.rs",
            "keyboard.nix",
            "README.md",
            "/p/.envrc",
        ] {
            assert!(!holds_secrets(path), "{path}");
        }
    }
}
