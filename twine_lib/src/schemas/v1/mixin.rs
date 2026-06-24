use std::hash::Hash;

use crate::Cid;
use serde::{Deserialize, Serialize};

use crate::twine::Stitch;

/// A Mixin is the old name for a Stitch
///
/// This represents the old way it was stored in the data structure
#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Mixin {
  /// The chain CID
  pub chain: Cid,
  /// The Tixel CID
  pub value: Cid,
}

impl Hash for Mixin {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.chain.hash().hash(state);
  }
}

impl From<Stitch> for Mixin {
  fn from(stitch: Stitch) -> Self {
    Mixin {
      chain: stitch.strand,
      value: stitch.tixel,
    }
  }
}

impl From<Mixin> for Stitch {
  fn from(mixin: Mixin) -> Self {
    Stitch {
      strand: mixin.chain,
      tixel: mixin.value,
    }
  }
}

impl FromIterator<Mixin> for Vec<Stitch> {
  fn from_iter<I: IntoIterator<Item = Mixin>>(iter: I) -> Self {
    iter.into_iter().map(Stitch::from).collect()
  }
}

impl FromIterator<Stitch> for Vec<Mixin> {
  fn from_iter<I: IntoIterator<Item = Stitch>>(iter: I) -> Self {
    iter.into_iter().map(Mixin::from).collect()
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use std::collections::HashSet;

  fn cids() -> (Cid, Cid) {
    (
      "bafyriqe3zxf5g4ifgqhea5zozxpdcfi5qcpkfpogtxzbizmmuxdjuzuq44a2cifbr7xplo4kcfsdz2c5pxfxektavrqxxb3nvbmclxz7qiz6e".parse().unwrap(),
      "bafyriqgqnqyqrjpq54oy5zv4w3ev4zm36rjuhbmmvw2noaqeii2ru3azvm6tc7qcknrdzegh44rbszdd3lr6m5cwnl7eohzubx2uolhqab2qm".parse().unwrap(),
    )
  }

  #[test]
  fn stitch_mixin_round_trip() {
    let (strand, tixel) = cids();
    let stitch = Stitch { strand, tixel };
    let mixin: Mixin = stitch.into();
    assert_eq!(mixin.chain, strand);
    assert_eq!(mixin.value, tixel);
    let back: Stitch = mixin.into();
    assert_eq!(back, stitch);
  }

  #[test]
  fn hash_only_depends_on_chain() {
    let (strand, tixel) = cids();
    let a = Mixin { chain: strand, value: tixel };
    // Same chain, different value still hashes equal (chain-only hash).
    let b = Mixin { chain: strand, value: strand };
    let mut set = HashSet::new();
    set.insert(hash_of(&a));
    assert!(set.contains(&hash_of(&b)));
  }

  fn hash_of(m: &Mixin) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    m.hash(&mut h);
    h.finish()
  }

  #[test]
  fn from_iterator_both_directions() {
    let (strand, tixel) = cids();
    let stitches = vec![Stitch { strand, tixel }];
    let mixins: Vec<Mixin> = stitches.clone().into_iter().collect();
    assert_eq!(mixins.len(), 1);
    let restored: Vec<Stitch> = mixins.into_iter().collect();
    assert_eq!(restored, stitches);
  }

  #[test]
  fn deny_unknown_fields_on_deserialize() {
    let (strand, tixel) = cids();
    let mixin = Mixin { chain: strand, value: tixel };
    let json = serde_json::to_value(&mixin).unwrap();
    // Round-trips cleanly.
    let _: Mixin = serde_json::from_value(json.clone()).unwrap();

    // An extra field is rejected.
    let mut obj = json.as_object().unwrap().clone();
    obj.insert("extra".into(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<Mixin>(serde_json::Value::Object(obj)).is_err());
  }
}
