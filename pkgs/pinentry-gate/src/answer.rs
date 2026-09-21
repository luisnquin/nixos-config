use std::io::Read;

use crate::{broker, fifo};

fn is_request_id(id: &str) -> bool {
    (16..=32).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn main(args: &[String]) -> i32 {
    let [id] = args else {
        eprintln!("usage: pinentry-gate answer ID");
        return 2;
    };
    if !is_request_id(id) {
        eprintln!("pinentry-gate: not a request id");
        return 2;
    }
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return 2;
    }
    let first = input.split('\n').next().unwrap_or_default().trim_end_matches('\r');
    if fifo::deliver(&broker::runtime_dir().join(id), &format!("{first}\n")) {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_lowercase_hex_of_the_right_length_is_an_id() {
        assert!(is_request_id("0123456789abcdef"));
        assert!(is_request_id("ae7fd1b08f6b0c89"));
        assert!(!is_request_id("0123456789ABCDEF"));
        assert!(!is_request_id("0123456789abcde"));
        assert!(!is_request_id("../../../etc/passwd"));
        assert!(!is_request_id(&"a".repeat(33)));
        assert!(!is_request_id(""));
    }
}
