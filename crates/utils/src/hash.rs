use std::hash::{Hash, Hasher};

pub fn hash_fast(input: impl Hash) -> u64 {
    let mut hasher = ahash::AHasher::default();
    input.hash(&mut hasher);
    hasher.finish()
}

pub fn hash_portable(input: impl AsRef<[u8]>) -> u64 {
    seahash::hash(input.as_ref())
}
