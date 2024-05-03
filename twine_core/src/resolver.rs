use std::{ops::RangeBounds, sync::Arc};
use futures::{stream::once, Stream, TryStreamExt};
use thiserror::Error;
use libipld::Cid;
use async_trait::async_trait;
use crate::{prelude::{AnyTwine, Stitch, Strand, Tixel, Twine, VerificationError}, as_cid::AsCid};

#[derive(Error, Debug)]
pub enum ResolutionError {
  #[error("Twine not found")]
  NotFound,
  #[error("Twine is invalid")]
  Invalid(#[from] VerificationError),
  #[error("Twine has wrong type: expected {expected}, found {found}")]
  WrongType {
    expected: String,
    found: String,
  },
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
pub struct DefiniteRange {
  pub strand: Cid,
  pub upper: u64,
  pub lower: u64,
}

impl DefiniteRange {
  pub fn new(strand: Cid, upper: u64, lower: u64) -> Self {
    let upper = upper.max(lower);
    let lower = lower.min(upper);
    Self { strand, upper, lower }
  }

  pub fn batches(&self, size: u64) -> Vec<Self> {
    let mut batches = Vec::new();
    let mut upper = self.upper;
    while upper > self.lower {
      let lower = (upper + 1).saturating_sub(size);
      batches.push(Self::new(self.strand.clone(), upper, lower));
      upper = lower.saturating_sub(1);
    }
    batches
  }
}

#[derive(Debug, Clone)]
pub struct DefiniteRangeIter {
  range: DefiniteRange,
  current: u64,
}

impl IntoIterator for DefiniteRange {
  type Item = Query;
  type IntoIter = DefiniteRangeIter;

  fn into_iter(self) -> Self::IntoIter {
    DefiniteRangeIter::new(self)
  }
}

impl DefiniteRangeIter {
  pub fn new(range: DefiniteRange) -> Self {
    Self { current: range.upper + 1, range }
  }
}

impl Iterator for DefiniteRangeIter {
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
/// The range can be definite, meaning the indices are known,
/// or indefinite, meaning the range begins at the latest index to a known index,
/// or relative, meaning the range begins at the latest index and goes back a certain number of indices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RangeQuery {
  Definite(DefiniteRange),
  Indefinite(Cid, u64),
  Relative(Cid, u64),
}

impl RangeQuery {
  // ..2 -> latest to 2
  // 2.. -> 2 to 0
  // 4..1 -> 4 to 2
  // 2..=4 -> 4 to 2 again
  // -1.. -> latest to 0
  // ..=-2 -> latest to (latest - 1)
  pub fn from_range_bounds<C: AsCid, T: RangeBounds<i64>>(strand: C, range: T) -> Self {
    let lower = match range.end_bound() {
      std::ops::Bound::Included(u) => *u,
      std::ops::Bound::Excluded(u) => u + 1,
      std::ops::Bound::Unbounded => 0,
    };
    let upper = match range.start_bound() {
      std::ops::Bound::Included(u) => *u,
      std::ops::Bound::Excluded(u) => u - 1,
      std::ops::Bound::Unbounded => -1,
    };
    match (upper, lower) {
      (u, l) if u < 0 && l < 0 => Self::Relative(strand.as_cid().clone(), (-l) as u64),
      (u, l) if u < 0 => Self::Indefinite(strand.as_cid().clone(), l as u64),
      (u, l) if l < 0 => Self::Definite(DefiniteRange::new(strand.as_cid().clone(), u as u64, 0)),
      (u, l) => Self::Definite(DefiniteRange::new(strand.as_cid().clone(), u as u64, l as u64)),
    }
  }

  pub fn to_definite(self, latest: u64) -> DefiniteRange {
    match self {
      Self::Definite(range) => range,
      Self::Indefinite(_, l) => DefiniteRange::new(Cid::default(), latest, l),
      Self::Relative(_, l) => DefiniteRange::new(Cid::default(), latest, (latest + 1).saturating_sub(l)),
    }
  }

  pub async fn try_to_definite<R: Resolver>(self, resolver: &R) -> Result<DefiniteRange, ResolutionError> {
    match self {
      Self::Definite(range) => Ok(range),
      Self::Indefinite(strand, l) => {
        let latest = resolver.resolve_latest(strand).await?.index();
        Ok(DefiniteRange::new(strand, latest, l))
      }
      Self::Relative(strand, l) => {
        let latest = resolver.resolve_latest(strand).await?.index();
        Ok(DefiniteRange::new(strand, latest, (latest + 1).saturating_sub(l)))
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

  pub fn to_batch_stream<'a, R: Resolver>(self, resolver: &'a R, size: u64) -> impl Stream<Item = Result<DefiniteRange, ResolutionError>> + 'a {
    use futures::stream::StreamExt;
    once(async move {
      self.try_to_definite(resolver).await
        .map(|result| futures::stream::iter(result.batches(size)).map(Ok))
    }).try_flatten()
  }

  pub fn is_definite(&self) -> bool {
    matches!(self, Self::Definite(_))
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
    match twine {
      AnyTwine::Tixel(tixel) => Ok(tixel),
      AnyTwine::Strand(_) => Err(ResolutionError::WrongType {
        expected: "Tixel".to_string(),
        found: "Strand".to_string(),
      }),
    }
  }

  async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
    let task = self.resolve_cid(strand);
    let twine = task.await?;
    match twine {
      AnyTwine::Strand(strand) => Ok(strand),
      AnyTwine::Tixel(_) => Err(ResolutionError::WrongType {
        expected: "Strand".to_string(),
        found: "Twine".to_string(),
      }),
    }
  }

  fn resolve_range<'a, R: Into<RangeQuery> + Send>(&'a self, range: R) -> impl Stream<Item = Result<Twine, ResolutionError>> + 'a {
    let range = range.into();
    use futures::stream::StreamExt;
    range.to_stream(self)
      .then(|q| async { self.resolve(q?).await })
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
