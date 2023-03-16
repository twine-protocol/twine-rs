use crate::sign::Signer;
use crate::twine::{Chain};

pub enum TwineError {}

impl Chain {
    pub fn builder(content: ChainContent, signer: Signer, hasher: multihash::Code) -> Result<Self, TwineError> {
        return Ok(Chain {  });
    }
}
