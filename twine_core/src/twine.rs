//! Structs and traits common to both Chain's and Pulses

use std::collections::HashMap;

use libipld::{Ipld, Link};

const DEFAULT_SPECIFICATION: &str = "twine/1.0.x"; // TODO: should setting this be a build time macro?

struct Mixin {
    chain: Link,
    value: Link
}

struct ChainContent {
    source: String,
    specification: String,
    radix: i64, // TODO: sizing; TODO: links_radix instead?
    key: Jwk, 
    mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
    meta: Ipld, // TODO: should be a map?
}

struct PulseContent {
    source: String,
    chain: Link,
    // TODO: check that u32 + range is same as i64 + range
    index: u32, // based on dag-cbor: https://ipld.io/docs/data-model/kinds/#integer-kind
    // TODO: good enough replacement for Ipld::List<Link>, which is Vec<Ipld, Global>? What if that definition changes?
    previous: Vec<Link>,
    mixins: Vec<Mixin>, // we check that these links are not on the same chain at runtime
    payload: HashMap<String, dyn Into<Ipld>> // TODO: is payload supposed to be a Map? (see specs/twine/data-structures.md)
}

pub enum TwineContent { // TODO: should devs be able to use TwineContent directly?
    Chain(ChainContent),
    Pulse(PulseContent)
}

impl TwineContent {

}

struct Twine<T: TwineContent> {
    content: TwineContent,
    signature: JwsCondensed
}

pub type Chain = Twine<ChainContent>;
pub type Pulse = Twine<PulseContent>;
