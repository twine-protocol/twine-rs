use twine_core::{twine::{Pulse, Chain}, sign::DefaultSigner};

fn main() {
    // create a chain
    let signer = DefaultSigner{};
    let hasher: multihash::Code = multihash::Code::Sha3_512;
    let chain = Chain::builder("gold".into()).finalize(signer, hasher)?;
    
    
    let first = chain.first(
        Vec::new(),
        HashMap::from(vec![(String::from("count"), 1)]),
        signer,
        hasher
    )?;

    let next = chain.pulse(
        first,
        Vec::new(),
        HashMap::from(vec![(String::from("count"), 2)]),
        signer,
        hasher
    )?;

    assert_eq!(next.content.index, first.content.index + 1)
}