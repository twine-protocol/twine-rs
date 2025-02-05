use async_trait::async_trait;
use futures::stream::Stream;
use twine_core::as_cid::AsCid;
use twine_core::twine::AnyTwine;
use std::sync::Arc;
use twine_core::errors::{ResolutionError, StoreError};
use twine_core::{twine::{Strand, Tixel}, Cid};
use twine_core::resolver::{unchecked_base, Resolver};
use twine_core::store::Store;
use twine_core::resolver::AbsoluteRange;

pub use sqlx;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "mysql")]
pub mod mysql;

type Block = (Vec<u8>, Vec<u8>);

fn to_resolution_error(err: sqlx::Error) -> ResolutionError {
  match err {
    sqlx::Error::RowNotFound => ResolutionError::NotFound,
    _ => ResolutionError::Fetch(err.to_string())
  }
}

fn to_storage_error(err: sqlx::Error) -> StoreError {
  StoreError::Saving(err.to_string())
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SqlStore {
  #[cfg(feature = "sqlite")]
  Sqlite(sqlite::SqliteStore),
  #[cfg(feature = "mysql")]
  Mysql(mysql::MysqlStore),
  //...
}

impl SqlStore {
  pub async fn open(uri: &str) -> Result<Self, sqlx::Error> {
    #[cfg(feature = "sqlite")]
    {
      if uri.starts_with("sqlite:") {
        return Ok(SqlStore::Sqlite(sqlite::SqliteStore::open(uri).await?));
      }
    }
    #[cfg(feature = "mysql")]
    {
      if uri.starts_with("mysql:") {
        return Ok(SqlStore::Mysql(mysql::MysqlStore::open(uri).await?));
      }
    }
    unimplemented!("unsupported uri: {}", uri);
  }

  pub async fn create_sqlite_tables(&self) -> Result<(), sqlx::Error> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.create_tables().await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }
}

#[async_trait]
impl unchecked_base::BaseResolver for SqlStore {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.has_index(strand, index).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.has_index(strand, index).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.has_twine(strand, cid).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.has_twine(strand, cid).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.has_strand(cid).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.has_strand(cid).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.fetch_latest(strand).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.fetch_latest(strand).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.fetch_index(strand, index).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.fetch_index(strand, index).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.fetch_tixel(strand, tixel).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.fetch_tixel(strand, tixel).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.fetch_strand(strand).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.fetch_strand(strand).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn range_stream<'a>(&'a self, range: AbsoluteRange) -> Result<unchecked_base::TwineStream<'a, Arc<Tixel>>, ResolutionError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.range_stream(range).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.range_stream(range).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn fetch_strands<'a>(&'a self) -> Result<unchecked_base::TwineStream<'a, Arc<Strand>>, ResolutionError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.fetch_strands().await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.fetch_strands().await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }
}

impl Resolver for SqlStore {}

#[async_trait]
impl Store for SqlStore {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.save(twine).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.save(twine).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn save_many<I: Into<AnyTwine> + Send, S: Iterator<Item = I> + Send, T: IntoIterator<Item = I, IntoIter = S> + Send>(&self, twines: T) -> Result<(), StoreError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.save_many(twines).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.save_many(twines).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send + Unpin>(&self, twines: T) -> Result<(), StoreError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.save_stream(twines).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.save_stream(twines).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }

  async fn delete<C: AsCid + Send>(&self, cid: C) -> Result<(), StoreError> {
    match self {
      #[cfg(feature = "sqlite")]
      SqlStore::Sqlite(store) => store.delete(cid).await,
      #[cfg(feature = "mysql")]
      SqlStore::Mysql(store) => store.delete(cid).await,
      #[allow(unreachable_patterns)]
      _ => unimplemented!(),
    }
  }
}
