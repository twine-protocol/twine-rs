use either::Either;
use ipld_core::cid::Cid;
use serde::{Deserialize, Serialize, Serializer};
use crate::{crypto::get_hasher, errors::VerificationError};

use super::{AnyTwine, Strand, StrandContainerVersion, Tixel, TixelContainerVersion, TwineBlock};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Tagged<T> {
  cid: Cid,
  data: T,
}

impl<T> Tagged<T> {
  pub fn unpack(self) -> T {
    self.data
  }
}

impl<T: TwineBlock> Tagged<T> {
  pub fn new(data: T) -> Self {
    let cid = data.cid().clone();
    Tagged { cid, data }
  }
}

impl From<Strand> for Tagged<Strand> {
  fn from(data: Strand) -> Self {
    Tagged::new(data)
  }
}

impl From<Tixel> for Tagged<Tixel> {
  fn from(data: Tixel) -> Self {
    Tagged::new(data)
  }
}

impl From<AnyTwine> for Tagged<AnyTwine> {
  fn from(data: AnyTwine) -> Self {
    let cid = data.cid();
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
    let data = Strand::try_new(container)?;
    data.verify_cid(&cid)?;
    let tagged = Tagged::new(data);
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
    let data = Tixel::try_new(container)?;
    data.verify_cid(&cid)?;
    let tagged = Tagged::new(data);
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

impl<'de> Deserialize<'de> for Tagged<AnyTwine> {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
    if let Ok(c) = Tagged::<Either<StrandContainerVersion, TixelContainerVersion>>::deserialize(deserializer) {
      match c.data {
        Either::Left(sc) => {
          let strand = Strand::try_new(sc).map_err(serde::de::Error::custom)?;
          Ok(Tagged { cid: c.cid, data: strand.into() })
        },
        Either::Right(tc) => {
          let tixel = Tixel::try_new(tc).map_err(serde::de::Error::custom)?;
          Ok(Tagged { cid: c.cid, data: tixel.into() })
        },
      }
    } else {
      Err(serde::de::Error::custom("failed to deserialize Tagged<AnyTwine>"))
    }
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

impl Serialize for Tagged<AnyTwine> {
  fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
    let data = match &self.data {
      AnyTwine::Strand(s) => {
        let c = Tagged { cid: self.cid.clone(), data: s.0.as_inner() };
        Either::Left(c)
      },
      AnyTwine::Tixel(t) => {
        let c = Tagged { cid: self.cid.clone(), data: t.0.as_inner() };
        Either::Right(c)
      },
    };
    let c = Tagged { cid: self.cid.clone(), data };
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
