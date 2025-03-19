use super::{AnyTwine, Strand, Tixel, TwineBlock};
use crate::schemas::{StrandSchemaVersion, TixelSchemaVersion};
use crate::{crypto::get_hasher, errors::VerificationError};
use either::Either;
use ipld_core::cid::Cid;
use serde::{Deserialize, Serialize, Serializer};

/// A data structure representing a format for twine data that includes a CID
///
/// This is useful for serializing and deserializing twine data as JSON.
/// For v1 twine data, the CID is needed to know the hash algorithm
/// to compute the CID.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Tagged<T> {
  cid: Cid,
  data: T,
}

impl<T> Tagged<T> {
  /// Unpack the twine data (Strand or Tixel) from the tagged data structure
  pub fn unpack(self) -> T {
    self.data
  }
}

impl<T: TwineBlock> Tagged<T> {
  /// Create a new tagged data structure from the twine data
  pub fn new(data: T) -> Self {
    let cid = data.cid().clone();
    Tagged { cid, data }
  }
}

impl<T> From<T> for Tagged<T>
where
  T: TwineBlock,
{
  fn from(data: T) -> Self {
    Tagged::new(data)
  }
}

impl TryFrom<Tagged<StrandSchemaVersion>> for Tagged<Strand> {
  type Error = VerificationError;

  fn try_from(c: Tagged<StrandSchemaVersion>) -> Result<Self, Self::Error> {
    let cid = c.cid;
    let container = match c.data {
      // v1 requires recomputing the CID
      mut container @ StrandSchemaVersion::V1(_) => {
        let hasher = get_hasher(&cid)?;
        container.compute_cid(hasher);
        container
      }
      container @ StrandSchemaVersion::V2(_) => container,
    };
    let data = Strand::try_new(container)?;
    data.verify_cid(&cid)?;
    let tagged = Tagged::new(data);
    Ok(tagged)
  }
}

impl TryFrom<Tagged<TixelSchemaVersion>> for Tagged<Tixel> {
  type Error = VerificationError;

  fn try_from(c: Tagged<TixelSchemaVersion>) -> Result<Self, Self::Error> {
    let cid = c.cid;
    let container = match c.data {
      // v1 requires recomputing the CID
      mut container @ TixelSchemaVersion::V1(_) => {
        let hasher = get_hasher(&cid)?;
        container.compute_cid(hasher);
        container
      }
      container @ TixelSchemaVersion::V2(_) => container,
    };
    let data = Tixel::try_new(container)?;
    data.verify_cid(&cid)?;
    let tagged = Tagged::new(data);
    Ok(tagged)
  }
}

impl<'de> Deserialize<'de> for Tagged<Strand> {
  fn deserialize<D: serde::Deserializer<'de>>(
    deserializer: D,
  ) -> std::result::Result<Self, D::Error> {
    let c: Tagged<StrandSchemaVersion> = Tagged::deserialize(deserializer)?;
    Tagged::try_from(c).map_err(serde::de::Error::custom)
  }
}

impl<'de> Deserialize<'de> for Tagged<Tixel> {
  fn deserialize<D: serde::Deserializer<'de>>(
    deserializer: D,
  ) -> std::result::Result<Self, D::Error> {
    let c: Tagged<TixelSchemaVersion> = Tagged::deserialize(deserializer)?;
    Tagged::try_from(c).map_err(serde::de::Error::custom)
  }
}

impl<'de> Deserialize<'de> for Tagged<AnyTwine> {
  fn deserialize<D: serde::Deserializer<'de>>(
    deserializer: D,
  ) -> std::result::Result<Self, D::Error> {
    #[derive(Deserialize)]
    #[serde(transparent)]
    struct EitherContainer(
      #[serde(with = "either::serde_untagged")]
      Either<Tagged<StrandSchemaVersion>, Tagged<TixelSchemaVersion>>,
    );
    let item = EitherContainer::deserialize(deserializer)?;
    match item.0 {
      Either::Left(c) => {
        let c: Tagged<Strand> = Tagged::try_from(c).map_err(serde::de::Error::custom)?;
        Ok(Tagged::new(AnyTwine::from(c.data)))
      }
      Either::Right(c) => {
        let c: Tagged<Tixel> = Tagged::try_from(c).map_err(serde::de::Error::custom)?;
        Ok(Tagged::new(AnyTwine::from(c.data)))
      }
    }
  }
}

impl Serialize for Tagged<Strand> {
  fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
    let c = Tagged {
      cid: self.cid.clone(),
      data: self.data.0.clone(),
    };
    c.serialize(serializer)
  }
}

impl Serialize for Tagged<Tixel> {
  fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
    let c = Tagged {
      cid: self.cid.clone(),
      data: self.data.0.clone(),
    };
    c.serialize(serializer)
  }
}

impl Serialize for Tagged<AnyTwine> {
  fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
    match &self.data {
      AnyTwine::Strand(s) => {
        let c = Tagged {
          cid: self.cid.clone(),
          data: s.0.clone(),
        };
        c.serialize(serializer)
      }
      AnyTwine::Tixel(t) => {
        let c = Tagged {
          cid: self.cid.clone(),
          data: t.0.clone(),
        };
        c.serialize(serializer)
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::{
    test::{STRANDJSON, TIXELJSON},
    twine::Strand,
  };

  #[test]
  fn test_strand_tagged() {
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
  fn test_tixel_tagged() {
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

  #[test]
  fn test_any_twine_tagged() {
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Thing {
      twine: Tagged<AnyTwine>,
    }

    let _: Tagged<AnyTwine> = serde_ipld_dagjson::from_slice(STRANDJSON.as_bytes()).unwrap();
    let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();

    let thing = Thing {
      twine: Tagged::new(AnyTwine::from(strand)),
    };

    let s = serde_ipld_dagjson::to_vec(&thing).unwrap();
    println!("{}", String::from_utf8(s).unwrap());
    let encoded = serde_ipld_dagjson::to_vec(&thing).unwrap();
    let decoded: Thing = serde_ipld_dagjson::from_slice(&encoded).unwrap();
    assert_eq!(thing, decoded);

    let thing = Thing {
      twine: Tagged::new(AnyTwine::from(tixel)),
    };

    let s = serde_ipld_dagjson::to_vec(&thing).unwrap();
    println!("{}", String::from_utf8(s).unwrap());
    let encoded = serde_ipld_dagjson::to_vec(&thing).unwrap();
    let decoded: Thing = serde_ipld_dagjson::from_slice(&encoded).unwrap();
    assert_eq!(thing, decoded);
  }
}
