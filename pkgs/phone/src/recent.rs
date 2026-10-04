use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct Recent<K, V> {
    life: Duration,
    seen: Mutex<Vec<(K, Instant, V)>>,
}

impl<K: PartialEq, V: Clone> Recent<K, V> {
    pub const fn new(life: Duration) -> Self {
        Self {
            life,
            seen: Mutex::new(Vec::new()),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let seen = self.seen.lock().ok()?;

        seen.iter()
            .find(|(k, at, _)| k == key && at.elapsed() < self.life)
            .map(|(_, _, v)| v.clone())
    }

    pub fn put(&self, key: K, value: V) {
        if let Ok(mut seen) = self.seen.lock() {
            seen.retain(|(k, _, _)| k != &key);
            seen.push((key, Instant::now(), value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_latest_answer_until_it_ages_out() {
        let recent: Recent<&str, u32> = Recent::new(Duration::from_secs(60));

        recent.put("a", 1);
        recent.put("a", 2);

        assert_eq!(recent.get(&"a"), Some(2));
        assert_eq!(recent.get(&"b"), None);

        let stale: Recent<&str, u32> = Recent::new(Duration::ZERO);

        stale.put("a", 1);

        assert_eq!(stale.get(&"a"), None);
    }
}
