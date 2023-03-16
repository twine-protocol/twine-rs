use serde::Serialize;

fn main() {
    let chain = ...;// somehow parse the chain
    let hasher: multihash::Code = ...;
    let previous = ...;// somehow get the previous pulse
    let mut mixins: Vec<Any> = Vec::new(); // TODO: should the mixins be any type, or just CIDs?
    let payload: impl Serialize = ...;
    let pulse = chain.create_pulse(
        previous, // TODO: should the chain store previous?
        mixins,
        payload,
        hasher, 
    );
}