use std::{ops::RangeBounds, sync::Arc};
use futures::{stream::once, Stream, TryStreamExt};
use thiserror::Error;
use libipld::Cid;
use async_trait::async_trait;
use std::pin::Pin;
use crate::{prelude::{AnyTwine, Stitch, Strand, Tixel, Twine, VerificationError}, as_cid::AsCid};

#[derive(Error, Debug)]
pub enum ResolutionError {
  #[error("Twine not found")]
  NotFound,
  #[error("Twine is invalid: {0}")]
  Invalid(#[from] VerificationError),
  #[error("Bad data: {0}")]
  BadData(String),
  #[error("Problem fetching data: {0}")]
  Fetch(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum Query {
  Stitch(Stitch),
  Index(Cid, u64),
  Latest(Cid),
}

impl From<Stitch> for Query {
  fn from(stitch: Stitch) -> Self {
    Self::Stitch(stitch)
  }
}

impl From<Tixel> for Query {
  fn from(tixel: Tixel) -> Self {
    tixel.into()
  }
}

impl From<Strand> for Query {
  fn from(strand: Strand) -> Self {
    Self::Latest(strand.into())
  }
}

impl<C> From<(C, u64)> for Query where C: AsCid {
  fn from((strand, index): (C, u64)) -> Self {
    Self::Index(strand.as_cid().clone(), index)
  }
}

impl PartialOrd for Query {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    match (self, other) {
      (Query::Index(a, i), Query::Index(b, j)) => {
        if a == b {
          i.partial_cmp(j)
        } else {
          None
        }
      }
      (Query::Latest(a), Query::Latest(b)) => if a == b {
        Some(std::cmp::Ordering::Equal)
      } else {
        None
      },
      (Query::Index(a, _), Query::Latest(b)) => if a == b {
        Some(std::cmp::Ordering::Less)
      } else {
        None
      },
      (Query::Latest(a), Query::Index(b, _)) => if a == b {
        Some(std::cmp::Ordering::Greater)
      } else {
        None
      },
      _ => None,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbsoluteRange {
  pub strand: Cid,
  pub upper: u64,
  pub lower: u64,
}

impl AbsoluteRange {
  pub fn new(strand: Cid, upper: u64, lower: u64) -> Self {
    let upper = upper.max(lower);
    let lower = lower.min(upper);
    Self { strand, upper, lower }
  }

  pub fn batches(&self, size: u64) -> Vec<Self> {
    let mut batches = Vec::new();
    let mut upper = self.upper;
    while upper > self.lower {
      let lower = (upper + 1).saturating_sub(size).max(self.lower);
      batches.push(Self::new(self.strand.clone(), upper, lower));
      upper = lower.saturating_sub(1);
    }
    batches
  }
}

#[derive(Debug, Clone)]
pub struct AbsoluteRangeIter {
  range: AbsoluteRange,
  current: u64,
}

impl IntoIterator for AbsoluteRange {
  type Item = Query;
  type IntoIter = AbsoluteRangeIter;

  fn into_iter(self) -> Self::IntoIter {
    AbsoluteRangeIter::new(self)
  }
}

impl AbsoluteRangeIter {
  pub fn new(range: AbsoluteRange) -> Self {
    Self { current: range.upper + 1, range }
  }
}

impl Iterator for AbsoluteRangeIter {
  type Item = Query;

  fn next(&mut self) -> Option<Self::Item> {
    if self.current > self.range.lower {
      self.current -= 1;
      Some(Query::Index(self.range.strand.clone(), self.current))
    } else {
      None
    }
  }
}

/// A range of indices on a strand
///
/// The range can be absolute, meaning the indices are known,
/// or relative, meaning the range is somehow relative to the latest index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RangeQuery {
  Absolute(AbsoluteRange),
  Relative(Cid, i64, i64),
}

impl RangeQuery {
  // ..2 -> latest to 2 (relative)
  // 2.. -> 2 to 0 (absolute)
  // 4..1 -> 4 to 2 (absolute)
  // 2..=4 -> 4 to 2 again (absolute)
  // -1..5 -> latest to 6 (relative)
  // ..=-2 -> latest to (latest - 1) (relative)
  pub fn from_range_bounds<C: AsCid, T: RangeBounds<i64>>(strand: C, range: T) -> Self {
    use std::ops::Bound;
    let dir = |start: i64, end: i64| (end - start).signum();
    let start = match range.start_bound() {
      Bound::Unbounded => Bound::Included(&-1i64),
      start@_ => start,
    };
    let end = match range.end_bound() {
      Bound::Unbounded => Bound::Included(&0i64),
      end@_ => end,
    };
    let (start, end) = match (start, end) {
      (Bound::Included(s), Bound::Included(e)) => (*s, *e),
      (Bound::Included(s), Bound::Excluded(e)) => (*s, e - dir(*s, *e)),
      (Bound::Excluded(s), Bound::Included(e)) => (s + dir(*s, *e), *e),
      (Bound::Excluded(s), Bound::Excluded(e)) => (s + dir(*s, *e), e - dir(*s, *e)),
      _ => unreachable!(),
    };
    let (upper, lower) = (start.max(end), start.min(end));
    match (upper, lower) {
      (u, l) if u >= 0 && l >= 0 => Self::Absolute(AbsoluteRange::new(strand.as_cid().clone(), u as u64, l as u64)),
      (u, l) if u >= 0 => Self::Relative(strand.as_cid().clone(), l, u),
      (u, l) => Self::Relative(strand.as_cid().clone(), u, l),
    }
  }

  pub fn to_definite(self, latest: u64) -> AbsoluteRange {
    match self {
      Self::Absolute(range) => range,
      Self::Relative(cid, u, l) => {
        let (u, l) = match (u, l) {
          // this shouldn't happen.. but anyway..
          (u, l) if u >= 0 && l >= 0 => ((u as u64).max(l as u64), (u as u64).min(l as u64)),
          // if they are both less than zero, they are both relative
          (u, l) if u < 0 && l < 0 =>
            (
              (latest + 1).saturating_sub(-u as u64),
              (latest + 1).saturating_sub(-l as u64)
            ),
          // otherwise the first is relative and the second is absolute
          (u, l) => (
            (latest + 1).saturating_sub(-u as u64),
            l as u64
          ),
        };
        // ensure that the lower bound is less than or equal to the upper bound
        AbsoluteRange::new(cid, u, l.min(u))
      }
    }
  }

  pub async fn try_to_definite<R: Resolver>(self, resolver: &R) -> Result<AbsoluteRange, ResolutionError> {
    match self {
      Self::Absolute(range) => Ok(range),
      Self::Relative(strand, _, _) => {
        let latest = resolver.resolve_latest(strand).await?.index();
        Ok(self.to_definite(latest))
      }
    }
  }

  pub fn to_stream<'a, R: Resolver>(self, resolver: &'a R) -> impl Stream<Item = Result<Query, ResolutionError>> + 'a {
    once(async move {
      self.try_to_definite(resolver).await
        .map(|result| futures::stream::iter(result.into_iter().map(Ok)))
    })
      .try_flatten()
  }

  pub fn to_batch_stream<'a, R: Resolver>(self, resolver: &'a R, size: u64) -> impl Stream<Item = Result<AbsoluteRange, ResolutionError>> + 'a {
    use futures::stream::StreamExt;
    once(async move {
      self.try_to_definite(resolver).await
        .map(|result| futures::stream::iter(result.batches(size)).map(Ok))
    }).try_flatten()
  }

  pub fn is_absolute(&self) -> bool {
    matches!(self, Self::Absolute(_))
  }

  pub fn strand_cid(&self) -> &Cid {
    match self {
      Self::Absolute(range) => &range.strand,
      Self::Relative(strand, _, _) => strand,
    }
  }
}

impl<C, R> From<(C, R)> for RangeQuery where R: RangeBounds<i64>, C: AsCid {
  fn from((strand, range): (C, R)) -> Self {
    Self::from_range_bounds(strand.as_cid(), range)
  }
}

#[async_trait]
pub trait Resolver: Clone + Send + Sync {
  async fn resolve_cid<C: AsCid + Send>(&self, cid: C) -> Result<AnyTwine, ResolutionError>;
  async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: u64) -> Result<Twine, ResolutionError>;
  async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError>;

  async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + 'a>>, ResolutionError>;

  async fn has<C: AsCid + Send>(&self, cid: C) -> bool {
    self.resolve_cid(cid).await.is_ok()
  }

  async fn resolve<Q: Into<Query> + Send>(&self, query: Q) -> Result<Twine, ResolutionError> {
    let query = query.into();
    match query {
      Query::Stitch(stitch) => {
        let strand = self.resolve_strand(stitch.strand);
        let tixel = self.resolve_tixel(stitch.tixel);
        let (strand, tixel) = futures::try_join!(strand, tixel)?;
        Ok(Twine::try_new_from_shared(strand, tixel)?)
      }
      Query::Index(strand, index) => self.resolve_index(strand, index).await,
      Query::Latest(strand) => self.resolve_latest(strand).await,
    }
  }

  async fn resolve_tixel<C: AsCid + Send>(&self, tixel: C) -> Result<Arc<Tixel>, ResolutionError> {
    let twine = self.resolve_cid(tixel).await?;
    Ok(twine.try_into()?)
  }

  async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
    let task = self.resolve_cid(strand);
    let twine = task.await?;
    Ok(twine.try_into()?)
  }

  async fn resolve_range<'a, R: Into<RangeQuery> + Send>(&'a self, range: R) -> Result<Pin<Box<dyn Stream<Item = Result<Twine, ResolutionError>> + 'a>>, ResolutionError> {
    let range = range.into();
    use futures::stream::StreamExt;
    let stream = range.to_stream(self)
      .then(|q| async { self.resolve(q?).await });
    Ok(stream.boxed())
  }
}

// #[async_trait]
// impl<T> Resolver for Arc<T> where T: Resolver {
//   async fn resolve_cid<C: AsCid + Send>(&self, cid: C) -> Result<AnyTwine, ResolutionError> {
//     self.as_ref().resolve_cid(cid).await
//   }

//   async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: u64) -> Result<Twine, ResolutionError> {
//     self.as_ref().resolve_index(strand, index).await
//   }

//   async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError> {
//     self.as_ref().resolve_latest(strand).await
//   }

//   async fn resolve_tixel<C: AsCid + Send>(&self, tixel: C) -> Result<Arc<Tixel>, ResolutionError> {
//     self.as_ref().resolve_tixel(tixel).await
//   }

//   async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
//     self.as_ref().resolve_strand(strand).await
//   }

//   fn resolve_range<C: AsCid + Send, R: RangeBounds<i64> + Send>(&self, strand: C, range: R) -> impl Stream<Item = Result<Twine, ResolutionError>> where Self: Sized + Sync {
//     self.as_ref().resolve_range(strand, range)
//   }
// }

#[cfg(test)]
mod test {
  use super::*;

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
}
