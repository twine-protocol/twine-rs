//! Utilities for working with skiplists

/// Get the highest layer for which this (tixel) index
/// is an anchor for.
/// For example: in base 10, for the following indicies...
///
/// # Examples
///
/// ```
/// use twine_lib::skiplist::get_layer_pos;
/// assert_eq!(get_layer_pos(10, 1560), 1); // (multiple of 10)
/// assert_eq!(get_layer_pos(10, 1264), 0); // (NOT a multiple of 10)
/// assert_eq!(get_layer_pos(10, 3000), 3); // (multiple of 1000)
/// assert_eq!(get_layer_pos(10, 3700), 2); // (multiple 100)
/// ```
pub fn get_layer_pos(radix: u8, index: u64) -> usize {
  if index == 0 {
    return 0;
  }

  if radix == 1 {
    return index as usize;
  }

  let mut m = radix as u64;
  let mut result = 1;

  while index % m == 0 {
    m *= radix as u64;
    result += 1;
  }

  return result - 1;
}

/// A utility for getting an iterator of indices that can be used to skip through a strand.
///
/// This can either provide the tixel indices themselves or a list of
/// array indices of the links list for each tixel along the path.
/// This will not include the from/to indices themselves.
/// A radix of 1 doesn't make sense since `1^r` is always `1`.
/// A radix of 0 is interpreted as no radix skipping, so the list
/// just has the previous tixel cid, therefore a radix 0 skiplist
/// is just a decreasing list of tixel indices.
///
/// # Examples
///
/// ```
/// use twine_lib::skiplist::SkipList;
/// let radix = 10;
/// let from_index = 23;
/// let to_index = 5;
/// let actual: Vec<u64> = SkipList::new(radix, from_index, to_index, false).into_iter().collect();
/// assert_eq!(actual, vec![20, 10, 9, 8, 7, 6]);
/// // because... 23 is in the `n * 10^1` range so it's links list should have `[22, 20]`
/// // same deal for 20 which has links `[19, 10]` so we can skip to 10
/// // then we get to the `n * 10^0` range and we can skip to 9, 8, 7, 6
/// // The array indices for this correspond to jumps of
/// // `10^1`, `10^1`, `10^0`, `10^0`, `10^0`, `10^0`
/// // so the array indices would be `[1, 1, 0, 0, 0, 0]
/// let actual: Vec<u64> = SkipList::new(10, 23, 5, true).into_iter().collect();
/// assert_eq!(actual, vec![1, 1, 0, 0, 0, 0]);
/// ```
pub struct SkipList {
  radix: u64,
  from_index: u64,
  to_index: u64,
  by_link: bool,
}

impl SkipList {
  /// Create a new SkipList
  ///
  /// # Arguments
  ///
  /// * `radix` - The radix to use for skipping
  /// * `from_index` - The index to start from
  /// * `to_index` - The index to stop at
  /// * `by_link` - Whether to return the array indices of the stitch list
  pub fn new(radix: u8, from_index: u64, to_index: u64, by_link: bool) -> Self {
    let radix = radix as u64;

    if radix == 1 {
      panic!("Invalid radix");
    }

    if to_index >= from_index {
      panic!("Invalid range");
    }

    Self {
      radix,
      from_index,
      to_index,
      by_link,
    }
  }
}

impl IntoIterator for SkipList {
  type Item = u64;
  type IntoIter = SkipListIter;

  fn into_iter(self) -> Self::IntoIter {
    SkipListIter::new(self.radix, self.from_index, self.to_index, self.by_link)
  }
}

/// An iterator for the SkipList
pub struct SkipListIter {
  radix: u64,
  curr: u64,
  to_index: u64,
  from_index: u64,
  by_link: bool,
  q: u32,
  pow: u64,
  starter: Option<u64>,
}

impl SkipListIter {
  /// Create a new SkipListIter
  ///
  /// Instead of calling this directly, use `SkipList::into_iter()`
  pub fn new(radix: u64, from_index: u64, to_index: u64, by_link: bool) -> Self {
    let diff = from_index - to_index;
    let startq = (diff as f64).log(radix as f64).floor() as u32;
    let curr = (from_index as f64 / radix.pow(startq) as f64).floor() as u64 * radix.pow(startq);
    let starter = if curr != from_index {
      if by_link {
        Some(startq as u64)
      } else {
        Some(curr)
      }
    } else {
      None
    };

    Self {
      radix,
      curr,
      to_index,
      from_index,
      by_link,
      q: startq,
      pow: radix.pow(startq),
      starter,
    }
  }
}

impl Iterator for SkipListIter {
  type Item = u64;

  fn next(&mut self) -> Option<Self::Item> {
    if self.to_index >= self.from_index {
      return None;
    }

    if let Some(starter) = self.starter {
      self.starter = None;
      return Some(starter);
    }

    if self.curr < self.to_index {
      return None;
    }

    while self.curr - self.pow <= self.to_index {
      if self.q == 0 {
        return None;
      }
      self.q -= 1;
      self.pow = self.radix.pow(self.q);
    }

    self.curr -= self.pow;

    if self.by_link {
      Some(self.q as u64)
    } else {
      Some(self.curr)
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_skip_list() {
    let actual: Vec<u64> = SkipList::new(0, 10, 1, false).into_iter().collect();
    assert_eq!(actual, vec![9, 8, 7, 6, 5, 4, 3, 2]);

    let actual: Vec<u64> = SkipList::new(10, 23, 5, false).into_iter().collect();
    assert_eq!(actual, vec![20, 10, 9, 8, 7, 6]);

    let actual: Vec<u64> = SkipList::new(10, 23, 5, true).into_iter().collect();
    assert_eq!(actual, vec![1, 1, 0, 0, 0, 0]);

    let actual: Vec<u64> = SkipList::new(0, 10, 1, true).into_iter().collect();
    assert_eq!(actual, vec![0, 0, 0, 0, 0, 0, 0, 0]);

    let actual: Vec<u64> = SkipList::new(32, 30, 21, false).into_iter().collect();
    assert_eq!(actual, vec![29, 28, 27, 26, 25, 24, 23, 22]);

    let actual: Vec<u64> = SkipList::new(32, 30, 21, true).into_iter().collect();
    assert_eq!(actual, vec![0, 0, 0, 0, 0, 0, 0, 0]);

    let actual: Vec<u64> = SkipList::new(2, 10, 1, false).into_iter().collect();
    assert_eq!(actual, vec![8, 4, 2]);

    let actual: Vec<u64> = SkipList::new(2, 10, 1, true).into_iter().collect();
    assert_eq!(actual, vec![3, 2, 1]);
  }
}
