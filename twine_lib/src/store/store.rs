use crate::as_cid::AsCid;
use crate::errors::StoreError;
use crate::resolver::unchecked_base::BaseResolver;
use crate::resolver::MaybeSend;
use crate::twine::AnyTwine;
use async_trait::async_trait;
use futures::stream::Stream;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Store: BaseResolver {
  async fn save<T: Into<AnyTwine> + MaybeSend>(&self, twine: T) -> Result<(), StoreError>;
  async fn save_many<
    I: Into<AnyTwine> + MaybeSend,
    S: Iterator<Item = I> + MaybeSend,
    T: IntoIterator<Item = I, IntoIter = S> + MaybeSend,
  >(
    &self,
    twines: T,
  ) -> Result<(), StoreError>;
  async fn save_stream<I: Into<AnyTwine> + MaybeSend, T: Stream<Item = I> + MaybeSend + Unpin>(
    &self,
    twines: T,
  ) -> Result<(), StoreError>;
  async fn delete<C: AsCid + MaybeSend>(&self, cid: C) -> Result<(), StoreError>;
}
