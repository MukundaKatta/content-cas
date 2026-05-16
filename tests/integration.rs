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
