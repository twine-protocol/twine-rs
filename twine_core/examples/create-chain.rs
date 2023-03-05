use twine_core::{Chain, sign::Signer, chain::ChainContent};
use libipld::cid::multihash;

fn main() -> Result<Chain, ChainError> {
    let signer = Signer{};
    let hasher: multihash::Code = multihash::Code::Sha3_512; // that implements
    let our_chain = Chain::builder(
        ChainContent {
            source: "twine".to_string(),
            specification: "twine/1.0.x".to_string(),
            radix: 5,
            mixins: vec![1,2,3,4],
            meta: "not much to say..."
        },
        signer,
        hasher,
    )?;
}