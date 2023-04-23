use libipld::{Cid, multihash::Code};

pub fn hasher_of(cid: Cid) -> Option<Code> {
    Code::try_from(cid.codec())
}