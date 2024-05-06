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
// use async_std::sync::RwLock;
use std::sync::RwLock;

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
  tixels: Arc<RwLock<HashMap<Cid, Arc<Tixel>>>>,
  strands: Arc<RwLock<HashMap<Cid, StrandMap>>>,
}

impl MemoryStore {
  pub fn new() -> Self {
    Self {
      tixels: Arc::new(RwLock::new(HashMap::new())),
      strands: Arc::new(RwLock::new(HashMap::new())),
    }
  }
}

#[async_trait]
impl Resolver for MemoryStore {
  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    let iter = self.strands.read().unwrap()
      .values()
      .map(|s| Ok(s.strand.clone()))
      .collect::<Vec<Result<Arc<Strand>, ResolutionError>>>();

    use futures::stream::StreamExt;
    let stream = futures::stream::iter(iter);
    Ok(stream.boxed())
  }

  async fn resolve_cid<'a, C: AsCid + Send>(&'a self, cid: C) -> Result<AnyTwine, ResolutionError> {
    let cid = cid.as_cid();
    if let Some(tixel) = self.tixels.read().unwrap().get(&cid) {
      Ok(AnyTwine::Tixel(tixel.clone()))
    } else if let Some(strand) = self.strands.read().unwrap().get(&cid) {
      Ok(AnyTwine::Strand(strand.strand.clone()))
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
    let cid = strand.as_cid();
    if let Some(s) = self.strands.read().unwrap().get(&cid) {
      Ok(s.strand.clone())
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: u64) -> Result<Twine, ResolutionError> {
    let cid = strand.as_cid();
    if let Some(s) = self.strands.read().unwrap().get(&cid) {
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
    if let Some(s) = self.strands.read().unwrap().get(&cid) {
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
    let strand = self.resolve_strand(range.strand).await?;
    use futures::stream::StreamExt;
    if let Some(entry) = self.strands.read().unwrap().get(&range.strand) {
      let list = (range.lower..=range.upper)
        .map(|i| entry.by_index.get(&i).cloned())
        .map(|t| t.ok_or(ResolutionError::NotFound))
        .map(|t| Ok(Twine::try_new_from_shared(strand.clone(), t?)?))
        .rev()
        .collect::<Vec<Result<Twine, ResolutionError>>>();
      let stream = futures::stream::iter(list);
      Ok(stream.boxed())
    } else {
      Err(ResolutionError::NotFound)
    }
  }
}

#[async_trait]
impl Store for MemoryStore {
  async fn save<T: Into<AnyTwine> + Send + Sync>(&self, twine: T) -> Result<(), Box<dyn Error>> {
    match twine.into() {
      AnyTwine::Strand(strand) => {
        self.strands.write().unwrap().entry(strand.cid()).or_insert(StrandMap::new(strand));
      },
      AnyTwine::Tixel(tixel) => {
        let mut tixels = self.tixels.write().unwrap();
        if let None = { tixels.get(&tixel.cid()) } {
          let strand_cid = tixel.strand_cid();
          if let Some(strand) = self.strands.write().unwrap().get_mut(&strand_cid) {
            strand.by_index.insert(tixel.index(), tixel.clone());
            tixels.insert(tixel.cid(), tixel);
          } else {
            return Err("Strand not found".into());
          }
        }
      },
    }
    Ok(())
  }

  async fn save_many<I: Into<AnyTwine> + Send + Sync, S: Iterator<Item = I> + Send + Sync, T: IntoIterator<Item = I, IntoIter = S> + Send + Sync>(&self, twines: T) -> Result<(), Box<dyn Error>> {
    self.save_stream(futures::stream::iter(twines.into_iter())).await
  }

  async fn save_stream<I: Into<AnyTwine> + Send + Sync, T: Stream<Item = I> + Send + Sync>(&self, twines: T) -> Result<(), Box<dyn Error>> {
    use futures::stream::{StreamExt, TryStreamExt};
    twines
      .then(|twine| async {
        self.save(twine).await?;
        Ok::<(), Box<dyn Error>>(())
      })
      .try_all(|_| async { true })
      .await?;
    Ok(())
  }

  async fn delete<C: AsCid + Send + Sync>(&self, cid: C) -> Result<(), Box<dyn Error>> {
    let cid = cid.as_cid();
    if let Some(s) = self.strands.write().unwrap().remove(&cid) {
      for tixel in s.by_index.values() {
        self.tixels.write().unwrap().remove(&tixel.cid());
      }
    } else if let Some(tixel) = self.tixels.write().unwrap().remove(&cid) {
      if let Some(strand) = self.strands.write().unwrap().get_mut(&tixel.strand_cid()) {
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
    let store = MemoryStore::new();
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
    let store = MemoryStore::new();
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
    let store = MemoryStore::new();
    let strand = Strand::from_dag_json(STRANDJSON).unwrap();
    store.save(strand.clone()).await.unwrap();
    let mut stream = store.strands().await.unwrap();
    use futures::stream::TryStreamExt;
    let strand2 = stream.try_next().await.unwrap().unwrap();
    assert_eq!(strand, *strand2);
  }
}
