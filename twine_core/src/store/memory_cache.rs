use crate::errors::ResolutionError;
use crate::resolver::{unchecked_base, AbsoluteRange, Resolver};
use crate::twine::Strand;
use crate::twine::Tixel;
use crate::Cid;
use async_trait::async_trait;
use futures::stream::StreamExt;
use quick_cache::sync::Cache;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, RwLock};

type TixelCache = Cache<Cid, Tixel>;
type StrandCache = HashMap<Cid, (Option<Strand>, Cache<u64, Cid>)>;

#[derive(Debug)]
pub struct MemoryCache<T: Resolver> {
  strands: Arc<RwLock<StrandCache>>,
  tixels: TixelCache,
  resolver: T,
  cache_size: usize,
}

impl<T: Resolver> MemoryCache<T> {
  pub fn new(resolver: T) -> Self {
    Self {
      strands: Arc::new(RwLock::new(HashMap::new())),
      tixels: Cache::new(1000),
      resolver,
      cache_size: 1000,
    }
  }

  pub fn with_cache_size(mut self, cache_size: usize) -> Self {
    self.cache_size = cache_size;
    self
  }

  fn cache_tixel(&self, tixel: Tixel) -> Tixel {
    let strand_cid = tixel.strand_cid();
    let mut store = self.strands.write().unwrap();
    let cache = store
      .entry(strand_cid)
      .or_insert_with(|| (None, Cache::new(self.cache_size)));
    let _ = cache
      .1
      .get_or_insert_with(&tixel.index(), || Ok::<_, ResolutionError>(tixel.cid()));
    self.tixels.insert(tixel.cid(), tixel.clone());
    tixel
  }

  fn cache_strand(&self, strand: Strand) -> Strand {
    let strand_cid = strand.cid();
    let mut store = self.strands.write().unwrap();
    let entry = store
      .entry(strand_cid)
      .or_insert_with(|| (None, Cache::new(self.cache_size)));
    if entry.0.is_none() {
      entry.0 = Some(strand.clone());
    }
    strand
  }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T: Resolver> unchecked_base::BaseResolver for MemoryCache<T> {
  async fn fetch_strands(
    &self,
  ) -> Result<unchecked_base::TwineStream<'_, Strand>, ResolutionError> {
    self.resolver.strands().await.and_then(|stream| {
      let s = stream.map(|strand| {
        let strand = strand?;
        Ok(self.cache_strand(strand))
      });

      #[cfg(target_arch = "wasm32")]
      {
        Ok(s.boxed_local())
      }
      #[cfg(not(target_arch = "wasm32"))]
      {
        Ok(s.boxed())
      }
    })
  }

  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    let has = match self.strands.read().unwrap().get(strand) {
      Some((_, cache)) => match cache.get(&index) {
        Some(_) => true,
        None => false,
      },
      None => false,
    };
    if has {
      Ok(true)
    } else {
      self.resolver.has_index(strand, index).await
    }
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    let has = match self.strands.read().unwrap().get(strand) {
      Some((_, _cache)) => match self.tixels.get(cid) {
        Some(_) => true,
        None => false,
      },
      None => false,
    };
    if has {
      Ok(true)
    } else {
      self.resolver.has_twine(strand, cid).await
    }
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    let has = self.strands.read().unwrap().contains_key(cid);
    if has {
      Ok(true)
    } else {
      self.resolver.has_strand(cid).await
    }
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
    // won't check cache
    let tixel = self.resolver.fetch_latest(strand).await?;
    Ok(self.cache_tixel(tixel.clone()))
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
    let maybe_cid = self
      .strands
      .read()
      .unwrap()
      .get(strand)
      .map(|(_, cache)| cache.get(&index))
      .flatten();
    if let Some(tixel) = maybe_cid.map(|cid| self.tixels.get(&cid)).flatten() {
      Ok(tixel.clone())
    } else {
      let tixel = self.resolver.fetch_index(strand, index).await?;
      Ok(self.cache_tixel(tixel))
    }
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
    let maybe_tixel = self.tixels.get(tixel);
    if let Some(tixel) = maybe_tixel {
      Ok(tixel.clone())
    } else {
      let tixel = self.resolver.fetch_tixel(strand, tixel).await?;
      Ok(self.cache_tixel(tixel))
    }
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
    let maybe_strand = self
      .strands
      .read()
      .unwrap()
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

  async fn range_stream<'a>(
    &'a self,
    range: AbsoluteRange,
  ) -> Result<unchecked_base::TwineStream<'a, Tixel>, ResolutionError> {
    let stream = self.resolver.range_stream(range).await?;
    let s = stream.map(|tixel| {
      let tixel = tixel?;
      Ok(self.cache_tixel(tixel))
    });

    #[cfg(target_arch = "wasm32")]
    {
      Ok(s.boxed_local())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      Ok(s.boxed())
    }
  }
}

impl<R: Resolver> Resolver for MemoryCache<R> {}

impl<R: Resolver> Deref for MemoryCache<R> {
  type Target = R;

  fn deref(&self) -> &Self::Target {
    &self.resolver
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::{test::*, twine::TwineBlock};

  #[derive(Debug, Clone)]
  struct DummyResolver {
    pub strand_hits: Arc<RwLock<HashMap<Cid, u32>>>,
    pub tixel_hits: Arc<RwLock<HashMap<Cid, u32>>>,
  }

  #[async_trait]
  impl unchecked_base::BaseResolver for DummyResolver {
    async fn fetch_strands<'a>(
      &'a self,
    ) -> Result<unchecked_base::TwineStream<'a, Strand>, ResolutionError> {
      let strand = Strand::from_tagged_dag_json(STRANDJSON)?;
      let s = vec![strand];
      let stream = futures::stream::iter(s.into_iter().map(Ok));
      Ok(stream.boxed())
    }

    async fn has_index(&self, _strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
      let tixel = Tixel::from_tagged_dag_json(TIXELJSON)?;
      if tixel.index() == index {
        *self
          .tixel_hits
          .write()
          .unwrap()
          .entry(tixel.cid())
          .or_insert(0) += 1;
        Ok(true)
      } else {
        Ok(false)
      }
    }

    async fn has_twine(&self, _strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
      let tixel = Tixel::from_tagged_dag_json(TIXELJSON)?;
      if tixel.cid() == *cid {
        *self
          .tixel_hits
          .write()
          .unwrap()
          .entry(tixel.cid())
          .or_insert(0) += 1;
        Ok(true)
      } else {
        Ok(false)
      }
    }

    async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
      let strand = Arc::new(Strand::from_tagged_dag_json(STRANDJSON)?);
      if strand.cid() == *cid {
        *self
          .strand_hits
          .write()
          .unwrap()
          .entry(strand.cid())
          .or_insert(0) += 1;
        Ok(true)
      } else {
        Ok(false)
      }
    }

    async fn fetch_latest(&self, _strand: &Cid) -> Result<Tixel, ResolutionError> {
      let tixel = Tixel::from_tagged_dag_json(TIXELJSON)?;
      Ok(tixel)
    }

    async fn fetch_index(&self, _strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
      let tixel = Tixel::from_tagged_dag_json(TIXELJSON)?;
      if tixel.index() != index {
        return Err(ResolutionError::NotFound);
      }
      *self
        .tixel_hits
        .write()
        .unwrap()
        .entry(tixel.cid())
        .or_insert(0) += 1;
      Ok(tixel)
    }

    async fn fetch_tixel(&self, _strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
      let tix = Tixel::from_tagged_dag_json(TIXELJSON)?;
      if tix.cid() != *tixel {
        return Err(ResolutionError::NotFound);
      }
      *self
        .tixel_hits
        .write()
        .unwrap()
        .entry(tixel.clone())
        .or_insert(0) += 1;
      Ok(tix)
    }

    async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
      let s = Strand::from_tagged_dag_json(STRANDJSON)?;
      if s.cid() != *strand {
        return Err(ResolutionError::NotFound);
      }
      *self
        .strand_hits
        .write()
        .unwrap()
        .entry(*strand)
        .or_insert(0) += 1;
      Ok(s)
    }

    async fn range_stream<'a>(
      &'a self,
      range: AbsoluteRange,
    ) -> Result<unchecked_base::TwineStream<'a, Tixel>, ResolutionError> {
      let tixel = Tixel::from_tagged_dag_json(TIXELJSON)?;
      if *range.strand_cid() != tixel.strand_cid() {
        return Err(ResolutionError::NotFound);
      }
      let stream = futures::stream::iter(vec![tixel].into_iter().map(Ok));
      Ok(stream.boxed())
    }
  }

  impl Resolver for DummyResolver {}

  #[tokio::test]
  async fn test_cache() {
    let resolver = DummyResolver {
      strand_hits: Arc::new(RwLock::new(HashMap::new())),
      tixel_hits: Arc::new(RwLock::new(HashMap::new())),
    };
    let cache = MemoryCache::new(resolver);
    let strand = Strand::from_tagged_dag_json(STRANDJSON).unwrap();
    let tixel = Tixel::from_tagged_dag_json(TIXELJSON).unwrap();
    let strand_cid = strand.cid();
    let tixel_cid = tixel.cid();

    let _ = cache.resolve_strand(&strand_cid).await.unwrap().unpack();
    let _ = cache
      .resolve_stitch(&strand_cid, &tixel_cid)
      .await
      .unwrap()
      .unpack();

    assert_eq!(cache.strand_hits.read().unwrap().get(&strand_cid), Some(&1));
    assert_eq!(cache.tixel_hits.read().unwrap().get(&tixel_cid), Some(&1));

    let _ = cache.resolve_strand(&strand_cid).await.unwrap().unpack();
    let _ = cache
      .resolve_stitch(&strand_cid, &tixel_cid)
      .await
      .unwrap()
      .unpack();

    assert_eq!(cache.strand_hits.read().unwrap().get(&strand_cid), Some(&1));
    assert_eq!(cache.tixel_hits.read().unwrap().get(&tixel_cid), Some(&1));

    let _ = cache.resolve_strand(&strand_cid).await.unwrap().unpack();
    let _ = cache
      .resolve_index(&strand_cid, tixel.index())
      .await
      .unwrap()
      .unpack();

    assert_eq!(cache.strand_hits.read().unwrap().get(&strand_cid), Some(&1));
    assert_eq!(cache.tixel_hits.read().unwrap().get(&tixel_cid), Some(&1));

    cache
      .resolve_range((strand_cid, 0..1))
      .await
      .unwrap()
      .collect::<Vec<_>>()
      .await;

    assert_eq!(cache.strand_hits.read().unwrap().get(&strand_cid), Some(&1));
    assert_eq!(cache.tixel_hits.read().unwrap().get(&tixel_cid), Some(&1));
  }
}
