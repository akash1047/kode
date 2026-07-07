use std::hash::Hasher;

/// Deterministic 64-bit hasher using the FNV-1a algorithm.
///
/// Unlike [`std::collections::hash_map::DefaultHasher`], this hasher is
/// guaranteed to produce identical outputs across Rust versions, platforms,
/// and process invocations, making it suitable for persistent identities.
pub struct Fnv1aHasher(u64);

impl Fnv1aHasher {
    pub fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
}

impl Default for Fnv1aHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for Fnv1aHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= byte as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}
