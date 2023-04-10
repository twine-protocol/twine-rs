use libipld::multihash::{Code, MultihashDigest};
use twine_core::{twine::Chain, sign::{DefaultSigner, Signer}};

fn main() {
    let signer = DefaultSigner::from_random()?;
    let hasher = Code::Sha3_512;
    let chain = Chain::builder(String::from("hello")).finalize(signer, hasher)?;
    let serialized = serde_ipld_dagcbor::to_vec(&chain)?;
    let replicated_chain: Chain = serde_ipld_dagcbor::from_slice(&serialized)?;
    assert_eq!(chain, replicated_chain)
}