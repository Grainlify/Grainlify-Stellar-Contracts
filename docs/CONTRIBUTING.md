# Contributing

## Build artifact hygiene

Do not commit generated Rust, Soroban, or SDK build outputs. Keep `target/`, `*.d`, `*.wasm`, `*.rs.bk`, editor files, OS metadata, and local test artifacts out of version control.

Before opening a pull request that touches build configuration, run:

```bash
git ls-files | grep -E '\.(d|wasm)$' && exit 1 || true
git status --porcelain
```

The tracked source tree should not contain generated `.d` or `.wasm` files. After a local build, `git status --porcelain` should only show intentional source changes.
