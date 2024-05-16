use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use futures::{Stream, StreamExt};
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
  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError>;
  async fn range_stream<'a>(&'a self, range: RangeQuery) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError>;
  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError>;
}

#[async_trait]
impl<'r> BaseResolver for Box<dyn BaseResolver + 'r> {
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

  async fn range_stream<'a>(&'a self, range: RangeQuery) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError> {
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
          i if i < 0 => self.resolve_latest(strand).await?.index() as i64 + i,
          i => i
        } as u64;
        self.resolve_index(strand, index).await
      },
      Query::Latest(strand) => self.resolve_latest(strand).await,
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
    let strand = self.resolve_strand(range.strand_cid()).await?;
    let stream = self.range_stream(range).await?
      .map(move |tixel| {
        Twine::try_new_from_shared(strand.clone(), tixel?)
          .map_err(|e| e.into())
      });
    Ok(stream.boxed())
  }
}

impl<R> Resolver for R where R: BaseResolver {}

#[async_trait]
impl BaseResolver for Vec<Box<dyn BaseResolver>> {
  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    for resolver in self {
      if resolver.has_twine(strand, cid).await? {
        return Ok(true);
      }
    }
    Ok(false)
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    for resolver in self {
      if resolver.has_strand(cid).await? {
        return Ok(true);
      }
    }
    Ok(false)
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    for resolver in self {
      if let Ok(tixel) = resolver.fetch_latest(strand).await {
        return Ok(tixel);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    for resolver in self {
      if let Ok(tixel) = resolver.fetch_index(strand, index).await {
        return Ok(tixel);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    for resolver in self {
      if let Ok(t) = resolver.fetch_tixel(strand, tixel).await {
        return Ok(t);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    for resolver in self {
      if let Ok(s) = resolver.fetch_strand(strand).await {
        return Ok(s);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn range_stream<'a>(&'a self, range: RangeQuery) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    for resolver in self {
      if let Ok(stream) = resolver.range_stream(range.clone()).await {
        return Ok(stream);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError> {
    let mut strands = HashMap::new();
    for resolver in self {
      while let Some(strand) = resolver.strands().await?.next().await {
        let strand = strand?;
        strands.insert(strand.cid(), strand);
      }
    }
    let vec = strands.values().cloned().collect::<Vec<_>>();
    Ok(futures::stream::iter(vec).map(|t| Ok(t.clone())).boxed())
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::Cid;

  #[test]
  fn test_range_query_bounds() {
    let cid = Cid::default();
    let range = RangeQuery::from_range_bounds(&cid, 0..2);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 1, 0)));
    let range = RangeQuery::from_range_bounds(&cid, 2..);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 2, 0)));
    let range = RangeQuery::from_range_bounds(&cid, 4..1);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 4, 2)));
    let range = RangeQuery::from_range_bounds(&cid, 2..=4);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 4, 2)));
    let range = RangeQuery::from_range_bounds(&cid, 3..=1);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 3, 1)));
    let range = RangeQuery::from_range_bounds(&cid, -1..);
    assert_eq!(range, RangeQuery::Relative(cid, -1, 0));
    let range = RangeQuery::from_range_bounds(&cid, ..=-2);
    assert_eq!(range, RangeQuery::Relative(cid, -1, -2));
    let range = RangeQuery::from_range_bounds(&cid, ..);
    assert_eq!(range, RangeQuery::Relative(cid, -1, 0));
    let range = RangeQuery::from_range_bounds(&cid, -1..-1);
    assert_eq!(range, RangeQuery::Relative(cid, -1, -1));
    let range = RangeQuery::from_range_bounds(&cid, -1..=-2);
    assert_eq!(range, RangeQuery::Relative(cid, -1, -2));
    let range = RangeQuery::from_range_bounds(&cid, ..=2);
    assert_eq!(range, RangeQuery::Relative(cid, -1, 2));
    let range = RangeQuery::from_range_bounds(&cid, -3..-1);
    assert_eq!(range, RangeQuery::Relative(cid, -2, -3));
  }

  #[test]
  fn test_batches(){
    let range = AbsoluteRange::new(Cid::default(), 100, 0);
    let batches = range.batches(100);
    let cid = Cid::default();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0], AbsoluteRange::new(cid.clone(), 100, 1));
    assert_eq!(batches[1], AbsoluteRange::new(cid, 0, 0));
  }
}
