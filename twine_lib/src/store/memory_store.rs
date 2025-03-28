use super::Store;
use crate::as_cid::AsCid;
use crate::errors::{ResolutionError, StoreError};
use crate::resolver::{unchecked_base, MaybeSend};
use crate::resolver::{unchecked_base::BaseResolver, AbsoluteRange, Resolver};
use crate::twine::AnyTwine;
use crate::twine::{Strand, Tixel};
use crate::Cid;
use async_trait::async_trait;
use futures::Stream;
use std::collections::BTreeMap;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
struct StrandMap {
  strand: Strand,
  by_index: BTreeMap<u64, Tixel>,
}

impl StrandMap {
  fn new(strand: Strand) -> Self {
    Self {
      strand,
      by_index: BTreeMap::new(),
    }
  }
}

/// A simple in-memory store
///
/// Clones will retain the data, since the data is
/// stored inside of [`Arc`]s and [`RwLock`]s.
#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
  tixels: Arc<RwLock<HashMap<Cid, Tixel>>>,
  strands: Arc<RwLock<HashMap<Cid, StrandMap>>>,
}

impl MemoryStore {
  /// Create an empty store
  pub fn new() -> Self {
    Self {
      tixels: Arc::new(RwLock::new(HashMap::new())),
      strands: Arc::new(RwLock::new(HashMap::new())),
    }
  }

  /// Save twine data synchronously
  pub fn save_sync(&self, twine: AnyTwine) -> Result<(), StoreError> {
    match twine {
      AnyTwine::Strand(strand) => {
        self
          .strands
          .write()
          .unwrap()
          .entry(strand.cid())
          .or_insert(StrandMap::new(strand));
      }
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
      }
    }
    Ok(())
  }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl BaseResolver for MemoryStore {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    Ok(
      self
        .strands
        .read()
        .unwrap()
        .get(strand)
        .map_or(false, |s| s.by_index.contains_key(&index)),
    )
  }

  async fn has_twine(&self, _strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(self.tixels.read().unwrap().contains_key(cid))
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(self.strands.read().unwrap().contains_key(cid))
  }

  async fn fetch_strands<'a>(
    &'a self,
  ) -> Result<unchecked_base::TwineStream<'a, Strand>, ResolutionError> {
    let iter = self
      .strands
      .read()
      .unwrap()
      .values()
      .map(|s| Ok(s.strand.clone()))
      .collect::<Vec<Result<Strand, ResolutionError>>>();

    use futures::stream::StreamExt;
    let stream = futures::stream::iter(iter);
    Ok(stream.boxed())
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
    let cid = strand.as_cid();
    if let Some(s) = self.strands.read().unwrap().get(&cid) {
      Ok(s.strand.clone())
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn fetch_tixel(&self, _strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
    let cid = tixel.as_cid();
    if let Some(t) = self.tixels.read().unwrap().get(&cid) {
      Ok(t.clone())
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
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

  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
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

  async fn range_stream<'a>(
    &'a self,
    range: AbsoluteRange,
  ) -> Result<unchecked_base::TwineStream<'a, Tixel>, ResolutionError> {
    use futures::stream::StreamExt;
    if let Some(entry) = self.strands.read().unwrap().get(range.strand_cid()) {
      let list = range
        .into_iter()
        .map(|q| entry.by_index.get(&(q.unwrap_index() as u64)).cloned())
        .map(|t| t.ok_or(ResolutionError::NotFound))
        .collect::<Vec<Result<Tixel, ResolutionError>>>();
      let stream = futures::stream::iter(list);
      Ok(stream.boxed())
    } else {
      Err(ResolutionError::NotFound)
    }
  }
}

impl Resolver for MemoryStore {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Store for MemoryStore {
  async fn save<T: Into<AnyTwine> + MaybeSend>(&self, twine: T) -> Result<(), StoreError> {
    self.save_sync(twine.into())
  }

  async fn save_many<
    I: Into<AnyTwine> + MaybeSend,
    S: Iterator<Item = I> + MaybeSend,
    T: IntoIterator<Item = I, IntoIter = S> + MaybeSend,
  >(
    &self,
    twines: T,
  ) -> Result<(), StoreError> {
    self
      .save_stream(futures::stream::iter(twines.into_iter()))
      .await
  }

  async fn save_stream<I: Into<AnyTwine> + MaybeSend, T: Stream<Item = I> + MaybeSend>(
    &self,
    twines: T,
  ) -> Result<(), StoreError> {
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

  async fn delete<C: AsCid + MaybeSend>(&self, cid: C) -> Result<(), StoreError> {
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
  use crate::resolver::Resolver;
  use crate::test::*;
  use crate::twine::*;

  #[tokio::test]
  async fn test_memory_store() {
    let store = MemoryStore::new();
    let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();
    store.save(strand.clone()).await.unwrap();
    store.save(tixel.clone()).await.unwrap();
    let strand2 = store.fetch_strand(&strand.cid()).await.unwrap();
    let tixel2 = store
      .fetch_tixel(&strand2.cid(), &tixel.cid())
      .await
      .unwrap();
    assert_eq!(strand, strand2);
    assert_eq!(tixel, tixel2);
    store.delete(strand.cid()).await.unwrap();
    store.delete(tixel.cid()).await.unwrap();
    assert!(store.fetch_strand(&strand.cid()).await.is_err());
    assert!(store
      .fetch_tixel(&strand.cid(), &tixel.cid())
      .await
      .is_err());
  }

  #[tokio::test]
  async fn test_memory_store_save_many() {
    let store = MemoryStore::new();
    let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();
    let things: Vec<AnyTwine> = vec![strand.clone().into(), tixel.clone().into()];
    store.save_many(things).await.unwrap();
    let strand2 = store.fetch_strand(&strand.cid()).await.unwrap();
    let tixel2 = store
      .fetch_tixel(&strand2.cid(), &tixel.cid())
      .await
      .unwrap();
    assert_eq!(strand, strand2);
    assert_eq!(tixel, tixel2);
    store.delete(strand.cid()).await.unwrap();
    store.delete(tixel.cid()).await.unwrap();
    assert!(store.fetch_strand(&strand.cid()).await.is_err());
    assert!(store
      .fetch_tixel(&strand.cid(), &tixel.cid())
      .await
      .is_err());
  }

  #[tokio::test]
  async fn test_memory_store_strand_list() {
    let store = MemoryStore::new();
    let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
    store.save(strand.clone()).await.unwrap();
    let mut stream = store.strands().await.unwrap();
    use futures::stream::TryStreamExt;
    let strand2 = stream.try_next().await.unwrap().unwrap();
    assert_eq!(strand, strand2);
  }

  #[tokio::test]
  async fn test_resolver() {
    let store = MemoryStore::new();
    let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();
    store.save(strand.clone()).await.unwrap();
    store.save(tixel.clone()).await.unwrap();
    let latest = store.resolve(strand).await.unwrap();
    assert_eq!(latest, tixel);
  }
}
