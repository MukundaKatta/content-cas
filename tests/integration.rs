use content_cas::Cas;
use std::env;

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let p = env::temp_dir().join(format!(
        "content-cas-test-{}-{}-{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    if p.exists() {
        std::fs::remove_dir_all(&p).ok();
    }
    p
}

#[test]
fn put_then_get_roundtrip() {
    let cas = Cas::new(tmp_dir("rt")).unwrap();
    let h = cas.put(b"hello").unwrap();
    assert_eq!(h.len(), 64);
    assert_eq!(cas.get(&h).unwrap().as_deref(), Some(&b"hello"[..]));
}

#[test]
fn put_is_idempotent() {
    let cas = Cas::new(tmp_dir("idem")).unwrap();
    let h1 = cas.put(b"data").unwrap();
    let h2 = cas.put(b"data").unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn missing_key_returns_none() {
    let cas = Cas::new(tmp_dir("miss")).unwrap();
    assert!(cas.get("0".repeat(64).as_str()).unwrap().is_none());
}

#[test]
fn contains_works() {
    let cas = Cas::new(tmp_dir("cont")).unwrap();
    let h = cas.put(b"x").unwrap();
    assert!(cas.contains(&h));
    assert!(!cas.contains("a".repeat(64).as_str()));
}

#[test]
fn remove_works() {
    let cas = Cas::new(tmp_dir("rm")).unwrap();
    let h = cas.put(b"y").unwrap();
    assert!(cas.remove(&h).unwrap());
    assert!(!cas.contains(&h));
    assert!(!cas.remove(&h).unwrap());
}

#[test]
fn path_for_uses_2char_prefix() {
    let cas = Cas::new(tmp_dir("path")).unwrap();
    let p = cas.path_for("abcdef0123");
    assert!(p.to_string_lossy().ends_with("ab/cdef0123"));
}

#[test]
fn short_keys_do_not_panic() {
    // Callers may pass arbitrary keys to get/contains/remove; keys shorter
    // than the 2-char shard prefix must be handled gracefully, not panic.
    let cas = Cas::new(tmp_dir("short")).unwrap();
    assert!(cas.get("").unwrap().is_none());
    assert!(cas.get("a").unwrap().is_none());
    assert!(!cas.contains(""));
    assert!(!cas.contains("a"));
    assert!(!cas.remove("").unwrap());
    assert!(!cas.remove("a").unwrap());
    // path_for itself must also not panic on short keys.
    let _ = cas.path_for("");
    let _ = cas.path_for("a");
}

#[test]
fn concurrent_puts_of_same_content_succeed() {
    use std::sync::Arc;
    use std::thread;

    let cas = Arc::new(Cas::new(tmp_dir("concurrent")).unwrap());
    let payload = b"the same immutable blob".to_vec();

    let handles: Vec<_> = (0..16)
        .map(|_| {
            let cas = Arc::clone(&cas);
            let payload = payload.clone();
            thread::spawn(move || cas.put(&payload).unwrap())
        })
        .collect();

    let hashes: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Every writer agrees on the key and the stored bytes are intact.
    let first = &hashes[0];
    assert!(hashes.iter().all(|h| h == first));
    assert_eq!(cas.get(first).unwrap().as_deref(), Some(&payload[..]));
}
