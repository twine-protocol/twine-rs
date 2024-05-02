//! Twine Core Library
//!
//! Docs...
//!
// pub(crate) mod serde_utils;

pub mod errors;
pub mod crypto;
pub mod as_cid;
pub mod twine;
pub mod verify;
// pub mod utils;
pub mod specification;
pub mod schemas;
pub mod resolver;

pub use semver;
pub use libipld;

pub mod prelude {
  pub use super::errors::VerificationError;
  pub use libipld::Cid;
  pub use super::twine::{Twine, AnyTwine, Strand, Tixel, Stitch, TwineBlock};
  pub use super::resolver::{Resolver, Query, ResolutionError};
  pub use super::as_cid::AsCid;
}
