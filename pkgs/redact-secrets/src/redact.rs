use std::io::{self, BufRead, Write};
use std::sync::LazyLock;

use regex::bytes::{Captures, Regex};

fn re(pattern: &str) -> Regex {
    Regex::new(&format!("(?-u){pattern}")).expect("static pattern")
}

static TOKENS: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    [
        (
            "private-key",
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----",
        ),
        ("gh", r"\bgithub_pat_[A-Za-z0-9_]{20,}"),
        ("gh", r"\bgh[pousr]_[A-Za-z0-9]{30,}"),
        ("gitlab", r"\bglpat-[A-Za-z0-9_-]{20,}"),
        ("anthropic", r"\bsk-ant-[A-Za-z0-9_-]{20,}"),
        ("openai", r"\bsk-[A-Za-z0-9_-]{20,}"),
        ("sentry", r"\bsntry[su]_[A-Za-z0-9+/=_-]{20,}"),
        ("aws", r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b"),
        ("slack", r"\bxox[abposr]-[A-Za-z0-9-]{10,}"),
        ("supabase", r"\bsb_secret_[A-Za-z0-9_-]{16,}"),
        ("npm", r"\bnpm_[A-Za-z0-9]{36}\b"),
        ("google", r"\bAIza[0-9A-Za-z_-]{35}"),
        ("stripe", r"\b[rs]k_live_[A-Za-z0-9]{16,}"),
        (
            "jwt",
            r"\beyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
        ),
    ]
    .into_iter()
    .map(|(label, pattern)| (label, re(pattern)))
    .collect()
});

static BEARER: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\b(bearer|basic)(\s+)[A-Za-z0-9._~+/=-]{12,}"));

static URL_PASSWORD: LazyLock<Regex> = LazyLock::new(|| re(r"(://[^/\s:@]+:)[^/\s@]+@"));

static ENV_LINE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(\s*(?:export\s+)?)([A-Za-z_][A-Za-z0-9_.-]*)(=)(.+?)(\s*)$"));

static INLINE: LazyLock<Regex> = LazyLock::new(|| {
    re(concat!(
        r#"(^|[^A-Za-z0-9_.])("?)([A-Za-z_][A-Za-z0-9_.-]*)("?)"#,
        r#"([ \t]*(?:=|:)[ \t]*)"#,
        r#"("[^"\n]*"|'[^'\n]*'|[^\s"',;&|(){}\[\]<>]+)"#,
    ))
});

static PEM_BEGIN: LazyLock<Regex> =
    LazyLock::new(|| re(r#"^\s*["']?-----BEGIN [A-Z ]*PRIVATE KEY-----["']?\s*$"#));
static PEM_END: LazyLock<Regex> = LazyLock::new(|| re(r"-----END [A-Z ]*PRIVATE KEY-----"));
static PEM_BODY: LazyLock<Regex> =
    LazyLock::new(|| re(r"^\s*(?:[A-Za-z0-9+/=]*|[A-Za-z-]+:.*)\s*$"));

const SENSITIVE: &[(&str, &str)] = &[
    ("PASSWORD", "password"),
    ("PASSWD", "password"),
    ("PASSPHRASE", "password"),
    ("TOKEN", "token"),
    ("SECRET", "secret"),
    ("CREDENTIAL", "credential"),
    ("DSN", "dsn"),
    ("APIKEY", "key"),
    ("PRIVATEKEY", "key"),
    ("ACCESSKEY", "key"),
    ("AUTH", "auth"),
];

const BENIGN: &[&str] = &[
    "TOKENS",
    "PATH",
    "FILE",
    "DIR",
    "PUBLIC",
    "PUBLISHABLE",
    "AUTHOR",
    "TYPE",
    "NAME",
    "COUNT",
    "TTL",
    "EXPIR",
    "LENGTH",
    "LIMIT",
    "URL",
    "ENDPOINT",
    "METHOD",
    "MODE",
    "PROVIDER",
    "USER",
    "HEADER",
    "CMD",
    "COMMAND",
];

const PLACEHOLDERS: &[&str] = &[
    "true", "false", "null", "none", "nil", "bearer", "basic", "token", "required", "optional",
];

fn hint(name: &[u8]) -> Option<&'static str> {
    let upper: String = String::from_utf8_lossy(name)
        .to_ascii_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if BENIGN.iter().any(|b| upper.contains(b)) {
        return None;
    }
    if let Some((_, label)) = SENSITIVE.iter().find(|(s, _)| upper.contains(s)) {
        return Some(label);
    }
    let is_key = upper == "KEY" || upper.ends_with("KEY") && !upper.ends_with("MONKEY");
    let is_pass = upper == "PASS" || upper == "PWD" || upper.ends_with("PASS");
    match (is_key, is_pass) {
        (true, _) => Some("key"),
        (_, true) => Some("password"),
        _ => None,
    }
}

fn unquote(value: &[u8]) -> (&[u8], &[u8]) {
    match value {
        [q @ (b'"' | b'\''), .., last] if last == q && value.len() >= 2 => {
            (&value[1..value.len() - 1], &value[..1])
        }
        _ => (value, b""),
    }
}

fn is_placeholder(value: &[u8]) -> bool {
    let text = String::from_utf8_lossy(value);
    value.is_empty()
        || value.iter().all(u8::is_ascii_digit)
        || matches!(
            value[0],
            b'$' | b'<' | b'%' | b'{' | b'[' | b'(' | b'*' | b'=' | b'`'
        )
        || text.contains("-redacted>")
        || PLACEHOLDERS.iter().any(|p| text.eq_ignore_ascii_case(p))
}

fn mask(value: &[u8], label: &str) -> Vec<u8> {
    let (_, quote) = unquote(value);
    [quote, format!("<{label}-redacted>").as_bytes(), quote].concat()
}

fn replace(input: &[u8], pattern: &Regex, with: &[u8]) -> Vec<u8> {
    pattern.replace_all(input, with).into_owned()
}

fn mask_tokens(input: &[u8]) -> Vec<u8> {
    let mut out = input.to_vec();
    for (label, pattern) in TOKENS.iter() {
        out = replace(&out, pattern, format!("<{label}-redacted>").as_bytes());
    }
    out = replace(&out, &BEARER, b"${1}${2}<bearer-redacted>");
    replace(&out, &URL_PASSWORD, b"${1}<password-redacted>@")
}

fn mask_env_line(caps: &Captures) -> Option<Vec<u8>> {
    let label = hint(&caps[2])?;
    if is_placeholder(unquote(&caps[4]).0) {
        return None;
    }
    Some(
        [
            &caps[1],
            &caps[2],
            &caps[3],
            &mask(&caps[4], label)[..],
            &caps[5],
        ]
        .concat(),
    )
}

fn mask_inline(caps: &Captures) -> Vec<u8> {
    let whole = caps[0].to_vec();
    let Some(label) = hint(&caps[3]) else {
        return whole;
    };
    let (inner, quote) = unquote(&caps[6]);
    let literal = inner.len() >= 8 && (!quote.is_empty() || inner.iter().any(u8::is_ascii_digit));
    if is_placeholder(inner) || !literal {
        return whole;
    }
    [
        &caps[1],
        &caps[2],
        &caps[3],
        &caps[4],
        &caps[5],
        &mask(&caps[6], label)[..],
    ]
    .concat()
}

pub fn line(input: &[u8]) -> Vec<u8> {
    let out = mask_tokens(input);
    if let Some(caps) = ENV_LINE.captures(&out) {
        return mask_env_line(&caps).unwrap_or_else(|| out.clone());
    }
    INLINE.replace_all(&out, mask_inline).into_owned()
}

#[derive(Default)]
struct Pem {
    open: bool,
}

impl Pem {
    fn swallows(&mut self, body: &[u8]) -> bool {
        if !self.open {
            return false;
        }
        if PEM_END.is_match(body) {
            self.open = false;
            return true;
        }
        self.open = PEM_BODY.is_match(body);
        self.open
    }

    fn opens(&mut self, body: &[u8]) -> bool {
        self.open = PEM_BEGIN.is_match(body);
        self.open
    }
}

pub fn stream(input: impl io::Read, mut output: impl Write) -> io::Result<()> {
    let mut reader = io::BufReader::new(input);
    let mut buf = Vec::new();
    let mut pem = Pem::default();
    loop {
        buf.clear();
        if reader.read_until(b'\n', &mut buf)? == 0 {
            return output.flush();
        }
        let (body, newline) = match buf.strip_suffix(b"\n") {
            Some(body) => (body, &b"\n"[..]),
            None => (&buf[..], &b""[..]),
        };
        if pem.swallows(body) {
            continue;
        }
        let masked = if pem.opens(body) {
            b"<private-key-redacted>".to_vec()
        } else {
            line(body)
        };
        output.write_all(&masked)?;
        output.write_all(newline)?;
        output.flush()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(s: &str) -> String {
        String::from_utf8(line(s.as_bytes())).unwrap()
    }

    fn s(s: &str) -> String {
        let mut out = Vec::new();
        stream(s.as_bytes(), &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    const GH: &str = "ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789";
    const JWT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJyb2xlIjoic2VydmljZV9yb2xlIn0.c2lnbmF0dXJlLXNpZ25hdHVyZQ";

    #[test]
    fn known_tokens_get_their_hint() {
        assert_eq!(r(&format!("using {GH} now")), "using <gh-redacted> now");
        assert_eq!(r(&format!("key {JWT}")), "key <jwt-redacted>");
        assert_eq!(
            r("sntrys_eyJpYXQiOjE3MDAwMDAwMDAuMH0abcdef"),
            "<sentry-redacted>"
        );
        assert_eq!(
            r("sk-ant-api03-abcdefghijklmnopqrstuvwxyz"),
            "<anthropic-redacted>"
        );
        assert_eq!(r("sk-proj-abcdefghijklmnopqrstuvwxyz"), "<openai-redacted>");
        assert_eq!(r("AKIAABCDEFGHIJKLMNOP"), "<aws-redacted>");
        assert_eq!(r("xoxb-123456789012-abcdefghij"), "<slack-redacted>");
    }

    #[test]
    fn env_file_keeps_names() {
        let env = format!(
            "GITHUB_TOKEN={GH}\nSUPABASE_SERVICE_ROLE_KEY=\"{JWT}\"\nexport DB_PASSWORD=hunter2\nSENTRY_AUTH_TOKEN=abc\nPORT=8080\nNODE_ENV=production\nAPI_KEY=\nTOKEN_PATH=/run/token\n"
        );
        assert_eq!(
            s(&env),
            "GITHUB_TOKEN=<gh-redacted>\nSUPABASE_SERVICE_ROLE_KEY=\"<jwt-redacted>\"\nexport DB_PASSWORD=<password-redacted>\nSENTRY_AUTH_TOKEN=<token-redacted>\nPORT=8080\nNODE_ENV=production\nAPI_KEY=\nTOKEN_PATH=/run/token\n"
        );
    }

    #[test]
    fn inline_assignments() {
        assert_eq!(
            r(r#"  "apiKey": "abcd1234efgh","#),
            r#"  "apiKey": "<key-redacted>","#
        );
        assert_eq!(
            r("curl --password=s3cr3tpass1 x"),
            "curl --password=<password-redacted> x"
        );
        assert_eq!(
            r("Authorization: Bearer abcdefghijklmnop"),
            "Authorization: Bearer <bearer-redacted>"
        );
        assert_eq!(
            r("postgres://app:p4ss@db:5432/x"),
            "postgres://app:<password-redacted>@db:5432/x"
        );
    }

    #[test]
    fn code_is_left_alone() {
        for code in [
            "let token = cfg.token;",
            "token = get_token()",
            "max_tokens: 4096",
            "passwordFile = \"/run/secrets/pw\";",
            "if [ \"$TOKEN\" = \"$EXPECTED\" ]; then",
            "SECRET=${SECRET:-}",
            "author: luis",
            "\"permissionDecision\": \"deny\"",
        ] {
            assert_eq!(r(code), code);
        }
    }

    #[test]
    fn pem_blocks_collapse() {
        let pem = "before\n-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAA\nAAAABG5vbmU=\n-----END OPENSSH PRIVATE KEY-----\nafter\n";
        assert_eq!(s(pem), "before\n<private-key-redacted>\nafter\n");
        assert_eq!(
            r(r#"{"k":"-----BEGIN PRIVATE KEY-----\nMIIE\n-----END PRIVATE KEY-----\n"}"#),
            r#"{"k":"<private-key-redacted>\n"}"#
        );
    }

    #[test]
    fn a_pem_marker_in_code_does_not_swallow_output() {
        let code = "-----BEGIN RSA PRIVATE KEY-----\nfn main() {}\n";
        assert_eq!(s(code), "<private-key-redacted>\nfn main() {}\n");
    }

    #[test]
    fn binary_passes_through() {
        let png: Vec<u8> = vec![
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0xff, 0xfe,
        ];
        let mut out = Vec::new();
        stream(&png[..], &mut out).unwrap();
        assert_eq!(out, png);
    }
}
