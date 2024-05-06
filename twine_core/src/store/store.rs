use crate::resolver::Resolver;
use std::error::Error;
use crate::twine::AnyTwine;
use crate::as_cid::AsCid;
use async_trait::async_trait;
use futures::stream::Stream;

#[async_trait]
pub trait Store: Resolver {
  async fn save<T: Into<AnyTwine> + Send + Sync>(&self, twine: T) -> Result<(), Box<dyn Error>>;
  async fn save_many<I: Into<AnyTwine> + Send + Sync, S: Iterator<Item = I> + Send + Sync, T: IntoIterator<Item = I, IntoIter = S> + Send + Sync>(&self, twines: T) -> Result<(), Box<dyn Error>>;
  async fn save_stream<I: Into<AnyTwine> + Send + Sync, T: Stream<Item = I> + Send + Sync + Unpin>(&self, twines: T) -> Result<(), Box<dyn Error>>;
  async fn delete<C: AsCid + Send + Sync>(&self, cid: C) -> Result<(), Box<dyn Error>>;
}
