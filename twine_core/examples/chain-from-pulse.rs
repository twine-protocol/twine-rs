use std::collections::HashMap;

use josekit::{jws::alg::eddsa::EddsaJwsAlgorithm, jwk::alg::ed::EdCurve::Ed25519};
use libipld::multihash::Code;
use twine_core::{twine::{Chain}};

fn main() {
    // create a pulse
    let alg = EddsaJwsAlgorithm::Eddsa;
    let keys = alg.generate_key_pair(Ed25519)?;
    let signer = alg.signer_from_jwk(&keys.to_jwk_private_key());
    let hasher = Code::Sha3_512;
    let chain = Chain::builder(String::from("gold"), keys.to_jwk_public_key()).finalize(signer, hasher);
    let pulse = chain.first(
        Vec::new(),
        HashMap::from(vec![("hello", 1), ("world", 2)]),
        signer,
        hasher
    );

    let retrieved_chain = pulse.chain();
    assert_eq!(retrieved_chain, chain)
}