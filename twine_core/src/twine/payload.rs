use libipld::Ipld;
use libipld::error::SerdeError;
use serde::{Serialize, Deserialize};
use crate::specification::Subspec;

pub trait Payload where Self: Sized {
  fn from_ipld(_spec: Subspec, ipld: Ipld) -> Result<Self, SerdeError>;
}

// This is awkward but it ensures that untagged enums could work
impl<T> Payload for T where T: Serialize + for<'de> Deserialize<'de> {
  fn from_ipld(_spec: Subspec, ipld: Ipld) -> Result<Self, SerdeError> {
    use ipld_core::codec::Codec;
    use serde_ipld_dagcbor::codec::DagCborCodec;
    let encoded = DagCborCodec::encode_to_vec(&ipld).unwrap();
    let result : Self = DagCborCodec::decode_from_slice(&encoded).unwrap();
    Ok(result)
  }
}
