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

## Repository Health

This repository includes a dependency-free health check for core documentation, metadata, and CI wiring. Run it locally before publishing changes:

```sh
python3 scripts/check_repository_health.py
```

The same check runs in GitHub Actions on pushes and pull requests.
