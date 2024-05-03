use crate::resolver::Resolver;
use std::error::Error;
use crate::twine::AnyTwine;
use crate::as_cid::AsCid;
use async_trait::async_trait;

#[async_trait]
pub trait Store: Resolver {
  async fn save<T: Into<AnyTwine> + Send + Sync>(&mut self, twine: T) -> Result<(), Box<dyn Error>>;
  async fn save_many<T: Into<AnyTwine> + Send + Sync>(&mut self, twines: Vec<T>) -> Result<(), Box<dyn Error>>;
  async fn delete<C: AsCid + Send + Sync>(&mut self, cid: C) -> Result<(), Box<dyn Error>>;
}
