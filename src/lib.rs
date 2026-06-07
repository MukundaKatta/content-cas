//! # content-cas
//!
//! Content-addressed cache. Store bytes under their SHA-256 hex, retrieve
//! by hex. On-disk layout is `root/aa/bbbb...` (first 2 hex chars become
//! a subdirectory to keep filesystem ls fast even at millions of keys).
//!
//! Reasonable for embeddings, model responses, tokenizer outputs — any
//! large, immutable, deterministic blob whose key is its content.
//!
//! ## Example
//!
//! ```no_run
//! use content_cas::Cas;
//! let cas = Cas::new("/tmp/my-cas").unwrap();
//! let hash = cas.put(b"hello world").unwrap();
//! assert_eq!(hash.len(), 64);
//! let bytes = cas.get(&hash).unwrap().unwrap();
//! assert_eq!(bytes, b"hello world");
//! ```

#![deny(missing_docs)]

mod sha256;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic counter making each `put` temp file name unique within a process,
/// so concurrent writers never stage bytes through the same temp path.
static WRITE_SEQ: AtomicU64 = AtomicU64::new(0);

/// On-disk content-addressed cache.
#[derive(Debug, Clone)]
pub struct Cas {
    root: PathBuf,
}

impl Cas {
    /// Create or open a CAS rooted at `root`.
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Compute the SHA-256 of `bytes` and store. Returns the 64-char hex
    /// key. Re-writing the same key is a no-op.
    pub fn put(&self, bytes: &[u8]) -> io::Result<String> {
        let hash = sha256::hex(bytes);
        let p = self.path_for(&hash);
        if !p.exists() {
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent)?;
            }
            // Atomic write: write to a unique temp file, then rename over the
            // final path. The temp name embeds the pid and a per-call counter
            // so concurrent writers of the same content never collide on the
            // same temp file (which would otherwise corrupt the staged bytes).
            let seq = WRITE_SEQ.fetch_add(1, Ordering::Relaxed);
            let tmp = p.with_extension(format!("tmp.{}.{}", std::process::id(), seq));
            fs::write(&tmp, bytes)?;
            // rename is atomic on the same filesystem; since the content is
            // immutable, racing renames simply settle on identical bytes.
            if let Err(e) = fs::rename(&tmp, &p) {
                // Clean up our temp file on failure so we don't leak it.
                let _ = fs::remove_file(&tmp);
                return Err(e);
            }
        }
        Ok(hash)
    }

    /// Retrieve the bytes for a hex key. Returns `Ok(None)` if absent.
    pub fn get(&self, hash: &str) -> io::Result<Option<Vec<u8>>> {
        let p = self.path_for(hash);
        match fs::read(&p) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// True when `hash` is present.
    pub fn contains(&self, hash: &str) -> bool {
        self.path_for(hash).exists()
    }

    /// Delete the entry for `hash`. Returns `Ok(false)` if absent.
    pub fn remove(&self, hash: &str) -> io::Result<bool> {
        let p = self.path_for(hash);
        match fs::remove_file(&p) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Compute the path where `hash` is or would be stored.
    ///
    /// Keys whose first shard (the leading two characters) can be split off on
    /// a UTF-8 boundary are sharded as `root/aa/bbb...`. Shorter keys — which a
    /// real SHA-256 hex digest never is, but which a caller may still pass to
    /// [`get`](Self::get), [`contains`](Self::contains), or
    /// [`remove`](Self::remove) — are stored under a reserved `_short/`
    /// subdirectory. This keeps such keys from colliding with the cache root or
    /// a two-character shard directory, and means lookups on short keys behave
    /// as ordinary misses instead of panicking on the prefix split.
    pub fn path_for(&self, hash: &str) -> PathBuf {
        // `split_at` panics on out-of-range or non-char-boundary indices, so we
        // only shard when offset 2 is a valid char boundary (which implies the
        // key has at least two characters).
        if hash.len() >= 2 && hash.is_char_boundary(2) {
            let (prefix, rest) = hash.split_at(2);
            self.root.join(prefix).join(rest)
        } else if hash.is_empty() {
            // An empty key would otherwise resolve to the root directory
            // itself; give it a concrete, non-directory leaf instead.
            self.root.join("_short").join("_empty")
        } else {
            self.root.join("_short").join(hash)
        }
    }
}
