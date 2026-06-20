# Twine Builder

[![Crates.io Version](https://img.shields.io/crates/v/twine_builder)](https://crates.io/crates/twine_builder)
[![docs.rs (with version)](https://img.shields.io/docsrs/twine_builder/latest)](https://docs.rs/twine_builder/latest/twine_builder/)

This crate contains functionality for creating Twine data structures. It can
be bundled and used from the [twine_protocol crate](https://crates.io/crates/twine_protocol) by enabling the `build` feature.

## Usage

Normal construction of twine data involves the following:

- Creating a [`Signer`] to sign data.
- Creating a [`TwineBuilder`] and providing it with the signer.
- Calling the build methods of the builder to construct data.
- Saving that data to some [`twine_lib::store::Store`].

See the documentation for specifics about the [`Signer`], [`TwineBuilder`],
and [`Store`](https://docs.rs/twine_lib/latest/twine_lib/store/trait.Store.html).

## Signers

No signer is enabled by default — pick a backend via a feature flag:

- `rustcrypto-signer` provides `RustCryptoSigner`, the recommended pure-Rust
  signer for v2 data. It emits canonical low-S ECDSA signatures, zeroizes key
  material on drop, and supports PKCS#8 (and, with the `encryption` feature,
  password-encrypted PKCS#8) key import/export.
- `ring-signer` provides the **deprecated** `RingSigner`. It emits non-canonical
  high-S ECDSA signatures that v2 verification rejects; prefer `RustCryptoSigner`.
- `rsa` adds RSA support to whichever signer backend is enabled.

## Version 1 data

In order to construct version 1 data structures, the `v1` feature flag
must be enabled and a `BiscuitSigner` can be used.
