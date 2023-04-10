use twine_core::{twine::Chain, sign::DefaultSigner};
use libipld::cid::multihash;

fn main() -> Result<Chain, ChainError> {
    let signer = DefaultSigner {};
    let hasher = multihash::Code::Sha3_512; 
    let builder = Chain::builder("gold".into());
    let chain = builder.finalize(signer, hasher)?;
    // builder is consumed
    builder
}