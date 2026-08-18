//! Entry model. cliplenz is a generic dmenu: an entry is just a raw input line,
//! preserved verbatim so it can be printed back on stdout unchanged.

#[derive(Debug, Clone)]
pub struct Entry {
    pub raw: String,
}

impl Entry {
    pub fn from_stdin(input: &str) -> Vec<Entry> {
        input
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| Entry { raw: l.to_string() })
            .collect()
    }

    /// What the list shows. Tabs (e.g. cliphizt's `id<tab>preview`) are widened
    /// for legibility; the raw line is what gets emitted on selection.
    pub fn display(&self) -> String {
        self.raw.replace('\t', "    ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lines_and_drops_empties() {
        let entries = Entry::from_stdin("a\n\nb\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].raw, "a");
        assert_eq!(entries[1].raw, "b");
    }

    #[test]
    fn display_widens_tabs_but_raw_is_verbatim() {
        let e = Entry { raw: "3788\t~/path".to_string() };
        assert_eq!(e.display(), "3788    ~/path");
        assert_eq!(e.raw, "3788\t~/path");
    }
}
