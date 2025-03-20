# Contributing

## Compiling documentation locally

```sh
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --workspace --all-features --no-deps
```
