#![doc = include_str!("../README.md")]

pub mod signer;
pub use signer::{Signer, SigningError};

pub mod builder;
pub use builder::TwineBuilder;

#[cfg(feature = "v1")]
pub use biscuit;
#[cfg(feature = "v1")]
mod biscuit_signer;
#[cfg(feature = "v1")]
pub use biscuit_signer::BiscuitSigner;

#[cfg(feature = "rustcrypto-signer")]
mod rustcrypto_signer;
#[cfg(feature = "rustcrypto-signer")]
pub use rustcrypto_signer::{RustCryptoSigner, RustCryptoSignerError};

#[cfg(feature = "ring-signer")]
mod ring_signer;
#[cfg(feature = "ring-signer")]
pub use ring_signer::RingSigner;

pub use pkcs8;
#[cfg(feature = "ring-signer")]
pub use ring;
