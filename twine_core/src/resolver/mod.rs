use std::ops::Deref;
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
  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError>;
  async fn range_stream<'a>(&'a self, range: RangeQuery) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError>;
  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError>;

  fn resolver(&self) -> Resolver<'_> where Self: Sized {
    Resolver::new(self)
  }
}

#[derive(Clone, Copy)]
pub struct Resolver<'a>(&'a dyn BaseResolver);

impl<'r> Resolver<'r> {
  pub fn new(resolver: &'r dyn BaseResolver) -> Self {
    Self(resolver)
  }

  pub async fn resolve<Q: Into<Query> + Send>(&self, query: Q) -> Result<Twine, ResolutionError> {
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

  pub async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError> {
    use futures::join;
    let (strand, tixel) = join!(self.fetch_strand(&strand.as_cid()), self.fetch_latest(&strand.as_cid()));
    Ok(Twine::try_new_from_shared(strand?, tixel?)?)
  }

  pub async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: u64) -> Result<Twine, ResolutionError> {
    use futures::join;
    let (strand, tixel) = join!(self.fetch_strand(&strand.as_cid()), self.fetch_index(&strand.as_cid(), index));
    Ok(Twine::try_new_from_shared(strand?, tixel?)?)
  }

  pub async fn resolve_twine<C: AsCid + Send>(&self, strand: C, tixel: C) -> Result<Twine, ResolutionError> {
    use futures::join;
    let (strand, tixel) = join!(self.fetch_strand(&strand.as_cid()), self.fetch_tixel(&strand.as_cid(), &tixel.as_cid()));
    Ok(Twine::try_new_from_shared(strand?, tixel?)?)
  }

  pub async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
    self.fetch_strand(&strand.as_cid()).await
  }

  pub async fn resolve_range<'a, R: Into<RangeQuery> + Send>(&'a self, range: R) -> Result<Pin<Box<dyn Stream<Item = Result<Twine, ResolutionError>> + Send + 'a>>, ResolutionError> {
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

impl<'a> Deref for Resolver<'a> {
  type Target = dyn BaseResolver + 'a;

  fn deref(&self) -> &'a Self::Target {
    self.0
  }
}

// pub trait Resolver: BaseResolver + Clone + Send + Sync {
//   async fn resolve_cid<C: AsCid + Send>(&self, cid: C) -> Result<AnyTwine, ResolutionError>;
//   async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: i64) -> Result<Twine, ResolutionError>;
//   async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError>;

//   async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError>;

//   async fn has<C: AsCid + Send>(&self, cid: C) -> bool {
//     self.resolve_cid(cid).await.is_ok()
//   }

//   async fn resolve<Q: Into<Query> + Send>(&self, query: Q) -> Result<Twine, ResolutionError> {
//     let query = query.into();
//     match query {
//       Query::Stitch(stitch) => {
//         let strand = self.resolve_strand(stitch.strand);
//         let tixel = self.resolve_tixel(stitch.tixel);
//         let (strand, tixel) = futures::try_join!(strand, tixel)?;
//         Ok(Twine::try_new_from_shared(strand, tixel)?)
//       }
//       Query::Index(strand, index) => self.resolve_index(strand, index).await,
//       Query::Latest(strand) => self.resolve_latest(strand).await,
//     }
//   }

//   async fn resolve_tixel<C: AsCid + Send>(&self, strand: C, tixel: C) -> Result<Twine, ResolutionError> {
//     let strand = self.fetch_strand(strand.as_cid()).await?;
//     self.fetch_tixel(strand, tixel.as_cid()).await
//   }

//   async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
//     self.fetch_strand(&strand.as_cid()).await
//   }

//   async fn resolve_range<'a, R: Into<RangeQuery> + Send>(&'a self, range: R) -> Result<Pin<Box<dyn Stream<Item = Result<Twine, ResolutionError>> + Send + 'a>>, ResolutionError> {
//     let range = range.into();
//     use futures::stream::StreamExt;
//     let stream = range.to_stream(self)
//       .then(|q| async { self.resolve(q?).await });
//     Ok(stream.boxed())
//   }
// }

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
