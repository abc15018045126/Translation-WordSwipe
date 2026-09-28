use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use crate::translator::types::TranslationResult;

pub struct SimpleCache {
    max_size: usize,
    ttl: Duration,
    entries: Mutex<HashMap<String, (TranslationResult, Instant)>>,
    lru_keys: Mutex<VecDeque<String>>,
}

impl SimpleCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            max_size,
            ttl,
            entries: Mutex::new(HashMap::new()),
            lru_keys: Mutex::new(VecDeque::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<TranslationResult> {
        let mut entries = self.entries.lock().ok()?;
        if let Some((result, expire_at)) = entries.get(key) {
            if Instant::now() < *expire_at {
                let res = result.clone();
                // Update LRU
                if let Ok(mut lru) = self.lru_keys.lock() {
                    if let Some(pos) = lru.iter().position(|k| k == key) {
                        lru.remove(pos);
                        lru.push_back(key.to_string());
                    }
                }
                return Some(res);
            } else {
                entries.remove(key);
            }
        }
        None
    }

    pub fn set(&self, key: String, value: TranslationResult) {
        let mut entries = match self.entries.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let mut lru = match self.lru_keys.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        if entries.contains_key(&key) {
            if let Some(pos) = lru.iter().position(|k| k == &key) {
                lru.remove(pos);
            }
        } else if entries.len() >= self.max_size {
            if let Some(oldest) = lru.pop_front() {
                entries.remove(&oldest);
            }
        }

        let expire_at = Instant::now() + self.ttl;
        entries.insert(key.clone(), (value, expire_at));
        lru.push_back(key);
    }
}
