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
            // Atomic write: write to .tmp, then rename.
            let tmp = p.with_extension("tmp");
            fs::write(&tmp, bytes)?;
            fs::rename(&tmp, &p)?;
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
    pub fn path_for(&self, hash: &str) -> PathBuf {
        let (prefix, rest) = hash.split_at(2);
        self.root.join(prefix).join(rest)
    }
}
