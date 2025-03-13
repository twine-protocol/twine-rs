use ipld_core::codec::Codec;
use crate::{Cid, Ipld};
use serde::{Deserialize, Serialize};
use serde_ipld_dagjson::codec::DagJsonCodec;
use crate::errors::VerificationError;

#[derive(Serialize, Deserialize)]
pub struct TwineContainerJson<T> {
  pub cid: Cid,
  pub data: T,
}

pub fn split_json_objects<S: AsRef<str>>(json: S) -> Result<Vec<String>, VerificationError> {
  let objects: Vec<Ipld> = DagJsonCodec::decode_from_slice(json.as_ref().as_bytes())?;
  objects.into_iter()
    .map(|object| Ok(String::from_utf8(DagJsonCodec::encode_to_vec(&object)?).unwrap()))
    .collect()
}
