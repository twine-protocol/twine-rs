use std::collections::HashSet;
use std::sync::Arc;
use futures::{Stream, StreamExt, TryStreamExt};
use async_trait::async_trait;
use crate::Cid;
use std::pin::Pin;
use crate::as_cid::AsCid;
use crate::twine::{Strand, Tixel, Twine};
use crate::errors::ResolutionError;

mod query;
pub use query::*;

#[async_trait]
pub trait BaseResolver: Send + Sync {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError>;
  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError>;
  async fn range_stream<'a>(&'a self, range: AbsoluteRange) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError>;
  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError>;
  async fn latest_index(&self, strand: &Cid) -> Result<u64, ResolutionError> {
    Ok(self.fetch_latest(strand).await?.index())
  }
}

#[async_trait]
impl<'r> BaseResolver for Box<dyn BaseResolver + 'r> {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    self.as_ref().has_index(strand, index).await
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    self.as_ref().has_twine(strand, cid).await
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    self.as_ref().has_strand(cid).await
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    self.as_ref().fetch_latest(strand).await
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    self.as_ref().fetch_index(strand, index).await
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    self.as_ref().fetch_tixel(strand, tixel).await
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    self.as_ref().fetch_strand(strand).await
  }

  async fn range_stream<'a>(&'a self, range: AbsoluteRange) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    self.as_ref().range_stream(range).await
  }

  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    self.as_ref().strands().await
  }
}

#[async_trait]
impl<T> BaseResolver for Arc<T> where T: BaseResolver {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    self.as_ref().has_index(strand, index).await
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    self.as_ref().has_twine(strand, cid).await
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    self.as_ref().has_strand(cid).await
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    self.as_ref().fetch_latest(strand).await
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    self.as_ref().fetch_index(strand, index).await
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    self.as_ref().fetch_tixel(strand, tixel).await
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    self.as_ref().fetch_strand(strand).await
  }

  async fn range_stream<'a>(&'a self, range: AbsoluteRange) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    self.as_ref().range_stream(range).await
  }

  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    self.as_ref().strands().await
  }

}

#[async_trait]
pub trait Resolver: BaseResolver + Send + Sync {
  async fn resolve<Q: Into<Query> + Send>(&self, query: Q) -> Result<Twine, ResolutionError> {
    let query = query.into();
    match query {
      Query::Stitch(stitch) => {
        self.resolve_twine(stitch.strand, stitch.tixel).await
      }
      Query::Index(strand, index) => {
        let index = match index {
          i if i < 0 => self.fetch_latest(strand.as_cid()).await?.index() as i64 + i + 1,
          i => i
        } as u64;
        self.resolve_index(strand, index).await
      },
      Query::Latest(strand) => self.resolve_latest(strand).await,
    }
  }

  async fn has<Q: Into<Query> + Send>(&self, query: Q) -> Result<bool, ResolutionError> {
    let query = query.into();
    match query {
      Query::Stitch(stitch) => {
        self.has_twine(stitch.strand.as_cid(), stitch.tixel.as_cid()).await
      }
      Query::Index(strand, index) => {
        let index = match index {
          i if i < 0 => self.fetch_latest(strand.as_cid()).await?.index() as i64 + i + 1,
          i => i
        } as u64;
        self.has_index(strand.as_cid(), index).await
      },
      Query::Latest(strand) => match self.fetch_latest(strand.as_cid()).await {
        Ok(_) => Ok(true),
        Err(ResolutionError::NotFound) => Ok(false),
        Err(e) => Err(e),
      },
    }
  }

  async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError> {
    use futures::join;
    let (strand, tixel) = join!(self.fetch_strand(&strand.as_cid()), self.fetch_latest(&strand.as_cid()));
    Ok(Twine::try_new_from_shared(strand?, tixel?)?)
  }

  async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: u64) -> Result<Twine, ResolutionError> {
    use futures::join;
    let (strand, tixel) = join!(self.fetch_strand(&strand.as_cid()), self.fetch_index(&strand.as_cid(), index));
    Ok(Twine::try_new_from_shared(strand?, tixel?)?)
  }

  async fn resolve_twine<C: AsCid + Send>(&self, strand: C, tixel: C) -> Result<Twine, ResolutionError> {
    use futures::join;
    let (strand, tixel) = join!(self.fetch_strand(&strand.as_cid()), self.fetch_tixel(&strand.as_cid(), &tixel.as_cid()));
    Ok(Twine::try_new_from_shared(strand?, tixel?)?)
  }

  async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
    self.fetch_strand(&strand.as_cid()).await
  }

  async fn resolve_range<'a, R: Into<RangeQuery> + Send>(&'a self, range: R) -> Result<Pin<Box<dyn Stream<Item = Result<Twine, ResolutionError>> + Send + 'a>>, ResolutionError> {
    let range = range.into();
    let latest = self.resolve_latest(range.strand_cid()).await?;
    let range = range.to_absolute(latest.index());
    if range.len() == 1 {
      return Ok::<_, ResolutionError>(futures::stream::once({
        let strand_cid = range.strand_cid().clone();
        async move {
          let tixel = self.fetch_index(&strand_cid, range.start).await?;
          if tixel.index() != range.start {
            return Err(ResolutionError::Fetch(format!("index mismatch (expected: {}, got: {})", tixel.index(), range.start)));
          }
          Twine::try_new_from_shared(latest.strand(), tixel).map_err(|e| e.into())
        }
      }).boxed());
    }
    let expected = range.clone().iter();
    let stream = self.range_stream(range).await?
      .zip(futures::stream::iter(expected))
      .map(move |(tixel, q)| {
        let tixel = tixel?;
        if tixel.index() != q.unwrap_index() as u64 {
          return Err(ResolutionError::Fetch(format!("index mismatch (expected: {}, got: {})", q.unwrap_index(), tixel.index())));
        }
        Twine::try_new_from_shared(latest.strand(), tixel)
          .map_err(|e| e.into())
      });
    Ok(stream.boxed())
  }
}

impl<R> Resolver for R where R: BaseResolver {}

#[derive(Clone)]
pub struct ResolverSetSeries<T>(Vec<T>) where T: BaseResolver;

impl<T> ResolverSetSeries<T> where T: BaseResolver {
  pub fn new(resolvers: Vec<T>) -> Self {
    Self(resolvers)
  }
}

impl<T> std::ops::Deref for ResolverSetSeries<T> where T: BaseResolver {
  type Target = Vec<T>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

#[async_trait]
impl<T> BaseResolver for ResolverSetSeries<T> where T: BaseResolver {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    for resolver in self.iter() {
      if resolver.has_index(strand, index).await? {
        return Ok(true);
      }
    }
    Ok(false)
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    for resolver in self.iter() {
      if resolver.has_twine(strand, cid).await? {
        return Ok(true);
      }
    }
    Ok(false)
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    for resolver in self.iter() {
      if resolver.has_strand(cid).await? {
        return Ok(true);
      }
    }
    Ok(false)
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let tasks = self.iter().map(|r| r.fetch_latest(strand))
      .collect::<Vec<_>>();
    let results = futures::future::join_all(tasks).await.into_iter()
      .filter_map(|res| match res {
        Ok(t) => Some(t),
        Err(_) => None,
      })
      .max_by(|a, b| a.index().cmp(&b.index()));
    match results {
      Some(t) => Ok(t),
      None => Err(ResolutionError::NotFound),
    }
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    for resolver in self.iter() {
      if let Ok(tixel) = resolver.fetch_index(strand, index).await {
        return Ok(tixel);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    for resolver in self.iter() {
      if let Ok(t) = resolver.fetch_tixel(strand, tixel).await {
        return Ok(t);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    for resolver in self.iter() {
      if let Ok(s) = resolver.fetch_strand(strand).await {
        return Ok(s);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn range_stream<'a>(&'a self, range: AbsoluteRange) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    for resolver in self.iter() {
      // TODO: should find a way to merge streams
      if resolver.has((range.strand_cid(), range.start)).await? {
        if let Ok(stream) = resolver.range_stream(range.into()).await {
          return Ok(stream);
        }
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    let stream = futures::stream::iter(self.iter())
      .map(|r| r.strands())
      .buffered(10)
      .try_flatten()
      .scan(HashSet::new(), |seen, strand| {
        use futures::future::ready;
        let strand = match strand {
          Ok(s) => s,
          Err(e) => return ready(Some(Err(e))),
        };
        if seen.contains(&strand.cid()) {
          return ready(Some(Ok(None)));
        }
        seen.insert(strand.cid());
        ready(Some(Ok(Some(strand))))
      })
      // TODO: maybe log error?
      .filter_map(|res| async move {
        match res {
          Ok(Some(s)) => Some(Ok(s)),
          Ok(None) => None,
          Err(_) => None,
        }
      })
      .boxed();

    Ok(stream)
  }
}
