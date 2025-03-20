use super::*;

/// Abstract trait that helps place conditional bounds of Send and Sync
#[cfg(target_arch = "wasm32")]
pub trait BaseResolverBounds {}

/// Abstract trait that helps place conditional bounds of Send and Sync
#[cfg(target_arch = "wasm32")]
impl<T> BaseResolverBounds for T {}

/// Abstract trait that helps place conditional bounds of Send and Sync
#[cfg(not(target_arch = "wasm32"))]
pub trait BaseResolverBounds: Send + Sync {}

#[cfg(not(target_arch = "wasm32"))]
impl<T> BaseResolverBounds for T where T: Send + Sync {}

/// A stream of Twine objects
#[cfg(target_arch = "wasm32")]
pub type TwineStream<'a, T> = Pin<Box<dyn Stream<Item = Result<T, ResolutionError>> + 'a>>;

/// A stream of Twine objects
#[cfg(not(target_arch = "wasm32"))]
pub type TwineStream<'a, T> = Pin<Box<dyn Stream<Item = Result<T, ResolutionError>> + Send + 'a>>;

/// The base trait for the Twine Resolver
///
/// This trait is kept behind the module [`crate::resolver::unchecked_base`]
/// to signify that it is not meant to be used directly by the end user.
/// This is because the methods are not guaranteed to handle all consistency
/// checks.
///
/// On wasm32, the trait does not have the Send and Sync bounds, as currently
/// web assembly does not support multi-threading generally.
///
/// The [`crate::resolver::Resolver`] trait is the one that should be used
/// by the end user.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait BaseResolver: BaseResolverBounds {
  /// Check if a Tixel of a given index for a Strand is available
  /// The Strand must also be available
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError>;
  /// Check if a Tixel of a given CID for a Strand is available
  /// The Strand must also be available
  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError>;
  /// Check if a Strand is available
  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError>;
  /// Fetch the latest Tixel of a Strand
  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError>;
  /// Fetch a Tixel of a given index for a Strand
  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError>;
  /// Fetch a Tixel of a given CID for a Strand
  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError>;
  /// Fetch a Strand
  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError>;
  /// Get a stream of Tixels for a given range of a Strand
  async fn range_stream<'a>(
    &'a self,
    range: AbsoluteRange,
  ) -> Result<TwineStream<'a, Tixel>, ResolutionError>;
  /// Get a stream of all Strands
  async fn fetch_strands<'a>(&'a self) -> Result<TwineStream<'a, Strand>, ResolutionError>;
}
