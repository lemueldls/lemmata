//! Derived-value memoization.
//!
//! Picante does not build for wasm32-unknown-unknown because its tokio
//! dependency enables `fs` and `rt-multi-thread` unconditionally, so the browser
//! build uses this table instead. Keys are content hashes of the inputs a query
//! reads, which keeps invalidation narrow: a query that reads one parameter
//! misses only when that parameter changes.

use std::collections::HashMap;
use std::hash::Hash;

pub mod assembly;
pub mod dotless;
pub mod font;
pub mod math;
pub mod texture;
pub mod variants;

/// Hit and miss counters for one table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub hits: u64,
    pub misses: u64,
}

/// A memo table from content-hash keys to computed values.
#[derive(Debug)]
pub struct Memo<K, V> {
    entries: HashMap<K, V>,
    stats: Stats,
}

impl<K: Eq + Hash, V> Default for Memo<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            stats: Stats::default(),
        }
    }
}

impl<K: Eq + Hash, V> Memo<K, V> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the value for `key`, computing it with `compute` on a miss.
    ///
    /// The `bool` is true when the value came from `compute`.
    pub fn get_or_insert_with(&mut self, key: K, compute: impl FnOnce() -> V) -> (&V, bool) {
        if let Some(value) = self.entries.get(&key) {
            self.stats.hits += 1;
            return (value, false);
        }
        self.stats.misses += 1;
        let value = self.entries.entry(key).or_insert_with(compute);

        (value, true)
    }

    #[must_use]
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.entries.insert(key, value)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub const fn stats(&self) -> Stats {
        self.stats
    }
}

/// FNV-1a, 64 bit.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Incremental content hash for structured inputs.
///
/// Floats are hashed by bit pattern, so a value has to change numerically to
/// change the key.
#[derive(Clone, Copy, Debug)]
pub struct ContentHasher {
    state: u64,
}

impl Default for ContentHasher {
    fn default() -> Self {
        Self { state: FNV_OFFSET }
    }
}

impl ContentHasher {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn mix(&mut self, bytes: u64) {
        let mut value = self.state;
        for byte in bytes.to_le_bytes() {
            value ^= u64::from(byte);
            value = value.wrapping_mul(FNV_PRIME);
        }
        self.state = value;
    }

    pub fn write_u64(&mut self, value: u64) {
        self.mix(value);
    }

    pub fn write_i64(&mut self, value: i64) {
        self.mix(value.cast_unsigned());
    }

    pub fn write_f64(&mut self, value: f64) {
        self.mix(value.to_bits());
    }

    pub fn write_str(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.mix(bytes.len() as u64);
        for byte in bytes {
            self.mix(u64::from(*byte));
        }
    }

    #[must_use]
    pub const fn finish(&self) -> u64 {
        self.state
    }
}

/// Content hash of a byte slice.
#[must_use]
pub fn content_hash(bytes: &[u8]) -> u64 {
    let mut hasher = ContentHasher::new();
    hasher.write_bytes(bytes);

    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::{ContentHasher, Memo, content_hash};

    #[test]
    fn same_key_computes_once() {
        let mut memo: Memo<u64, i32> = Memo::new();
        let (first, computed) = memo.get_or_insert_with(7, || 42);
        assert_eq!(*first, 42);
        assert!(computed);
        let (second, computed) = memo.get_or_insert_with(7, || panic!("should not run"));
        assert_eq!(*second, 42);
        assert!(!computed);
        assert_eq!(memo.stats().misses, 1);
        assert_eq!(memo.stats().hits, 1);
    }

    #[test]
    fn different_keys_compute_separately() {
        let mut memo: Memo<u64, i32> = Memo::new();
        memo.get_or_insert_with(1, || 10);
        memo.get_or_insert_with(2, || 20);
        assert_eq!(memo.len(), 2);
        assert_eq!(memo.stats().misses, 2);
        assert_eq!(memo.stats().hits, 0);
    }

    #[test]
    fn hashing_is_stable_and_content_sensitive() {
        let mut a = ContentHasher::new();
        a.write_str("stem");
        a.write_f64(84.0);
        let mut b = ContentHasher::new();
        b.write_str("stem");
        b.write_f64(84.0);
        let mut c = ContentHasher::new();
        c.write_str("stem");
        c.write_f64(85.0);

        assert_eq!(a.finish(), b.finish());
        assert_ne!(a.finish(), c.finish());
    }

    #[test]
    fn content_hash_matches_hasher() {
        let mut hasher = ContentHasher::new();
        hasher.write_str("abc");
        assert_eq!(hasher.finish(), content_hash(b"abc"));
    }
}
