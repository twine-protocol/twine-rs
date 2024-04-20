mod chain;
mod mixin;
mod pulse;

pub type V1 = crate::specification::Specification<1>;

impl Default for V1 {
  fn default() -> Self {
    Self("twine/1.0.x".to_string())
  }
}

pub use mixin::*;
pub use chain::*;
pub use pulse::*;
