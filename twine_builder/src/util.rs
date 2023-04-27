use libipld::{Cid, multihash::Code};

pub fn hasher_of(&cid: &Cid) -> Result<Code, libipld::multihash::Error> {
    Code::try_from(cid.hash().code())
}