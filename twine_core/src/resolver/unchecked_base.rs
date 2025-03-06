use super::*;

#[cfg(target_arch = "wasm32")]
pub trait BaseResolverBounds {}

#[cfg(target_arch = "wasm32")]
impl<T> BaseResolverBounds for T {}

#[cfg(not(target_arch = "wasm32"))]
pub trait BaseResolverBounds: Send + Sync {}

#[cfg(not(target_arch = "wasm32"))]
impl<T> BaseResolverBounds for T where T: Send + Sync {}

#[cfg(target_arch = "wasm32")]
pub type TwineStream<'a, T> = Pin<Box<dyn Stream<Item = Result<T, ResolutionError>> + 'a>>;

#[cfg(not(target_arch = "wasm32"))]
pub type TwineStream<'a, T> = Pin<Box<dyn Stream<Item = Result<T, ResolutionError>> + Send + 'a>>;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait BaseResolver: BaseResolverBounds {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError>;
  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError>;
  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError>;
  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError>;
  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError>;
  async fn range_stream<'a>(
    &'a self,
    range: AbsoluteRange,
  ) -> Result<TwineStream<'a, Tixel>, ResolutionError>;
  async fn fetch_strands<'a>(&'a self) -> Result<TwineStream<'a, Strand>, ResolutionError>;
}
