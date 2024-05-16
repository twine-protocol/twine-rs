use crate::resolver::{BaseResolver, Resolver};
use crate::errors::ResolutionError;
use super::Store;
use crate::twine::Tixel;
use crate::Cid;
use crate::twine::Strand;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::pin::Pin;
use futures::stream::Stream;
use async_trait::async_trait;
use futures::stream::StreamExt;
use quick_cache::Equivalent;
use crate::resolver::RangeQuery;
use quick_cache::sync::Cache;

// Cid, index
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct CacheKey(Cid, u64);

impl Equivalent<Cid> for CacheKey {
  fn equivalent(&self, other: &Cid) -> bool {
    self.0 == *other
  }
}

impl Equivalent<u64> for CacheKey {
  fn equivalent(&self, other: &u64) -> bool {
    self.1 == *other
  }
}

impl Equivalent<CacheKey> for Cid {
  fn equivalent(&self, other: &CacheKey) -> bool {
    *self == other.0
  }
}

impl Equivalent<CacheKey> for u64 {
  fn equivalent(&self, other: &CacheKey) -> bool {
    *self == other.1
  }
}

type StrandCache = HashMap<Cid, (Option<Arc<Strand>>, Cache<CacheKey, Arc<Tixel>>)>;

#[derive(Debug)]
pub struct MemoryCache<T: Resolver> {
  strands: Arc<RwLock<StrandCache>>,
  resolver: T,
  size_per_strand: usize,
}

impl<T: Resolver> MemoryCache<T> {
  pub fn new(resolver: T) -> Self {
    Self {
      strands: Arc::new(RwLock::new(HashMap::new())),
      resolver,
      size_per_strand: 1000,
    }
  }

  pub fn with_size_per_strand(mut self, size_per_strand: usize) -> Self {
    self.size_per_strand = size_per_strand;
    self
  }

  fn cache_tixel(&self, tixel: Arc<Tixel>) -> Arc<Tixel> {
    let strand_cid = tixel.strand_cid();
    let mut store = self.strands.write().unwrap();
    let cache = store.entry(strand_cid).or_insert_with(|| (None, Cache::new(self.size_per_strand)));
    let _ = cache.1.get_or_insert_with(&CacheKey(tixel.cid(), tixel.index()), || Ok::<_, ResolutionError>(tixel.clone()));
    dbg!(cache.1.len(), cache.1.get(&tixel.index()));
    tixel
  }

  fn cache_strand(&self, strand: Arc<Strand>) -> Arc<Strand> {
    let strand_cid = strand.cid();
    let mut store = self.strands.write().unwrap();
    let entry = store.entry(strand_cid).or_insert_with(|| (None, Cache::new(self.size_per_strand)));
    if entry.0.is_none() {
      entry.0 = Some(strand.clone());
    }
    strand
  }
}

#[async_trait]
impl<T: Resolver> BaseResolver for MemoryCache<T> {
  async fn strands(&self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + '_ + Send>>, ResolutionError> {
    self.resolver.strands().await
  }

  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError>{
    let has = match self.strands.read().unwrap().get(strand) {
      Some((_, cache)) => {
        match cache.get(&index) {
          Some(_) => true,
          None => false,
        }
      },
      None => false,
    };
    if has {
      Ok(true)
    } else {
      self.resolver.has_index(strand, index).await
    }
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError>{
    let has = match self.strands.read().unwrap().get(strand) {
      Some((_, cache)) => {
        match cache.get(cid) {
          Some(_) => true,
          None => false,
        }
      },
      None => false,
    };
    if has {
      Ok(true)
    } else {
      self.resolver.has_twine(strand, cid).await
    }
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError>{
    let has = self.strands.read().unwrap().contains_key(cid);
    if has {
      Ok(true)
    } else {
      self.resolver.has_strand(cid).await
    }
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError>{
    // won't check cache
    let tixel = self.resolver.fetch_latest(strand).await?;
    Ok(self.cache_tixel(tixel.clone()))
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError>{
    let maybe_tixel = self.strands.read().unwrap()
      .get(strand)
      .map(|(_, cache)| cache.get(&index))
      .flatten();
    if let Some(tixel) = maybe_tixel {
      Ok(tixel.clone())
    } else {
      let tixel = self.resolver.fetch_index(strand, index).await?;
      Ok(self.cache_tixel(tixel))
    }
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError>{
    let maybe_tixel = self.strands.read().unwrap()
      .get(strand)
      .map(|(_, cache)| cache.get(tixel))
      .flatten();
    if let Some(tixel) = maybe_tixel {
      Ok(tixel.clone())
    } else {
      let tixel = self.resolver.fetch_tixel(strand, tixel).await?;
      Ok(self.cache_tixel(tixel))
    }
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError>{
    let maybe_strand = self.strands.read().unwrap()
      .get(strand)
      .map(|(strand, _)| strand.clone())
      .flatten();
    if let Some(strand) = maybe_strand {
      Ok(strand)
    } else {
      let strand = self.resolver.fetch_strand(strand).await?;
      Ok(self.cache_strand(strand))
    }
  }

  async fn range_stream<'a>(&'a self, range: RangeQuery) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError>{
    let stream = self.resolver.range_stream(range).await?;
    Ok(
      stream
        .map(|tixel| {
          let tixel = tixel?;
          Ok(self.cache_tixel(tixel))
        })
        .boxed()
    )
  }
}

impl<S: Store> Deref for MemoryCache<S> {
  type Target = S;

  fn deref(&self) -> &Self::Target {
    &self.resolver
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_cache_key() {
    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    struct MyKey(u64, u8);
    impl Equivalent<u64> for MyKey {
      fn equivalent(&self, other: &u64) -> bool {
        self.0 == *other
      }
    }
    impl Equivalent<MyKey> for u64 {
      fn equivalent(&self, other: &MyKey) -> bool {
        *self == other.0
      }
    }
    impl Equivalent<u8> for MyKey {
      fn equivalent(&self, other: &u8) -> bool {
        self.1 == *other
      }
    }
    impl Equivalent<MyKey> for u8 {
      fn equivalent(&self, other: &MyKey) -> bool {
        *self == other.1
      }
    }

    let cache = Cache::new(10);
    let key = MyKey(1, 2);
    let value = "hello".to_string();
    cache.insert(key.clone(), value.clone());
    assert_eq!(cache.get(&1u64), Some(value));
  }
}
