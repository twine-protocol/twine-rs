use std::fmt::Display;
use std::str::FromStr;
use std::ops::RangeBounds;
use futures::{stream::once, Stream, TryStreamExt};
use libipld::Cid;
use crate::as_cid::AsCid;
use crate::twine::{Stitch, Strand, Tixel};
use crate::errors::{ConversionError, ResolutionError};
use super::{BaseResolver, Resolver};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum Query {
  Stitch(Stitch),
  Index(Cid, i64),
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
    Self::Index(strand.as_cid().clone(), index as i64)
  }
}

impl<C> From<(C, i64)> for Query where C: AsCid {
  fn from((strand, index): (C, i64)) -> Self {
    Self::Index(strand.as_cid().clone(), index)
  }
}

impl<C> From<(C, C)> for Query where C: AsCid {
  fn from((strand, tixel): (C, C)) -> Self {
    Self::Stitch((strand.as_cid().clone(), tixel.as_cid().clone()).into())
  }
}

impl From<Cid> for Query {
  fn from(cid: Cid) -> Self {
    Self::Latest(cid)
  }
}

impl FromStr for Query {
  type Err = ConversionError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let parts: Vec<&str> = s.split(':').collect();
    let cid = parts.get(0).ok_or(ConversionError::InvalidFormat("Invalid Selector".into()))?;
    let strand_cid = Cid::try_from(cid.to_string())?;
    if parts.len() == 1 {
      Ok(strand_cid.into())
    } else if parts.len() == 2 {
      let arg = parts.get(1).unwrap();
      match *arg {
        "latest"|"" => Ok(strand_cid.into()),
        _ => {
          if let Ok(cid) = Cid::try_from(arg.to_string()) {
            Ok((strand_cid, cid).into())
          } else {
            let index: i64 = arg.parse()?;
            Ok((strand_cid, index).into())
          }
        }
      }
    } else {
      Err(ConversionError::InvalidFormat("Invalid Selector".into()))
    }
  }
}

impl Display for Query {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Query::Stitch(stitch) => write!(f, "{}:{}", stitch.strand, stitch.tixel),
      Query::Index(cid, index) => write!(f, "{}:{}", cid, index),
      Query::Latest(cid) => write!(f, "{}:latest", cid),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
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
      if lower - self.lower < size {
        batches.push(Self::new(self.strand.clone(), lower.saturating_sub(1), self.lower));
        break;
      }
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
      Some((self.range.strand.clone(), self.current).into())
    } else {
      None
    }
  }
}

/// A range of indices on a strand
///
/// The range can be absolute, meaning the indices are known,
/// or relative, meaning the range is somehow relative to the latest index.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
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

  pub fn to_absolute(self, latest: u64) -> AbsoluteRange {
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

  pub async fn try_to_absolute(self, resolver: Resolver<'_>) -> Result<AbsoluteRange, ResolutionError> {
    match self {
      Self::Absolute(range) => Ok(range),
      Self::Relative(strand, _, _) => {
        let latest = resolver.resolve_latest(strand).await?.index();
        Ok(self.to_absolute(latest))
      }
    }
  }

  pub fn to_stream<'a>(self, resolver: Resolver<'a>) -> impl Stream<Item = Result<Query, ResolutionError>> + 'a {
    once(async move {
      self.try_to_absolute(resolver).await
        .map(|result| futures::stream::iter(result.into_iter().map(Ok)))
    })
      .try_flatten()
  }

  pub fn to_batch_stream<'a>(self, resolver: Resolver<'a>, size: u64) -> impl Stream<Item = Result<AbsoluteRange, ResolutionError>> + 'a {
    use futures::stream::StreamExt;
    once(async move {
      self.try_to_absolute(resolver).await
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

impl FromStr for RangeQuery {
  type Err = ConversionError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let parts: Vec<&str> = s.split(':').collect();
    if !parts.len() == 3 {
      return Err(ConversionError::InvalidFormat("Invalid range query string".to_string()));
    }
    let cid_str = parts.get(0).unwrap();
    let maybe_upper = parts.get(1).unwrap();
    let maybe_lower = parts.get(2).unwrap();
    let cid = Cid::try_from(*cid_str)?;
    match (*maybe_upper, *maybe_lower) {
      ("", "") => Ok((cid, ..).into()),
      (upper, "") => {
        let upper: i64 = upper.parse()?;
        Ok((cid, upper..).into())
      },
      ("", lower) => {
        let lower: i64 = lower.parse()?;
        Ok((cid, ..lower).into())
      },
      (upper, lower) => {
        let upper: i64 = upper.parse()?;
        let lower: i64 = lower.parse()?;
        Ok((cid, upper..lower).into())
      }
    }
  }
}

impl Display for RangeQuery {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RangeQuery::Absolute(range) => write!(f, "{}:{}:{}", range.strand, range.upper, range.lower),
      RangeQuery::Relative(strand, upper, lower) => write!(f, "{}:{}:{}", strand, upper, lower),
    }
  }
}
