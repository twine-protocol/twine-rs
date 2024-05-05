use crate::resolver::{Resolver, ResolutionError};
use super::memory_store::MemoryStore;
use super::Store;
use crate::twine::{Twine, AnyTwine};
use crate::as_cid::AsCid;
use crate::twine::Strand;
use std::sync::Arc;
use std::pin::Pin;
use futures::lock::Mutex;
use futures::stream::Stream;
use async_trait::async_trait;
use futures::stream::{TryStreamExt, StreamExt};
use crate::resolver::RangeQuery;

#[derive(Debug, Clone)]
pub struct MemoryCache<T: Resolver> {
  cache: MemoryStore,
  resolver: T,
}

impl<T: Resolver> MemoryCache<T> {
  pub fn new(resolver: T) -> Self {
    Self {
      cache: MemoryStore::new(),
      resolver,
    }
  }
}

#[async_trait]
impl<T: Resolver> Resolver for MemoryCache<T> {
  async fn strands(&self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + '_ + Send>>, ResolutionError> {
    Ok(
      self.resolver.strands().await?
        .and_then(|strand| async {
          let _ = self.cache.save(strand.clone()).await;
          Ok(strand)
        })
        .boxed()
    )
  }

  async fn resolve_cid<C: AsCid + Send>(&self, cid: C) -> Result<AnyTwine, ResolutionError> {
    let cid = cid.as_cid();
    let res = {
      self.cache.resolve_cid(cid).await
    };
    if res.is_ok() {
      res
    } else {
      let twine = self.resolver.resolve_cid(cid).await?;
      let _ = self.cache.save(twine.clone()).await;
      Ok(twine)
    }
  }

  async fn resolve_latest<C: AsCid + Send>(&self, cid: C) -> Result<Twine, ResolutionError> {
    let cid = cid.as_cid();
    let twine = self.resolver.resolve_latest(cid).await?;
    let _ = self.cache.save(twine.clone()).await;
    Ok(twine)
  }

  async fn resolve_index<C: AsCid + Send>(&self, cid: C, index: u64) -> Result<Twine, ResolutionError> {
    let cid = cid.as_cid();
    let res = {
      self.cache.resolve_index(cid, index).await
    };
    if res.is_ok() {
      res
    } else {
      let twine = self.resolver.resolve_index(cid, index).await?;
      let _ = self.cache.save(twine.clone()).await;
      Ok(twine)
    }
  }

  async fn resolve_range<'a, R: Into<RangeQuery> + Send>(&'a self, range: R) -> Result<Pin<Box<dyn Stream<Item = Result<Twine, ResolutionError>> + Send + 'a>>, ResolutionError> {
    Ok(
      // TODO: make this check cache first
      self.resolver.resolve_range(range).await?
        .and_then(|twine| async {
          let _ = self.cache.save(twine.clone()).await;
          Ok(twine)
        })
        .boxed()
    )
  }

}
