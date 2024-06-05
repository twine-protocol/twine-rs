mod tixel;
mod strand;

pub use tixel::*;
pub use strand::*;

pub type Bytes = Vec<[u8]>;
pub type V2 = crate::specification::Specification<1>;

impl Default for V2 {
  fn default() -> Self {
    Self("twine/2.0.0".into())
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ContainerV2<C: Clone + Verifiable + Send> {
  #[serde(skip)]
  cid: Cid,

  content: Verified<C>,
  signature: Bytes,
}
