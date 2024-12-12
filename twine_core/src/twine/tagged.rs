use ipld_core::cid::Cid;
use serde::{Deserialize, Serialize, Serializer};
use crate::{crypto::get_hasher, errors::VerificationError};

use super::{Strand, StrandContainerVersion, Tixel, TixelContainerVersion, TwineBlock};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Tagged<T> {
  pub cid: Cid,
  pub data: T,
}

impl<T: TwineBlock> Tagged<T> {
  pub fn new(data: T) -> Self {
    let cid = data.cid().clone();
    Tagged { cid, data }
  }
}

impl TryFrom<Tagged<StrandContainerVersion>> for Tagged<Strand> {
  type Error = VerificationError;

  fn try_from(c: Tagged<StrandContainerVersion>) -> Result<Self, Self::Error> {
    let cid = c.cid;
    let container = match c.data {
      // v1 requires recomputing the CID
      mut container@StrandContainerVersion::V1(_) => {
        let hasher = get_hasher(&cid)?;
        container.compute_cid(hasher);
        container
      },
      container@StrandContainerVersion::V2(_) => container,
    };
    let tagged = Tagged { cid, data: Strand::try_new(container)? };
    Ok(tagged)
  }
}

impl TryFrom<Tagged<TixelContainerVersion>> for Tagged<Tixel> {
  type Error = VerificationError;

  fn try_from(c: Tagged<TixelContainerVersion>) -> Result<Self, Self::Error> {
    let cid = c.cid;
    let container = match c.data {
      // v1 requires recomputing the CID
      mut container@TixelContainerVersion::V1(_) => {
        let hasher = get_hasher(&cid)?;
        container.compute_cid(hasher);
        container
      },
      container@TixelContainerVersion::V2(_) => container,
    };
    let tagged = Tagged { cid, data: Tixel::try_new(container)? };
    Ok(tagged)
  }
}

impl<'de> Deserialize<'de> for Tagged<Strand> {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
    let c: Tagged<StrandContainerVersion> = Tagged::deserialize(deserializer)?;
    Tagged::try_from(c).map_err(serde::de::Error::custom)
  }
}

impl<'de> Deserialize<'de> for Tagged<Tixel> {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
    let c: Tagged<TixelContainerVersion> = Tagged::deserialize(deserializer)?;
    Tagged::try_from(c).map_err(serde::de::Error::custom)
  }
}

impl Serialize for Tagged<Strand> {
  fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
    let c = Tagged { cid: self.cid.clone(), data: self.data.0.as_inner() };
    c.serialize(serializer)
  }
}

impl Serialize for Tagged<Tixel> {
  fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
    let c = Tagged { cid: self.cid.clone(), data: self.data.0.as_inner() };
    c.serialize(serializer)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::{test::{STRANDJSON, TIXELJSON}, twine::Strand};

  #[test]
  fn test_strand_tagged(){
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Thing {
      strand: Tagged<Strand>,
    }

    let _: Tagged<Strand> = serde_ipld_dagjson::from_slice(STRANDJSON.as_bytes()).unwrap();
    let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();

    let thing = Thing {
      strand: Tagged::new(strand),
    };

    let s = serde_ipld_dagjson::to_vec(&thing).unwrap();
    println!("{}", String::from_utf8(s).unwrap());
    let encoded = serde_ipld_dagjson::to_vec(&thing).unwrap();
    let decoded: Thing = serde_ipld_dagjson::from_slice(&encoded).unwrap();
    assert_eq!(thing, decoded);
  }

  #[test]
  fn test_tixel_tagged(){
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Thing {
      tixel: Tagged<Tixel>,
    }

    let _: Tagged<Tixel> = serde_ipld_dagjson::from_slice(TIXELJSON.as_bytes()).unwrap();
    let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();

    let thing = Thing {
      tixel: Tagged::new(tixel),
    };

    let s = serde_ipld_dagjson::to_vec(&thing).unwrap();
    println!("{}", String::from_utf8(s).unwrap());
    let encoded = serde_ipld_dagjson::to_vec(&thing).unwrap();
    let decoded: Thing = serde_ipld_dagjson::from_slice(&encoded).unwrap();
    assert_eq!(thing, decoded);
  }

}
