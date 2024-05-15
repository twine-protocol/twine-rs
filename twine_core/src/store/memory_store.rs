use std::collections::BTreeMap;
use std::pin::Pin;
use std::{collections::HashMap, sync::Arc};
use futures::Stream;
use libipld::Cid;
use crate::errors::{ResolutionError, StoreError};
use crate::resolver::{BaseResolver, RangeQuery, Resolver};
use crate::twine::{Strand, Tixel};
use super::Store;
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
impl BaseResolver for MemoryStore {
  async fn has_twine(&self, _strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(self.tixels.read().unwrap().contains_key(cid))
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(self.strands.read().unwrap().contains_key(cid))
  }

  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    let iter = self.strands.read().unwrap()
      .values()
      .map(|s| Ok(s.strand.clone()))
      .collect::<Vec<Result<Arc<Strand>, ResolutionError>>>();

    use futures::stream::StreamExt;
    let stream = futures::stream::iter(iter);
    Ok(stream.boxed())
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    let cid = strand.as_cid();
    if let Some(s) = self.strands.read().unwrap().get(&cid) {
      Ok(s.strand.clone())
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn fetch_tixel(&self, _strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let cid = tixel.as_cid();
    if let Some(t) = self.tixels.read().unwrap().get(&cid) {
      Ok(t.clone())
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    let cid = strand.as_cid();
    if let Some(s) = self.strands.read().unwrap().get(&cid) {
      if let Some(tixel) = s.by_index.get(&index) {
        Ok(tixel.clone())
      } else {
        Err(ResolutionError::NotFound)
      }
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let cid = strand.as_cid();
    if let Some(s) = self.strands.read().unwrap().get(&cid) {
      if let Some((_index, tixel)) = s.by_index.last_key_value() {
        Ok(tixel.clone())
      } else {
        Err(ResolutionError::NotFound)
      }
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn range_stream<'a>(&'a self, range: RangeQuery) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    let range = range.try_to_absolute(self).await?;
    use futures::stream::StreamExt;
    if let Some(entry) = self.strands.read().unwrap().get(&range.strand) {
      let list = (range.lower..=range.upper)
        .map(|i| entry.by_index.get(&i).cloned())
        .map(|t| t.ok_or(ResolutionError::NotFound))
        .rev()
        .collect::<Vec<Result<Arc<Tixel>, ResolutionError>>>();
      let stream = futures::stream::iter(list);
      Ok(stream.boxed())
    } else {
      Err(ResolutionError::NotFound)
    }
  }
}

#[async_trait]
impl Store for MemoryStore {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
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
            return Err(StoreError::Saving("Strand not found".into()));
          }
        }
      },
    }
    Ok(())
  }

  async fn save_many<I: Into<AnyTwine> + Send, S: Iterator<Item = I> + Send, T: IntoIterator<Item = I, IntoIter = S> + Send>(&self, twines: T) -> Result<(), StoreError> {
    self.save_stream(futures::stream::iter(twines.into_iter())).await
  }

  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send>(&self, twines: T) -> Result<(), StoreError> {
    use futures::stream::{StreamExt, TryStreamExt};
    twines
      .then(|twine| async {
        self.save(twine).await?;
        Ok::<(), StoreError>(())
      })
      .try_all(|_| async { true })
      .await?;
    Ok(())
  }

  async fn delete<C: AsCid + Send>(&self, cid: C) -> Result<(), StoreError> {
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
  use crate::twine::*;
  use crate::test::*;

  #[tokio::test]
  async fn test_memory_store() {
    let store = MemoryStore::new();
    let strand = Strand::from_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_dag_json(TIXELJSON).unwrap();
    store.save(strand.clone()).await.unwrap();
    store.save(tixel.clone()).await.unwrap();
    let strand2 = store.fetch_strand(&strand.cid()).await.unwrap();
    let tixel2 = store.fetch_tixel(&strand2.cid(), &tixel.cid()).await.unwrap();
    assert_eq!(strand, *strand2);
    assert_eq!(tixel, *tixel2);
    store.delete(strand.cid()).await.unwrap();
    store.delete(tixel.cid()).await.unwrap();
    assert!(store.fetch_strand(&strand.cid()).await.is_err());
    assert!(store.fetch_tixel(&strand.cid(), &tixel.cid()).await.is_err());
  }

  #[tokio::test]
  async fn test_memory_store_save_many() {
    let store = MemoryStore::new();
    let strand = Strand::from_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_dag_json(TIXELJSON).unwrap();
    let things: Vec<AnyTwine> = vec![strand.clone().into(), tixel.clone().into()];
    store.save_many(things).await.unwrap();
    let strand2 = store.fetch_strand(&strand.cid()).await.unwrap();
    let tixel2 = store.fetch_tixel(&strand2.cid(), &tixel.cid()).await.unwrap();
    assert_eq!(strand, *strand2);
    assert_eq!(tixel, *tixel2);
    store.delete(strand.cid()).await.unwrap();
    store.delete(tixel.cid()).await.unwrap();
    assert!(store.fetch_strand(&strand.cid()).await.is_err());
    assert!(store.fetch_tixel(&strand.cid(), &tixel.cid()).await.is_err());
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

  #[tokio::test]
  async fn test_resolver() {
    let store = MemoryStore::new();
    let strand = Strand::from_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_dag_json(TIXELJSON).unwrap();
    store.save(strand.clone()).await.unwrap();
    store.save(tixel.clone()).await.unwrap();
    let latest = store.resolve(strand).await.unwrap();
    assert_eq!(latest, tixel);
  }
}
