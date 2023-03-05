use crate::sign::Signer;
use libipld::multihash;
use serde::Serialize;

pub struct ChainContent {
    source: String,
    specification: String,
    radix: u32,
    mixins: Vec<dyn Serialize>, // TODO: how to get these `dyn`s to work...
    meta: dyn Serialize

}

pub struct Chain {  }

#[derive(Debug)]
pub enum TwineError {}

impl Chain {
    pub fn builder(content: ChainContent, signer: Signer, hasher: multihash::Code) -> Result<Self, TwineError> {
        return Ok(Chain {  });
    }
}
