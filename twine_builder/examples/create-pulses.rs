use std::io::Chain;
use josekit::{jws::{JwsSigner, alg::eddsa::EddsaJwsAlgorithm}, jwk::alg::ed::EdCurve::Ed25519};
use twine_builder::{ChainBuilder, PulseBuilder};
use twine_core::twine::Pulse;

fn main() {
    // create a chain
    let alg = EddsaJwsAlgorithm::Eddsa;
    let keys = alg.generate_key_pair(Ed25519)?;
    let signer = alg.signer_from_jwk(keys);
    let hasher: multihash::Code = multihash::Code::Sha3_512;
    let chain = ChainBuilder::new("gold".into(), keys.to_jwk_public_key()).finalize(signer, hasher)?;
    
    let first = PulseBuilder::first(chain)?.payload(HashMap::from(vec![("count", 1)]))?.finalize(signer);    
    let next = PulseBuilder::new(chain, first)
        ?.payload(HashMap::from(vec![(String::from("count"), 2)]))
        ?.finalize(signer);

    assert_eq!(next.content.index, first.content.index + 1)
}