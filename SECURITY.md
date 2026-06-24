# Security notes

## Signature canonicalization (ECDSA low-S)

ECDSA signatures are **malleable**: for any valid signature `(r, s)` the value
`(r, n - s)` (where `n` is the curve order) is *also* a valid signature for the
same message and key, and anyone can compute it without the private key. Because
a Twine CID is hashed over **content *and* signature**, a malleated signature
produces a record with identical content but a *different* CID — a "twin" of the
original. This can lead to branching.

The canonical fix is to require **low-S** signatures (`s <= n / 2`), so each
`(key, content)` pair maps to exactly one valid signature and therefore one CID.

Twine applies this policy **asymmetrically by version**:

| Version | ECDSA verification | Rationale |
| ------- | ------------------ | --------- |
| **v1**  | **Permissive** (high-S accepted) | v1 chains were signed by the original JS implementation, which emits high-S signatures roughly half the time (including some strand self-signatures). Rejecting them would make already-deployed chains — e.g. `random.colorado.edu` — unverifiable. v1 verification runs through biscuit JWS (`twine_lib::crypto::jws`). |
| **v2**  | **Strict** (high-S rejected) | v2 is canonical-by-construction. `twine_lib::crypto::PublicKey::verify` (the `verify_ecdsa` path) rejects non-canonical high-S signatures. |

When producing v2 data, sign with a signer that emits canonical low-S
signatures. [`twine_builder::RustCryptoSigner`] does this for you; the deprecated
`RingSigner` does **not** and will produce v2 data that fails verification.
