use crate::errors::VerificationError;
use crate::twine::container::TwineContainer;
use crate::twine::container::TwineContent;
use ipld_core::codec::Codec;
use serde_ipld_dagjson::codec::DagJsonCodec;
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize)]
pub struct TwineContainerJson<T: TwineContent> {
  pub cid: Cid,
  pub data: TwineContainer<T>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum DagJsonDecodeResult<T: TwineContent> {
  One(TwineContainerJson<T>),
  Many(Vec<TwineContainerJson<T>>),
}

pub fn decode_dag_json<T: TwineContent + Serialize + for<'de> Deserialize<'de>, S: Display>(json: S) -> Result<DagJsonDecodeResult<T>, VerificationError> {
  Ok(DagJsonCodec::decode_from_slice(json.to_string().as_bytes())?)
}
