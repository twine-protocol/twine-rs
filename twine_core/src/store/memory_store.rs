use std::collections::BTreeMap;
use std::pin::Pin;
use std::{collections::HashMap, sync::Arc};
use futures::Stream;
use libipld::Cid;
use crate::prelude::{RangeQuery, ResolutionError, Resolver, Strand, Tixel, Twine};
use super::Store;
use std::error::Error;
use crate::as_cid::AsCid;
use crate::twine::AnyTwine;
use async_trait::async_trait;

#[derive(Debug, Clone)]
struct StrandMap {
  strand: Arc<Strand>,
  by_index: BTreeMap<u64, Arc<Tixel>>,
}

impl StrandMap {
  fn new(strand: Arc<Strand>) -> Self {
    Self {
      strand,
      by_index: BTreeMap::new(),
    }
  }
}

#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
  tixels: HashMap<Cid, Arc<Tixel>>,
  strands: HashMap<Cid, StrandMap>,
}

impl MemoryStore {
  pub fn new() -> Self {
    Self {
      tixels: HashMap::new(),
      strands: HashMap::new(),
    }
  }
}

#[async_trait]
impl Resolver for MemoryStore {
  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    let iter = self.strands.values().map(|s| Ok(s.strand.clone()));
    use futures::stream::StreamExt;
    let stream = futures::stream::iter(iter);
    Ok(stream.boxed())
  }

  async fn resolve_cid<'a, C: AsCid + Send>(&'a self, cid: C) -> Result<AnyTwine, ResolutionError> {
    let cid = cid.as_cid();
    if let Some(tixel) = self.tixels.get(&cid) {
      Ok(AnyTwine::Tixel(tixel.clone()))
    } else if let Some(strand) = self.strands.get(&cid) {
      Ok(AnyTwine::Strand(strand.strand.clone()))
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
    let cid = strand.as_cid();
    if let Some(s) = self.strands.get(&cid) {
      Ok(s.strand.clone())
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: u64) -> Result<Twine, ResolutionError> {
    let cid = strand.as_cid();
    if let Some(s) = self.strands.get(&cid) {
      if let Some(tixel) = s.by_index.get(&index) {
        Ok(Twine::try_new_from_shared(s.strand.clone(), tixel.clone())?)
      } else {
        Err(ResolutionError::NotFound)
      }
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError> {
    let cid = strand.as_cid();
    if let Some(s) = self.strands.get(&cid) {
      if let Some((_index, tixel)) = s.by_index.last_key_value() {
        Ok(Twine::try_new_from_shared(s.strand.clone(), tixel.clone())?)
      } else {
        Err(ResolutionError::NotFound)
      }
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn resolve_range<R: Into<RangeQuery> + Send>(&self, range: R) -> Result<Pin<Box<dyn Stream<Item = Result<Twine, ResolutionError>> + Send + '_>>, ResolutionError> {
    let range = range.into();
    let range = range.try_to_definite(self).await?;
    use futures::stream::StreamExt;
    let strand = self.resolve_strand(range.strand).await?;
    if let Some(entry) = self.strands.get(&strand.cid()) {
      let iter = entry.by_index.values()
        .skip(range.lower as usize)
        .take((range.upper - range.lower) as usize)
        .rev()
        .map(move |t| Ok(Twine::try_new_from_shared(strand.clone(), t.clone())?));
      let stream = futures::stream::iter(iter);
      Ok(stream.boxed())
    } else {
      Err(ResolutionError::NotFound)
    }
  }
}

#[async_trait]
impl Store for MemoryStore {
  async fn save<T: Into<AnyTwine> + Send + Sync>(&mut self, twine: T) -> Result<(), Box<dyn Error>> {
    match twine.into() {
      AnyTwine::Strand(strand) => {
        self.strands.entry(strand.cid()).or_insert(StrandMap::new(strand));
      },
      AnyTwine::Tixel(tixel) => {
        if let None = self.tixels.get(&tixel.cid()) {
          let strand_cid = tixel.strand_cid();
          if let Some(strand) = self.strands.get_mut(&strand_cid) {
            strand.by_index.insert(tixel.index(), tixel.clone());
            self.tixels.insert(tixel.cid(), tixel);
          } else {
            return Err("Strand not found".into());
          }
        }
      },
    }
    Ok(())
  }

  async fn save_many<T: Into<AnyTwine> + Send + Sync>(&mut self, twines: Vec<T>) -> Result<(), Box<dyn Error>> {
    for twine in twines {
      self.save(twine).await?;
    }
    Ok(())
  }

  async fn delete<C: AsCid + Send + Sync>(&mut self, cid: C) -> Result<(), Box<dyn Error>> {
    let cid = cid.as_cid();
    if let Some(s) = self.strands.remove(&cid) {
      for tixel in s.by_index.values() {
        self.tixels.remove(&tixel.cid());
      }
    } else if let Some(tixel) = self.tixels.remove(&cid) {
      if let Some(strand) = self.strands.get_mut(&tixel.strand_cid()) {
        strand.by_index.remove(&tixel.index());
      }
    }
    Ok(())
  }
}


#[cfg(test)]
mod test {
  use super::*;
  use crate::prelude::*;
  use crate::test::*;

  #[tokio::test]
  async fn test_memory_store() {
    let mut store = MemoryStore::new();
    let strand = Strand::from_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_dag_json(TIXELJSON).unwrap();
    store.save(strand.clone()).await.unwrap();
    store.save(tixel.clone()).await.unwrap();
    let strand2 = store.resolve_cid(strand.cid()).await.unwrap();
    let tixel2 = store.resolve_cid(tixel.cid()).await.unwrap();
    assert_eq!(strand, strand2);
    assert_eq!(tixel, tixel2);
    store.delete(strand.cid()).await.unwrap();
    store.delete(tixel.cid()).await.unwrap();
    assert!(store.resolve_cid(strand.cid()).await.is_err());
    assert!(store.resolve_cid(tixel.cid()).await.is_err());
  }

  #[tokio::test]
  async fn test_memory_store_save_many() {
    let mut store = MemoryStore::new();
    let strand = Strand::from_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_dag_json(TIXELJSON).unwrap();
    let things: Vec<AnyTwine> = vec![strand.clone().into(), tixel.clone().into()];
    store.save_many(things).await.unwrap();
    let strand2 = store.resolve_cid(strand.cid()).await.unwrap();
    let tixel2 = store.resolve_cid(tixel.cid()).await.unwrap();
    assert_eq!(strand, strand2);
    assert_eq!(tixel, tixel2);
    store.delete(strand.cid()).await.unwrap();
    store.delete(tixel.cid()).await.unwrap();
    assert!(store.resolve_cid(strand.cid()).await.is_err());
    assert!(store.resolve_cid(tixel.cid()).await.is_err());
  }

  #[tokio::test]
  async fn test_memory_store_strand_list() {
    let mut store = MemoryStore::new();
    let strand = Strand::from_dag_json(STRANDJSON).unwrap();
    store.save(strand.clone()).await.unwrap();
    let mut stream = store.strands().await.unwrap();
    use futures::stream::TryStreamExt;
    let strand2 = stream.try_next().await.unwrap().unwrap();
    assert_eq!(strand, *strand2);
  }
}
