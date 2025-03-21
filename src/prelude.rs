//! Commonly used types and traits for working with Twine
//!
//! This module re-exports the most commonly used types and traits from the [`twine_lib`] crate.
//! Also re-exports types from the [`twine_builder`] crate if the `build` feature is enabled.
#[doc(no_inline)]
pub use twine_lib::as_cid::AsCid;
#[doc(no_inline)]
pub use twine_lib::errors::*;
#[doc(no_inline)]
pub use twine_lib::resolver::*;
#[doc(no_inline)]
pub use twine_lib::store::*;
#[doc(no_inline)]
pub use twine_lib::twine::{AnyTwine, Stitch, Strand, Tixel, Twine, TwineBlock};
#[doc(no_inline)]
pub use twine_lib::{Cid, Ipld};

#[doc(no_inline)]
#[cfg(feature = "build")]
pub use twine_builder::{builder::BuildError, signer::SigningError, Signer, TwineBuilder};
