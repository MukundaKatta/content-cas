# content-cas

[![crates.io](https://img.shields.io/crates/v/content-cas.svg)](https://crates.io/crates/content-cas)

Content-addressed cache on disk. Store bytes by their SHA-256, retrieve
by hex. `root/aa/bbb...` layout for fast `ls`.

```rust
use content_cas::Cas;
let cas = Cas::new("/tmp/my-cas").unwrap();
let h = cas.put(b"hello").unwrap();
let bytes = cas.get(&h).unwrap();
```

Zero deps. MIT or Apache-2.0.
