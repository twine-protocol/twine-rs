pub fn is_all_unique<T: Eq + std::hash::Hash, I: IntoIterator<Item = T>>(iter: I) -> bool {
  let mut seen = std::collections::HashSet::new();
  for item in iter {
    if !seen.insert(item) {
      return false;
    }
  }
  true
}
