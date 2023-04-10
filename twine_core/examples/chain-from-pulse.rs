use std::collections::HashMap;

use libipld::multihash::Code;
use twine_core::{twine::{Pulse, Chain}, sign::DefaultSigner};

fn main() {
    // create a pulse
    let signer = DefaultSigner{};
    let hasher = Code::Sha3_512;
    let chain = Chain::builder(String::from("gold")).finalize(signer, hasher)?;
    let pulse = chain.first(
        Vec::new(),
        HashMap::from(vec![("hello", 1), ("world", 2)]),
        signer,
        hasher
    );

    let retrieved_chain = pulse.chain();
    assert_eq!(retrieved_chain, chain)
}