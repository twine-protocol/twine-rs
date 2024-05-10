use crate::errors::StoreError;
use crate::resolver::Resolver;
use crate::twine::AnyTwine;
use crate::as_cid::AsCid;
use async_trait::async_trait;
use futures::stream::Stream;

#[async_trait]
pub trait Store: Resolver {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError>;
  async fn save_many<I: Into<AnyTwine> + Send, S: Iterator<Item = I> + Send, T: IntoIterator<Item = I, IntoIter = S> + Send>(&self, twines: T) -> Result<(), StoreError>;
  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send + Unpin>(&self, twines: T) -> Result<(), StoreError>;
  async fn delete<C: AsCid + Send>(&self, cid: C) -> Result<(), StoreError>;
}
