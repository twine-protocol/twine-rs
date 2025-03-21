use crate::as_cid::AsCid;
use crate::errors::StoreError;
use crate::resolver::unchecked_base::BaseResolver;
use crate::resolver::MaybeSend;
use crate::twine::AnyTwine;
use async_trait::async_trait;
use futures::stream::Stream;

/// Methods for types that are able to save twine data
///
/// This provides a standard interface for saving. Anything
/// implementing Store also must implement [`BaseResolver`]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Store: BaseResolver {
  /// Save a single twine object
  ///
  /// Note: if saving a full [`Twine`](crate::twine::Twine) type, it will store
  /// the [`Tixel`](crate::tixel::Tixel) data. The strand data must be stored
  /// with its own call.
  ///
  /// # Example
  ///
  /// ```no_run
  /// # use twine_lib::{resolver::{Resolver, SingleQuery}, errors::StoreError, Cid};
  /// use twine_lib::store::Store;
  /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
  /// use twine_lib::store::MemoryStore;
  /// # let strand_cid = Cid::default();
  /// # let resolver = MemoryStore::default();
  /// let my_store = MemoryStore::default();
  /// // a twine from another source
  /// let twine = resolver.resolve_latest(strand_cid).await?.unpack();
  /// // save the strand first
  /// my_store.save(twine.strand().clone()).await?;
  /// // then save the tixel
  /// my_store.save(twine).await?;
  /// # Ok::<_, StoreError>(())
  /// });
  /// ```
  async fn save<T: Into<AnyTwine> + MaybeSend>(&self, twine: T) -> Result<(), StoreError>;
  /// Save many objects at once
  ///
  /// Different stores may implement this differently with respect to
  /// requirements of how they are ordered.
  async fn save_many<
    I: Into<AnyTwine> + MaybeSend,
    S: Iterator<Item = I> + MaybeSend,
    T: IntoIterator<Item = I, IntoIter = S> + MaybeSend,
  >(
    &self,
    twines: T,
  ) -> Result<(), StoreError>;
  /// Save many objects from a stream
  async fn save_stream<I: Into<AnyTwine> + MaybeSend, T: Stream<Item = I> + MaybeSend + Unpin>(
    &self,
    twines: T,
  ) -> Result<(), StoreError>;
  /// Delete an object
  ///
  /// Different stores may handle removal differently. Some stores
  /// may choose to keep orphaned data (if the strand is removed but not tixels).
  /// Others may choose to require pre-deletion of tixel data before the strand is
  /// removed.
  async fn delete<C: AsCid + MaybeSend>(&self, cid: C) -> Result<(), StoreError>;
}
