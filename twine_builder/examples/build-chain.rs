use josekit::{jws::{alg::eddsa::EddsaJwsAlgorithm, JwsVerifier}, jwk::alg::ed::EdCurve::Ed25519};
use twine_core::{twine::Chain};
use twine_builder::ChainBuilder;
use libipld::cid::multihash;

fn main() -> Result<Chain, ChainError> {
    let keys = EddsaJwsAlgorithm::Eddsa.generate_key_pair(Ed25519)?;
    let signer = EddsaJwsAlgorithm::Eddsa.signer_from_jwk(keys)?;
    let hasher = multihash::Code::Sha3_512; 
    let builder = ChainBuilder::new("gold".into(), keys.to_jwk_public_key());
    let chain = builder.finalize(signer, hasher)?;

    // builder is consumed, so this should be invalid:
    builder
}