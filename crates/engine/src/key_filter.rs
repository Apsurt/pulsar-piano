//! Which keyboard notes may sound. While practicing with "wrong keys silent", the app keeps this set
//! to the keys it expects next; any other key press is still reported to the UI but never played.
//! Lock-free, because the MIDI callback reads it on every note.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Default)]
pub struct KeyFilter {
    on: AtomicBool,
    /// One bit per MIDI key, 0–127.
    keys: [AtomicU64; 2],
}

impl KeyFilter {
    /// `Some(keys)` lets only those keys sound; `None` lets every key sound.
    pub fn set(&self, keys: Option<&[u8]>) {
        let mut bits = [0u64; 2];
        for &key in keys.unwrap_or_default() {
            if key < 128 {
                bits[usize::from(key / 64)] |= 1 << (key % 64);
            }
        }
        self.keys[0].store(bits[0], Ordering::Relaxed);
        self.keys[1].store(bits[1], Ordering::Relaxed);
        self.on.store(keys.is_some(), Ordering::Release);
    }

    pub fn allows(&self, key: u8) -> bool {
        if !self.on.load(Ordering::Acquire) || key >= 128 {
            return true;
        }
        self.keys[usize::from(key / 64)].load(Ordering::Relaxed) & (1 << (key % 64)) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lets_everything_through_until_set() {
        let filter = KeyFilter::default();
        assert!(filter.allows(60));
        filter.set(Some(&[60, 64, 100]));
        assert!(filter.allows(60) && filter.allows(64) && filter.allows(100));
        assert!(!filter.allows(61) && !filter.allows(0) && !filter.allows(127));
        filter.set(Some(&[]));
        assert!(!filter.allows(60));
        filter.set(None);
        assert!(filter.allows(61));
    }
}
