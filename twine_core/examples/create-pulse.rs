use twine_core::twine::{Pulse, Chain};

fn main() {
    // create a chain
    let signer = Signer{};
    let hasher: multihash::Code = multihash::Code::Sha3_512; // that implements
    let chain = Chain::build_chain(
        ChainContent {
            source: "twine".to_string(),
            specification: "twine/1.0.x".to_string(),
            radix: 5,
            mixins: vec![],
            meta: "not much to say..."
        },
        signer,
        hasher,
    )?;

    // previous is none since there is no previous pulse
    let previous: Option<Pulse> = None; // TODO: should `previous` be an entire pulse or just a CID? how should we prevent branching?
    let mixins: Vec<Any> = Vec::new(); // TODO: should the mixins be any type, or just CIDs?
    let payload = vec![ ("foo", "bar" ) ].into_iter().collect();
    let first_pulse = chain.create_pulse(
        previous, // TODO: should the chain store previous?
        mixins,
        payload,
        signer,
        hasher, 
    )?;

    let second_pulse = chain.create_pulse(
        first_pulse, // TODO: need to figure out why this works even though it is not an `Option`
        mixins,
        payload,
        signer,
        hasher
    );
}