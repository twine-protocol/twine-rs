use twine_core::{twine::Chain, sign::DefaultSigner};
use libipld::cid::multihash;

fn main() -> Result<Chain, ChainError> {
    let signer = DefaultSigner {};
    let hasher = multihash::Code::Sha3_512; 
    let chain: Chain = Chain::builder("gold".into()).finalize(signer, hasher)?;
}