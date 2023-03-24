use twine_core::twine::{Chain, ChainContent};

fn main() {
    let hasher = multihash::Code::Sha3_512;
    let our_signer = Signer::from_random()?;
    let our_chain = Chain::build_chain(
        ChainContent {
            source: "twine".to_string(),
            specification: "twine/1.0.x".to_string(),
            radix: 5,
            key: our_signer.public_key(), // TODO: should we allow users to set this?
            mixins: vec![1,2,3,4],
            meta: "not much to say..."
        },
        signer,
        hasher,
    )?;
    
    let hasher = multihash::Code::Sha3_512;
    let their_signer = Signer::from_random()?;
    let their_chain = Chain::build_chain(
        ChainContent {
            source: "twine".to_string(),
            specification: "twine/1.0.x".to_string(),
            radix: 5,
            key: their_signer.public_key(),
            mixins: vec![our_chain.cid],
            meta: "not much to say..."
        },
        signer,
        hasher,
    )?;

    // TODO: create pulses here    

}