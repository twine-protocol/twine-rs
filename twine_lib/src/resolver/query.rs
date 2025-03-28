use super::Resolver;
use crate::as_cid::AsCid;
use crate::errors::{ConversionError, ResolutionError};
use crate::twine::{Stitch, Strand, Tixel, Twine};
use crate::Cid;
use futures::{stream::once, Stream, TryStreamExt};
use std::fmt::Display;
use std::ops::Bound;
use std::ops::RangeBounds;
use std::str::FromStr;

/// A Query for retrieval of a single tixel
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum SingleQuery {
  /// A query referenced using a stitch
  Stitch(Stitch),
  /// A query referenced using an index and a strand CID
  Index(Cid, i64),
  /// A query for the latest tixel on a strand
  Latest(Cid),
}

impl SingleQuery {
  /// Get the strand CID of the query
  pub fn strand_cid(&self) -> &Cid {
    match self {
      SingleQuery::Stitch(stitch) => &stitch.strand,
      SingleQuery::Index(cid, _) => cid,
      SingleQuery::Latest(cid) => cid,
    }
  }

  /// Get the index of the query
  ///
  /// Panics if the query is not an index query
  pub fn unwrap_index(self) -> i64 {
    match self {
      SingleQuery::Index(_, index) => index,
      _ => panic!("SingleQuery is not an index query"),
    }
  }

  /// Check if the query matches a twine
  ///
  /// A query matches a twine if:
  /// - It's a stitch query and the twine has the same strand and tixel cids
  /// - It's a positive index query and the twine has the same strand cid and the index matches
  /// - For a relative index query, the twine has the same strand cid
  /// - For a latest query, the twine has the same strand cid
  pub fn matches(&self, twine: &Twine) -> bool {
    match self {
      SingleQuery::Stitch(stitch) => twine == stitch,
      SingleQuery::Index(cid, index) => {
        if *index >= 0 && *index as u64 != twine.index() {
          return false;
        }
        twine.strand_cid() == *cid
      }
      SingleQuery::Latest(cid) => twine.strand_cid() == *cid,
    }
  }
}

impl From<Stitch> for SingleQuery {
  fn from(stitch: Stitch) -> Self {
    Self::Stitch(stitch)
  }
}

impl From<Tixel> for SingleQuery {
  fn from(tixel: Tixel) -> Self {
    tixel.into()
  }
}

impl From<Strand> for SingleQuery {
  fn from(strand: Strand) -> Self {
    Self::Latest(strand.into())
  }
}

impl<C> From<(C, u64)> for SingleQuery
where
  C: AsCid,
{
  fn from((strand, index): (C, u64)) -> Self {
    Self::Index(strand.as_cid().clone(), index as i64)
  }
}

impl<C> From<(C, i64)> for SingleQuery
where
  C: AsCid,
{
  fn from((strand, index): (C, i64)) -> Self {
    if index == -1 {
      Self::Latest(strand.as_cid().clone())
    } else {
      Self::Index(strand.as_cid().clone(), index)
    }
  }
}

impl<C> From<(C, i32)> for SingleQuery
where
  C: AsCid,
{
  fn from((strand, index): (C, i32)) -> Self {
    (strand, index as i64).into()
  }
}

impl<C> From<(C, u32)> for SingleQuery
where
  C: AsCid,
{
  fn from((strand, index): (C, u32)) -> Self {
    (strand, index as i64).into()
  }
}

impl<C> From<(C, usize)> for SingleQuery
where
  C: AsCid,
{
  fn from((strand, index): (C, usize)) -> Self {
    (strand, index as u64).into()
  }
}

impl<C> From<(C, i16)> for SingleQuery
where
  C: AsCid,
{
  fn from((strand, index): (C, i16)) -> Self {
    (strand, index as i64).into()
  }
}

impl<C> From<(C, u16)> for SingleQuery
where
  C: AsCid,
{
  fn from((strand, index): (C, u16)) -> Self {
    (strand, index as u64).into()
  }
}

impl<C> From<(C, C)> for SingleQuery
where
  C: AsCid,
{
  fn from((strand, tixel): (C, C)) -> Self {
    Self::Stitch((strand.as_cid().clone(), tixel.as_cid().clone()).into())
  }
}

impl From<Cid> for SingleQuery {
  fn from(cid: Cid) -> Self {
    Self::Latest(cid)
  }
}

impl FromStr for SingleQuery {
  type Err = ConversionError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let parts: Vec<&str> = s.split(':').collect();
    let cid = parts
      .get(0)
      .ok_or(ConversionError::InvalidFormat("Invalid Selector".into()))?;
    let strand_cid = Cid::try_from(cid.to_string())?;
    match parts.len() {
      2 => {
        let arg = parts.get(1).unwrap();
        match *arg {
          "latest" | "" | "-1" => Ok(strand_cid.into()),
          _ => {
            if let Ok(cid) = Cid::try_from(arg.to_string()) {
              Ok((strand_cid, cid).into())
            } else {
              let index: i64 = arg.parse()?;
              Ok((strand_cid, index).into())
            }
          }
        }
      }
      _ => Err(ConversionError::InvalidFormat("Invalid Selector".into())),
    }
  }
}

impl Display for SingleQuery {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SingleQuery::Stitch(stitch) => write!(f, "{}:{}", stitch.strand, stitch.tixel),
      SingleQuery::Index(cid, index) => write!(f, "{}:{}", cid, index),
      SingleQuery::Latest(cid) => write!(f, "{}:-1", cid),
    }
  }
}

/// A range of indices on a strand
///
/// The range is inclusive on both ends and indices are positive.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct AbsoluteRange {
  /// The strand CID
  pub strand: Cid,
  /// The start index
  pub start: u64,
  /// The end index
  pub end: u64,
}

impl AbsoluteRange {
  /// Create a new AbsoluteRange
  ///
  /// The start and end indices are inclusive
  ///
  /// It is preferred to use the `RangeQuery` enum to create ranges
  pub fn new(strand: Cid, start: u64, end: u64) -> Self {
    Self { strand, start, end }
  }

  /// Check if the range is increasing
  pub fn is_increasing(&self) -> bool {
    self.start <= self.end
  }

  /// Check if the range is decreasing
  pub fn is_decreasing(&self) -> bool {
    self.start > self.end
  }

  /// Get the lower bound of the range
  pub fn lower(&self) -> u64 {
    if self.is_increasing() {
      self.start
    } else {
      self.end
    }
  }

  /// Get the upper bound of the range
  pub fn upper(&self) -> u64 {
    if self.is_increasing() {
      self.end
    } else {
      self.start
    }
  }

  /// Get the length of the range
  pub fn len(&self) -> u64 {
    if self.is_increasing() {
      self.end - self.start + 1
    } else {
      self.start - self.end + 1
    }
  }

  /// Batch this range into a Vec of AbsoluteRanges of a given size
  pub fn batches(&self, size: u64) -> Vec<Self> {
    let mut batches = Vec::new();
    assert!(size > 0, "Batch size must be greater than 0");
    if self.is_decreasing() {
      // decreasing
      let mut upper = self.start;
      while upper >= self.end {
        let lower = upper.saturating_sub(size - 1).max(self.end);
        batches.push(Self::new(self.strand.clone(), upper, lower));
        if lower == 0 {
          break;
        }
        upper = lower.saturating_sub(1);
      }
    } else {
      // increasing
      let mut lower = self.start;
      while lower <= self.end {
        let upper = (lower + size - 1).min(self.end);
        batches.push(Self::new(self.strand.clone(), lower, upper));
        lower = upper + 1;
      }
    }
    batches
  }

  /// Get an iterator over the range
  pub fn iter(&self) -> AbsoluteRangeIter {
    AbsoluteRangeIter::new(*self)
  }

  /// Get the strand CID of the range
  pub fn strand_cid(&self) -> &Cid {
    &self.strand
  }
}

impl Display for AbsoluteRange {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}:{}:={}", self.strand, self.start, self.end)
  }
}

/// An iterator over an AbsoluteRange
///
/// Should be created by calling `iter` on an AbsoluteRange
#[derive(Debug, Clone)]
pub struct AbsoluteRangeIter {
  range: AbsoluteRange,
  current: Option<u64>,
  decreasing: bool,
}

impl IntoIterator for AbsoluteRange {
  type Item = SingleQuery;
  type IntoIter = AbsoluteRangeIter;

  fn into_iter(self) -> Self::IntoIter {
    AbsoluteRangeIter::new(self)
  }
}

impl AbsoluteRangeIter {
  /// Create a new AbsoluteRangeIter
  pub fn new(range: AbsoluteRange) -> Self {
    let decreasing = range.is_decreasing();
    let current = Some(range.start);
    Self {
      current,
      range,
      decreasing,
    }
  }
}

impl Iterator for AbsoluteRangeIter {
  type Item = SingleQuery;

  fn next(&mut self) -> Option<Self::Item> {
    if self.decreasing {
      if let Some(current) = self.current {
        if current >= self.range.end {
          self.current = current.checked_sub(1);
          Some((self.range.strand.clone(), current).into())
        } else {
          None
        }
      } else {
        None
      }
    } else {
      let current = self.current.unwrap();
      if current <= self.range.end {
        self.current = Some(current + 1);
        Some((self.range.strand.clone(), current).into())
      } else {
        None
      }
    }
  }
}

fn range_dir(s: i64, e: i64) -> i64 {
  if (s < 0) ^ (e < 0) {
    // one is relative and the other is absolute
    if s < 0 {
      // the relative one is the start
      -1
    } else {
      // the relative one is the end
      1
    }
  } else {
    // both are relative or both are absolute
    if s < e {
      1
    } else {
      -1
    }
  }
}

/// A range of indices on a strand
///
/// The range can be absolute, meaning the indices are known,
/// or relative, meaning the range is somehow relative to the latest index.
///
/// They can be constructed from a tuple of a strand and a range, or from a string.
/// The range is converted as follows:
/// - Positive numbers are absolute indices
/// - Negative numbers are relative to the latest index
/// - Range will respect rust's Inclusive/Exclusive range semantics
///
/// The range can be increasing or decreasing. Absolute ranges are increasing
/// if the start is less than the end and vice versa.
///
/// A relative range with both negative start and end indices is
/// increasing if the start is less than the end and vice versa.
///
/// Relative ranges with one negative and one positive index are
/// increasing if the start is positive and vice versa.
///
/// The "all" range is represented as `..` is equivalent to `0..=latest`.
/// If you need a decreasing all range, you can use `-1..`.
///
/// # Examples
///
/// ```
/// use twine_lib::{Cid, resolver::{RangeQuery, AbsoluteRange}};
/// let cid = Cid::default();
/// let latest = 10;
/// let range = RangeQuery::from((cid, 0..2)).to_absolute(latest).unwrap();
/// assert_eq!(range, AbsoluteRange::new(cid, 0, 1));
/// let range = RangeQuery::from((cid, 2..)).to_absolute(latest).unwrap();
/// assert_eq!(range, AbsoluteRange::new(cid, 2, 10));
/// let range = RangeQuery::from((cid, 4..=1)).to_absolute(latest).unwrap();
/// assert_eq!(range, AbsoluteRange::new(cid, 4, 1));
/// let range = RangeQuery::from((cid, ..=-2)).to_absolute(latest).unwrap();
/// assert_eq!(range, AbsoluteRange::new(cid, 0, 9));
/// let range = RangeQuery::from((cid, -1..-5)).to_absolute(latest).unwrap();
/// assert_eq!(range, AbsoluteRange::new(cid, 10, 7));
/// let range = RangeQuery::from((cid, -1..)).to_absolute(latest).unwrap();
/// assert_eq!(range, AbsoluteRange::new(cid, 10, 0));
/// let range = RangeQuery::from((cid, ..)).to_absolute(latest).unwrap();
/// assert_eq!(range, AbsoluteRange::new(cid, 0, 10));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum RangeQuery {
  /// An absolute range where the indices are known and constant
  Absolute(AbsoluteRange),
  /// A relative range where the indices can be relative to the latest index
  Relative(Cid, Bound<i64>, Bound<i64>),
}

impl RangeQuery {
  /// Create a new RangeQuery from a CID and a [`RangeBounds<i64>`]
  pub fn from_range_bounds<C: AsCid, T: RangeBounds<i64>>(strand: C, range: T) -> Self {
    let start = match range.start_bound() {
      Bound::Unbounded => Bound::Included(&0),
      bound @ _ => bound,
    };
    let neg_start = match start {
      Bound::Included(s) => s < &0,
      Bound::Excluded(s) => s < &0,
      _ => false,
    };
    let end = match range.end_bound() {
      Bound::Unbounded if neg_start => Bound::Included(&0),
      Bound::Unbounded => Bound::Included(&-1),
      bound @ _ => bound,
    };
    let neg_end = match end {
      Bound::Included(e) => e < &0,
      Bound::Excluded(e) => e < &0,
      _ => unreachable!(),
    };

    if neg_start || neg_end {
      Self::Relative(strand.as_cid().clone(), start.cloned(), end.cloned())
    } else {
      // 0, 0 is empty
      // 1, 0 is [0]
      // 0, 1 is [0]
      // 1, 1 is empty
      // larger number is always exclusive
      let (start, end) = match (start, end) {
        (Bound::Included(s), Bound::Included(e)) => {
          if e > s {
            (*s, *e)
          } else {
            (*s, *e)
          }
        }
        (Bound::Included(s), Bound::Excluded(e)) => {
          if e > s {
            (*s, e - 1)
          } else {
            (*s, e + 1)
          }
        }
        (Bound::Excluded(s), Bound::Included(e)) => {
          if e > s {
            (s + 1, *e)
          } else {
            (s - 1, *e)
          }
        }
        (Bound::Excluded(s), Bound::Excluded(e)) => {
          if e > s {
            (s + 1, e - 1)
          } else {
            (s - 1, e + 1)
          }
        }
        _ => unreachable!(),
      };

      Self::Absolute(AbsoluteRange::new(
        strand.as_cid().clone(),
        start as u64,
        end as u64,
      ))
    }
  }

  /// Convert the range to an absolute range given the latest index
  ///
  /// If the range is already absolute, it will be returned as is.
  ///
  /// If the range is empty, None will be returned.
  pub fn to_absolute(self, latest: u64) -> Option<AbsoluteRange> {
    match self {
      Self::Absolute(range) => Some(range),
      Self::Relative(cid, s, e) => {
        let dir = range_dir(
          match s {
            Bound::Included(s) | Bound::Excluded(s) => s,
            _ => unreachable!(),
          },
          match e {
            Bound::Included(e) | Bound::Excluded(e) => e,
            _ => unreachable!(),
          },
        );
        let e = e.map(|e| if e < 0 { latest as i64 + e + 1 } else { e });
        let e = match e {
          Bound::Included(e) => e,
          Bound::Excluded(e) => e - dir,
          _ => unreachable!(),
        };
        let s = s.map(|s| if s < 0 { latest as i64 + s + 1 } else { s });
        let s = match s {
          Bound::Included(s) => s,
          Bound::Excluded(s) => s + dir,
          _ => unreachable!(),
        };
        let l = latest as i64;
        // if the range is increasing, the start be less than the latest,
        // if the range is decreasing, the end must be less than the latest
        // both start and end must be greater than 0
        if (dir > 0 && s > l) || (dir < 0 && e > l) || (s < 0 && e < 0) {
          return None;
        }
        let range = if dir < 0 {
          AbsoluteRange::new(cid, s.max(e).max(0) as u64, e.max(0) as u64)
        } else {
          AbsoluteRange::new(cid, s.max(0) as u64, e.max(s).max(0) as u64)
        };
        Some(range)
      }
    }
  }

  /// Try to convert the range to an absolute range given a resolver
  ///
  /// # See also
  ///
  /// - [`RangeQuery::to_absolute`]
  /// - [`Resolver`]
  pub async fn try_to_absolute<R: Resolver>(
    self,
    resolver: &R,
  ) -> Result<Option<AbsoluteRange>, ResolutionError> {
    match self {
      Self::Absolute(range) => Ok(range.into()),
      Self::Relative(strand, _, _) => {
        let latest = resolver.resolve_latest(strand).await?.unpack().index();
        Ok(self.to_absolute(latest))
      }
    }
  }

  /// Convert the range to a stream of single queries
  pub fn to_stream<'a, R: Resolver>(
    self,
    resolver: &'a R,
  ) -> impl Stream<Item = Result<SingleQuery, ResolutionError>> + 'a {
    once(async move { self.try_to_absolute(resolver).await })
      .try_filter_map(|res| async move { Ok(res) })
      .map_ok(|result| futures::stream::iter(result.into_iter().map(Ok)))
      .try_flatten()
  }

  /// Convert the range to a stream of absolute ranges
  pub fn to_batch_stream<'a, R: Resolver>(
    self,
    resolver: &'a R,
    size: u64,
  ) -> impl Stream<Item = Result<AbsoluteRange, ResolutionError>> + 'a {
    use futures::stream::StreamExt;
    once(async move {
      self
        .try_to_absolute(resolver)
        .await
        .map(|result| match result {
          Some(result) => futures::stream::iter(result.batches(size)).map(Ok),
          None => futures::stream::iter(vec![].into_iter()).map(Ok),
        })
    })
    .try_flatten()
  }

  /// Check if the range is absolute
  pub fn is_absolute(&self) -> bool {
    matches!(self, Self::Absolute(_))
  }

  /// Get the strand CID of the range
  pub fn strand_cid(&self) -> &Cid {
    match self {
      Self::Absolute(range) => &range.strand,
      Self::Relative(strand, _, _) => strand,
    }
  }
}

impl From<AbsoluteRange> for RangeQuery {
  fn from(range: AbsoluteRange) -> Self {
    Self::Absolute(range)
  }
}

impl From<(Cid, i64, i64)> for RangeQuery {
  fn from((strand, upper, lower): (Cid, i64, i64)) -> Self {
    Self::Relative(strand, Bound::Included(upper), Bound::Included(lower))
  }
}

impl<C, R> From<(C, R)> for RangeQuery
where
  R: RangeBounds<i64>,
  C: AsCid,
{
  fn from((strand, range): (C, R)) -> Self {
    Self::from_range_bounds(strand.as_cid(), range)
  }
}

impl FromStr for RangeQuery {
  type Err = ConversionError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    fn index_from_str(s: &str) -> Result<i64, ConversionError> {
      if s == "latest" {
        return Ok(-1);
      }
      let s = s.parse()?;
      Ok(s)
    }

    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
      return Err(ConversionError::InvalidFormat(
        "Invalid range query string".to_string(),
      ));
    }
    let cid_str = parts.get(0).unwrap();
    let maybe_start = parts.get(1).unwrap();
    let maybe_end = parts.get(2).unwrap();
    let cid = Cid::try_from(*cid_str)?;
    match (*maybe_start, *maybe_end) {
      ("", "") => Ok((cid, ..).into()),
      (start, "") => {
        let start: i64 = index_from_str(start)?;
        Ok((cid, start..).into())
      }
      ("", end) => {
        let parts = end.split('=').collect::<Vec<_>>();
        if parts.len() == 2 {
          let end: i64 = index_from_str(parts[1])?;
          Ok((cid, ..=end).into())
        } else {
          let end: i64 = index_from_str(end)?;
          Ok((cid, ..end).into())
        }
      }
      (start, end) => {
        let start: i64 = index_from_str(start)?;
        let parts = end.split('=').collect::<Vec<_>>();
        if parts.len() == 2 {
          let end: i64 = index_from_str(parts[1])?;
          Ok((cid, start..=end).into())
        } else {
          let end: i64 = index_from_str(end)?;
          Ok((cid, start..end).into())
        }
      }
    }
  }
}

impl Display for RangeQuery {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RangeQuery::Absolute(range) => write!(f, "{}", range),
      RangeQuery::Relative(strand, start, end) => {
        let start = match start {
          Bound::Included(s) => s.to_string(),
          Bound::Unbounded => "".to_string(),
          Bound::Excluded(_) => unimplemented!("Excluded start bounds not supported"),
        };
        let end = match end {
          Bound::Included(e) => format!("={}", e),
          Bound::Unbounded => "".to_string(),
          Bound::Excluded(e) => e.to_string(),
        };
        write!(f, "{}:{}:{}", strand, start, end)
      }
    }
  }
}

/// A query that could be a strand, a single tixel, or a range of tixels
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnyQuery {
  /// A query for a strand
  Strand(Cid),
  /// A query for a single tixel
  One(SingleQuery),
  /// A query for a range of tixels
  Many(RangeQuery),
}

impl AnyQuery {
  /// Get the strand CID of the query
  pub fn strand_cid(&self) -> &Cid {
    match self {
      Self::Strand(cid) => cid,
      Self::One(query) => query.strand_cid(),
      Self::Many(range) => range.strand_cid(),
    }
  }

  /// Check if it's a strand query
  pub fn is_strand(&self) -> bool {
    matches!(self, Self::Strand(_))
  }

  /// Check if it's a single query
  pub fn is_one(&self) -> bool {
    matches!(self, Self::One(_))
  }

  /// Check if it's a range query
  pub fn is_many(&self) -> bool {
    matches!(self, Self::Many(_))
  }

  /// If the query is a range of length 1, reduce it to a single query
  pub fn reduce(self) -> Self {
    // if it's a range query with a single index, reduce it to a query
    match self {
      Self::Many(range) => {
        if let RangeQuery::Absolute(range) = range {
          if range.len() == 1 {
            return Self::One(SingleQuery::Index(range.strand, range.start as i64));
          }
        }
      }
      _ => {}
    }
    self
  }
}

impl From<SingleQuery> for AnyQuery {
  fn from(query: SingleQuery) -> Self {
    Self::One(query)
  }
}

impl From<RangeQuery> for AnyQuery {
  fn from(range: RangeQuery) -> Self {
    Self::Many(range).reduce()
  }
}

impl FromStr for AnyQuery {
  type Err = ConversionError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.split(':').count() {
      1 => {
        let cid = Cid::try_from(s.to_string())?;
        Ok(Self::Strand(cid))
      }
      2 => {
        let query = SingleQuery::from_str(s)?;
        Ok(Self::One(query))
      }
      3 => {
        let query = RangeQuery::from_str(s)?;
        Ok(Self::Many(query).reduce())
      }
      _ => Err(ConversionError::InvalidFormat(
        "Invalid query string".to_string(),
      )),
    }
  }
}

impl Display for AnyQuery {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      AnyQuery::Strand(cid) => write!(f, "{}", cid),
      AnyQuery::One(query) => write!(f, "{}", query),
      AnyQuery::Many(range) => write!(f, "{}", range),
    }
  }
}

impl From<(Cid, i64)> for AnyQuery {
  fn from((strand, index): (Cid, i64)) -> Self {
    SingleQuery::from((strand, index)).into()
  }
}

impl From<(Cid, Cid)> for AnyQuery {
  fn from((strand, tixel): (Cid, Cid)) -> Self {
    SingleQuery::from((strand, tixel)).into()
  }
}

impl From<Cid> for AnyQuery {
  fn from(cid: Cid) -> Self {
    Self::Strand(cid)
  }
}

impl From<(Cid, i64, i64)> for AnyQuery {
  fn from((strand, start, end): (Cid, i64, i64)) -> Self {
    RangeQuery::from((strand, start, end)).into()
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
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 0, 1)));
    let range = RangeQuery::from_range_bounds(&cid, 4..1);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 4, 2)));
    let range = RangeQuery::from_range_bounds(&cid, 2..=4);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 2, 4)));
    let range = RangeQuery::from_range_bounds(&cid, 3..=1);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 3, 1)));
    let range = RangeQuery::from_range_bounds(&cid, ..=2);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 0, 2)));
    let range = RangeQuery::from_range_bounds(&cid, 3..=0);
    assert_eq!(range, RangeQuery::Absolute(AbsoluteRange::new(cid, 3, 0)));
    let range = RangeQuery::from_range_bounds(&cid, -1..);
    assert_eq!(
      range,
      RangeQuery::Relative(cid, Bound::Included(-1), Bound::Included(0))
    );
    let range = RangeQuery::from_range_bounds(&cid, ..=-2);
    assert_eq!(
      range,
      RangeQuery::Relative(cid, Bound::Included(0), Bound::Included(-2))
    );
    let range = RangeQuery::from_range_bounds(&cid, ..);
    assert_eq!(
      range,
      RangeQuery::Relative(cid, Bound::Included(0), Bound::Included(-1))
    );
    let range = RangeQuery::from_range_bounds(&cid, 2..);
    assert_eq!(
      range,
      RangeQuery::Relative(cid, Bound::Included(2), Bound::Included(-1))
    );
    let range = RangeQuery::from_range_bounds(&cid, -1..-1);
    assert_eq!(
      range,
      RangeQuery::Relative(cid, Bound::Included(-1), Bound::Excluded(-1))
    );
    let range = RangeQuery::from_range_bounds(&cid, -1..=-2);
    assert_eq!(
      range,
      RangeQuery::Relative(cid, Bound::Included(-1), Bound::Included(-2))
    );
    let range = RangeQuery::from_range_bounds(&cid, -3..-1);
    assert_eq!(
      range,
      RangeQuery::Relative(cid, Bound::Included(-3), Bound::Excluded(-1))
    );
  }

  // -100..20 if latest is 100... would mean: 1..20
  // but the intention is a decreasing range. so it should be 21..21
  // 20..-100 would mean 20..1 which is also not what we want
  // since the intention is an increasing range, so it should be 20..20
  #[test]
  fn relative_range_edge_cases() {
    let latest = 100;
    let cid = Cid::default();
    let range: RangeQuery = (cid.clone(), -100..20).into();
    let absolute = range.to_absolute(latest).unwrap();
    assert_eq!(absolute, AbsoluteRange::new(cid, 21, 21));

    let range: RangeQuery = (cid.clone(), -1..=0).into();
    let absolute = range.to_absolute(latest).unwrap();
    assert_eq!(absolute, AbsoluteRange::new(cid, 100, 0));

    let range: RangeQuery = (cid.clone(), 20..-100).into();
    let absolute = range.to_absolute(latest).unwrap();
    assert_eq!(absolute, AbsoluteRange::new(cid, 20, 20));
  }

  #[test]
  fn test_iter() {
    let range = AbsoluteRange::new(Cid::default(), 0, 100);
    let queries = range.into_iter().collect::<Vec<_>>();
    assert_eq!(queries.len(), 101);
    assert_eq!(queries[0], SingleQuery::Index(Cid::default(), 0));
    assert_eq!(queries[100], SingleQuery::Index(Cid::default(), 100));

    let range = AbsoluteRange::new(Cid::default(), 100, 0);
    let queries = range.into_iter().collect::<Vec<_>>();
    assert_eq!(queries.len(), 101);
    assert_eq!(queries[0], SingleQuery::Index(Cid::default(), 100));
    assert_eq!(queries[100], SingleQuery::Index(Cid::default(), 0));
  }

  #[test]
  fn test_batches() {
    let range = AbsoluteRange::new(Cid::default(), 99, 0);
    let batches = range.batches(100);
    let cid = Cid::default();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0], AbsoluteRange::new(cid.clone(), 99, 0));

    let range = AbsoluteRange::new(Cid::default(), 0, 99);
    let batches = range.batches(100);
    let cid = Cid::default();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0], AbsoluteRange::new(cid.clone(), 0, 99));

    let range = AbsoluteRange::new(Cid::default(), 100, 0);
    let batches = range.batches(100);
    let cid = Cid::default();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0], AbsoluteRange::new(cid.clone(), 100, 1));
    assert_eq!(batches[1], AbsoluteRange::new(cid, 0, 0));

    let range = AbsoluteRange::new(Cid::default(), 0, 100);
    let batches = range.batches(100);
    let cid = Cid::default();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0], AbsoluteRange::new(cid.clone(), 0, 99));
    assert_eq!(batches[1], AbsoluteRange::new(cid, 100, 100));

    let range = AbsoluteRange::new(Cid::default(), 101, 0);
    let batches = range.batches(100);
    let cid = Cid::default();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0], AbsoluteRange::new(cid.clone(), 101, 2));
    assert_eq!(batches[1], AbsoluteRange::new(cid, 1, 0));

    let range = AbsoluteRange::new(Cid::default(), 0, 101);
    let batches = range.batches(100);
    let cid = Cid::default();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0], AbsoluteRange::new(cid.clone(), 0, 99));
    assert_eq!(batches[1], AbsoluteRange::new(cid, 100, 101));
  }

  #[test]
  fn test_to_absolute() {
    let range: RangeQuery = (Cid::default(), -1..=2).into();
    let absolute = range.to_absolute(10).unwrap();
    assert_eq!(absolute.start, 10);
    assert_eq!(absolute.end, 2);
    assert_eq!(absolute, AbsoluteRange::new(Cid::default(), 10, 2));

    let range: RangeQuery = (Cid::default(), -3..2).into();
    let absolute = range.to_absolute(10).unwrap();
    assert_eq!(absolute.start, 8);
    assert_eq!(absolute.end, 3);

    let range: RangeQuery = (Cid::default(), -3..-15).into();
    let absolute = range.to_absolute(10).unwrap();
    assert_eq!(absolute.start, 8);
    assert_eq!(absolute.end, 0);

    let range: RangeQuery = (Cid::default(), -15..-2).into();
    let absolute = range.to_absolute(10).unwrap();
    assert_eq!(absolute.start, 0);
    assert_eq!(absolute.end, 8);

    let range: RangeQuery = (Cid::default(), 5..-2).into();
    let absolute = range.to_absolute(10).unwrap();
    assert_eq!(absolute.start, 5);
    assert_eq!(absolute.end, 8);

    let range: RangeQuery = (Cid::default(), 5..=2).into();
    let absolute = range.to_absolute(10).unwrap();
    assert_eq!(absolute.start, 5);
    assert_eq!(absolute.end, 2);

    let range: RangeQuery = (Cid::default(), 5..).into();
    let absolute = range.to_absolute(10).unwrap();
    assert_eq!(absolute.start, 5);
    assert_eq!(absolute.end, 10);

    let range: RangeQuery = (Cid::default(), ..=2).into();
    let absolute = range.to_absolute(10).unwrap();
    assert_eq!(absolute.start, 0);
    assert_eq!(absolute.end, 2);

    let range: RangeQuery = (Cid::default(), ..).into();
    let absolute = range.to_absolute(10).unwrap();
    assert_eq!(absolute.start, 0);
    assert_eq!(absolute.end, 10);

    let range: RangeQuery = (Cid::default(), ..3).into();
    let absolute = range.to_absolute(0).unwrap();
    assert_eq!(absolute.start, 0);
    assert_eq!(absolute.end, 2);
  }

  #[test]
  fn test_to_absolute_empty() {
    let range: RangeQuery = (Cid::default(), 2..).into();
    let absolute = range.to_absolute(0);
    assert!(absolute.is_none());

    let range: RangeQuery = (Cid::default(), -1..3).into();
    let absolute = range.to_absolute(0);
    assert!(absolute.is_none());

    let range: RangeQuery = (Cid::default(), -15..-20).into();
    let absolute = range.to_absolute(10);
    assert!(absolute.is_none());
  }

  #[test]
  fn test_to_string_roundtrip_latest() {
    let cid = Cid::default().to_string();
    let output = cid.clone() + ":-1";
    let inputs = [
      cid.clone() + ":",
      cid.clone() + ":latest",
      cid.clone() + ":-1",
    ];
    for input in inputs.iter() {
      let range: SingleQuery = input.parse().unwrap();
      assert_eq!(range.to_string(), output);
    }
  }

  #[test]
  fn test_to_string_roundtrip_range() {
    let cid = Cid::default().to_string();
    let output = cid.clone() + ":0:=-1";
    let inputs = [
      cid.clone() + "::",
      cid.clone() + ":0:=-1",
      cid.clone() + ":0:",
      cid.clone() + "::=-1",
      cid.clone() + "::=latest",
    ];
    for input in inputs.iter() {
      let range: RangeQuery = input.parse().unwrap();
      assert_eq!(range.to_string(), output);
    }

    let output = cid.clone() + ":-1:=0";
    let inputs = [
      cid.clone() + ":latest:",
      cid.clone() + ":-1:",
      cid.clone() + ":latest:=0",
      cid.clone() + ":-1:=0",
    ];
    for input in inputs.iter() {
      dbg!(input);
      let range: RangeQuery = input.parse().unwrap();
      assert_eq!(range.to_string(), output);
    }
  }

  #[test]
  fn test_to_from_string() {
    let s = "bafyriqdik6t7lricocnj4gu7bcac2rk52566ff2qy7fcg2gxzzj5sjbl5kbera6lurzghkeoanrz73pqb4buzpvb7iy54j5opgvlxtpfhfune:0:=99";
    let range: RangeQuery = s.parse().unwrap();
    assert_eq!(range.to_absolute(0).unwrap().len(), 100);
    assert_eq!(&range.to_string(), s);

    let s = "bafyriqdik6t7lricocnj4gu7bcac2rk52566ff2qy7fcg2gxzzj5sjbl5kbera6lurzghkeoanrz73pqb4buzpvb7iy54j5opgvlxtpfhfune:99:=0";
    let range: RangeQuery = s.parse().unwrap();
    assert_eq!(range.to_absolute(0).unwrap().len(), 100);
    assert_eq!(&range.to_string(), s);

    let s = "bafyriqdik6t7lricocnj4gu7bcac2rk52566ff2qy7fcg2gxzzj5sjbl5kbera6lurzghkeoanrz73pqb4buzpvb7iy54j5opgvlxtpfhfune:-1:4";
    let range: RangeQuery = s.parse().unwrap();
    assert_eq!(&range.to_string(), s);
  }

  #[test]
  fn test_any_query() {
    let s = "bafyriqdik6t7lricocnj4gu7bcac2rk52566ff2qy7fcg2gxzzj5sjbl5kbera6lurzghkeoanrz73pqb4buzpvb7iy54j5opgvlxtpfhfune:0:=99";
    let query: AnyQuery = s.parse().unwrap();
    assert_eq!(query.to_string(), s);
    assert!(matches!(&query, AnyQuery::Many(_)));
    if let AnyQuery::Many(range) = query {
      assert_eq!(range.to_absolute(0).unwrap().len(), 100);
    }

    let s = "bafyriqdik6t7lricocnj4gu7bcac2rk52566ff2qy7fcg2gxzzj5sjbl5kbera6lurzghkeoanrz73pqb4buzpvb7iy54j5opgvlxtpfhfune:-1";
    let query: AnyQuery = s.parse().unwrap();
    assert_eq!(query.to_string(), s);
    assert!(matches!(&query, AnyQuery::One(_)));
    if let AnyQuery::One(query) = query {
      assert_eq!(query.to_string(), s);
      assert!(matches!(&query, SingleQuery::Latest(_)));
    }

    let s = "bafyriqdik6t7lricocnj4gu7bcac2rk52566ff2qy7fcg2gxzzj5sjbl5kbera6lurzghkeoanrz73pqb4buzpvb7iy54j5opgvlxtpfhfune";
    let query: AnyQuery = s.parse().unwrap();
    assert_eq!(query.to_string(), s);
    assert!(matches!(&query, AnyQuery::Strand(_)));
    if let AnyQuery::Strand(cid) = query {
      assert_eq!(cid.to_string(), s);
    }
  }
}
