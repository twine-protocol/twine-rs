//! SQLite store implementation for Twine
use super::{to_resolution_error, to_storage_error, Block};
use async_trait::async_trait;
use futures::stream::{unfold, Stream};
use futures::stream::{StreamExt, TryStreamExt};
use std::pin::Pin;
use twine_lib::as_cid::AsCid;
use twine_lib::errors::{ResolutionError, StoreError};
use twine_lib::resolver::unchecked_base::BaseResolver;
use twine_lib::resolver::AbsoluteRange;
use twine_lib::resolver::{unchecked_base, Resolver};
use twine_lib::store::Store;
use twine_lib::twine::{AnyTwine, TwineBlock};
use twine_lib::{
  twine::{Strand, Tixel},
  Cid,
};

/// The SQL schema for the SQLite store
pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS Strands (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  -- Cid bytes (2x varint (9) + 512bit hash (64)) = 18 + 64 = 82
  cid BINARY(82) UNIQUE NOT NULL,
  spec TEXT NOT NULL,
  data BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_strands_cid ON Strands (cid);

CREATE TABLE IF NOT EXISTS Tixels (
  cid BINARY(82) UNIQUE NOT NULL,
  strand INTEGER NOT NULL,
  idx INTEGER NOT NULL,
  data BLOB NOT NULL,

  -- Keys
  PRIMARY KEY (strand, idx),
  FOREIGN KEY (strand) REFERENCES Strands(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tixels_cid ON Tixels (cid);
"#;

/// A Sqlite store for Twine data
#[derive(Debug, Clone)]
pub struct SqliteStore {
  pool: sqlx::SqlitePool,
}

impl SqliteStore {
  /// Create a new Sqlite store from a sqlx pool
  pub fn new(pool: sqlx::SqlitePool) -> Self {
    Self { pool }
  }

  /// Open a new Sqlite store from a URI
  ///
  /// # Example
  ///
  /// ```no_run
  /// use twine_sql_store::sqlite::SqliteStore;
  /// # async {
  /// let store = SqliteStore::open("sqlite:my_database.db").await.unwrap();
  /// # };
  /// ```
  pub async fn open(uri: &str) -> Result<Self, sqlx::Error> {
    let pool = sqlx::Pool::connect(uri).await?;
    Ok(Self::new(pool))
  }

  /// Create the tables for the store
  ///
  /// This will create the necessary tables for the store if they do not already exist
  pub async fn create_tables(&self) -> Result<(), sqlx::Error> {
    let mut conn = self.pool.acquire().await?;
    sqlx::query(SCHEMA).execute(&mut *conn).await?;
    Ok(())
  }

  async fn all_strands(
    &self,
  ) -> Result<
    Pin<Box<dyn Stream<Item = Result<Strand, ResolutionError>> + Send + '_>>,
    ResolutionError,
  > {
    let query = "SELECT cid, data FROM Strands LIMIT 10 OFFSET $1";

    let stream = unfold(0, move |offset| async move {
      let mut conn = match self.pool.acquire().await.map_err(to_resolution_error) {
        Ok(conn) => conn,
        Err(e) => return Some((Err(e), offset)),
      };
      let strands: Result<Vec<_>, ResolutionError> = sqlx::query_as::<_, Block>(query)
        .bind(offset)
        .fetch(&mut *conn)
        .map_err(to_resolution_error)
        .map_ok(|(cid, data)| {
          let cid = Cid::try_from(cid).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
          Ok::<_, ResolutionError>(Strand::from_block(cid, data)?)
        })
        .try_collect()
        .await;
      if let Ok(strands) = &strands {
        if strands.is_empty() {
          return None;
        }
      }
      Some((strands, offset + 10))
    })
    .map_ok(|v| futures::stream::iter(v.into_iter()))
    .try_flatten()
    .boxed();

    Ok(stream)
  }

  async fn get_strand(&self, cid: &Cid) -> Result<Strand, ResolutionError> {
    let query = "SELECT cid, data FROM Strands WHERE cid = $1";

    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;

    let block: Block = sqlx::query_as(&query)
      .bind(cid.to_bytes())
      .fetch_one(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    let cid = Cid::try_from(block.0).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(Strand::from_block(cid, block.1)?)
  }

  async fn has_tixel(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    let query = "SELECT 1 FROM Tixels WHERE cid = $1 LIMIT 1";

    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;

    let exists: Option<i64> = sqlx::query_scalar(&query)
      .bind(cid.to_bytes())
      .fetch_optional(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    Ok(exists.is_some())
  }

  async fn has_strand_cid(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    let query = "SELECT 1 FROM Strands WHERE cid = $1 LIMIT 1";

    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;

    let exists: Option<i64> = sqlx::query_scalar(&query)
      .bind(cid.to_bytes())
      .fetch_optional(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    Ok(exists.is_some())
  }

  async fn cid_for_index(&self, strand: &Cid, index: u64) -> Result<Cid, ResolutionError> {
    let query = "SELECT t.cid FROM Tixels t JOIN Strands s ON t.strand = s.id WHERE s.cid = $1 AND t.idx = $2";

    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;

    let cid: Option<Vec<u8>> = sqlx::query_scalar(&query)
      .bind(strand.to_bytes())
      .bind(index as i64)
      .fetch_optional(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    if let Some(cid) = cid {
      Ok(Cid::try_from(cid).map_err(|e| ResolutionError::Fetch(e.to_string()))?)
    } else {
      Err(ResolutionError::NotFound)
    }
  }

  async fn get_tixel(&self, cid: &Cid) -> Result<Tixel, ResolutionError> {
    let query = "SELECT cid, data FROM Tixels WHERE cid = $1";

    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;

    let block: Block = sqlx::query_as(&query)
      .bind(cid.to_bytes())
      .fetch_one(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    let cid = Cid::try_from(block.0).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(Tixel::from_block(cid, block.1)?)
  }

  async fn get_tixel_by_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
    let query = "SELECT t.cid, t.data FROM Tixels t JOIN Strands s ON t.strand = s.id WHERE s.cid = $1 AND t.idx = $2";

    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;

    let block: Block = sqlx::query_as(&query)
      .bind(strand.to_bytes())
      .bind(index as i64)
      .fetch_one(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    let cid = Cid::try_from(block.0).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(Tixel::from_block(cid, block.1)?)
  }

  async fn latest_tixel(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
    let query = "SELECT t.cid, t.data FROM Tixels t JOIN Strands s ON t.strand = s.id WHERE s.cid = $1 ORDER BY t.idx DESC LIMIT 1";

    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;

    let block: Block = sqlx::query_as(&query)
      .bind(strand.to_bytes())
      .fetch_one(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    let cid = Cid::try_from(block.0).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(Tixel::from_block(cid, block.1)?)
  }

  async fn save_strand(&self, strand: &Strand) -> Result<(), StoreError> {
    let mut conn = self.pool.acquire().await.map_err(to_storage_error)?;

    let query = "INSERT OR IGNORE INTO Strands (cid, data, spec) VALUES ($1, $2, $3)";

    let cid = strand.cid().to_bytes();
    let data = strand.bytes().to_vec();

    let _ret = sqlx::query(&query)
      .bind(&cid)
      .bind(&data)
      .bind(strand.spec_str())
      .execute(&mut *conn)
      .await
      .map_err(to_storage_error)?;

    Ok(())
  }

  async fn save_tixel(&self, tixel: &Tixel) -> Result<(), StoreError> {
    let mut conn = self.pool.acquire().await.map_err(to_storage_error)?;

    // Ensure that the previous tixel exists
    let previous_exists = if tixel.index() == 0 {
      self.has_strand(&tixel.strand_cid()).await?
    } else {
      self.has_tixel(&tixel.previous().unwrap().tixel).await?
    };

    if !previous_exists {
      return Err(StoreError::Saving(
        "Previous tixel does not exist in store".to_string(),
      ));
    }

    let query = "
      INSERT OR IGNORE INTO Tixels (cid, data, strand, idx)
      SELECT $1, $2, s.id, $4 FROM Strands s
      WHERE s.cid = $3 AND
      ($4 = 0 OR EXISTS (
        SELECT 1 FROM Tixels t WHERE t.strand = s.id AND t.idx = $4 - 1
      ));
    ";

    let cid = tixel.cid().to_bytes();
    let data = tixel.bytes().to_vec();

    let _ret = sqlx::query(&query)
      .bind(&cid)
      .bind(&data)
      .bind(tixel.strand_cid().to_bytes())
      .bind(tixel.index() as i64)
      .execute(&mut *conn)
      .await
      .map_err(to_storage_error)?;

    Ok(())
  }

  async fn remove_strand(&self, cid: &Cid) -> Result<(), StoreError> {
    let query = "DELETE FROM Strands WHERE cid = $1";

    let mut conn = self.pool.acquire().await.map_err(to_storage_error)?;

    let _ret = sqlx::query(&query)
      .bind(cid.to_bytes())
      .execute(&mut *conn)
      .await
      .map_err(to_storage_error)?;

    Ok(())
  }

  async fn remove_tixel_if_latest(&self, cid: &Cid) -> Result<(), StoreError> {
    let query = "DELETE FROM Tixels WHERE cid = $1 AND idx = (SELECT MAX(idx) FROM Tixels WHERE strand = Tixels.strand)";

    let mut conn = self.pool.acquire().await.map_err(to_storage_error)?;

    let _ret = sqlx::query(&query)
      .bind(cid.to_bytes())
      .execute(&mut *conn)
      .await
      .map_err(to_storage_error)?;

    Ok(())
  }
}

#[async_trait]
impl unchecked_base::BaseResolver for SqliteStore {
  async fn fetch_strands(
    &self,
  ) -> Result<
    Pin<Box<dyn Stream<Item = Result<Strand, ResolutionError>> + Send + '_>>,
    ResolutionError,
  > {
    self.all_strands().await
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    self.has_strand_cid(cid).await
  }

  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    self
      .cid_for_index(strand, index)
      .await
      .map(|_| true)
      .or_else(|e| {
        if let ResolutionError::NotFound = e {
          Ok(false)
        } else {
          Err(e)
        }
      })
  }

  async fn has_twine(&self, _strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    self.has_tixel(cid).await
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
    self.get_strand(strand).await
  }

  async fn fetch_tixel(&self, _strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
    self.get_tixel(tixel).await
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
    self.get_tixel_by_index(strand, index).await
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
    self.latest_tixel(strand).await
  }

  async fn range_stream(
    &self,
    range: AbsoluteRange,
  ) -> Result<
    Pin<Box<dyn Stream<Item = Result<Tixel, ResolutionError>> + Send + '_>>,
    ResolutionError,
  > {
    let batches = range.batches(100);
    let stream = unfold(batches.into_iter(), move |mut batches| async move {
      let batch = batches.next()?;
      let mut conn = match self.pool.acquire().await.map_err(to_resolution_error) {
        Ok(conn) => conn,
        Err(e) => return Some((Err(e), batches)),
      };
      let dir = if range.is_increasing() { "ASC" } else { "DESC" };
      let tixels: Result<Vec<_>, ResolutionError> = sqlx::query_as::<_, Block>(&format!(
        "
          SELECT t.cid, t.data
          FROM Tixels t JOIN Strands s ON t.strand = s.id
          WHERE s.cid = $1 AND t.idx >= $2 AND t.idx <= $3
          ORDER BY t.idx {}
        ",
        dir
      ))
      .bind(range.strand.to_bytes())
      .bind(batch.lower() as i64)
      .bind(batch.upper() as i64)
      .fetch(&mut *conn)
      .map_err(to_resolution_error)
      .map_ok(|(cid, data)| {
        let cid = Cid::try_from(cid).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        Ok::<_, ResolutionError>(Tixel::from_block(cid, data)?)
      })
      .try_collect()
      .await;
      Some((tixels, batches))
    })
    .map_ok(|v| futures::stream::iter(v.into_iter()))
    .try_flatten()
    .boxed();

    Ok(stream)
  }
}

impl Resolver for SqliteStore {}

#[async_trait]
impl Store for SqliteStore {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
    match twine.into() {
      AnyTwine::Tixel(t) => self.save_tixel(&t).await,
      AnyTwine::Strand(s) => self.save_strand(&s).await,
    }
  }

  async fn save_many<
    I: Into<AnyTwine> + Send,
    S: Iterator<Item = I> + Send,
    T: IntoIterator<Item = I, IntoIter = S> + Send,
  >(
    &self,
    twines: T,
  ) -> Result<(), StoreError> {
    for twine in twines {
      self.save(twine).await?;
    }
    Ok(())
  }

  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send + Unpin>(
    &self,
    twines: T,
  ) -> Result<(), StoreError> {
    twines
      .chunks(100)
      .then(|chunk| self.save_many(chunk))
      .try_for_each(|_| async { Ok(()) })
      .await?;
    Ok(())
  }

  async fn delete<C: AsCid + Send>(&self, cid: C) -> Result<(), StoreError> {
    if self.has_strand_cid(cid.as_cid()).await? {
      self.remove_strand(cid.as_cid()).await
    } else if self.has_tixel(cid.as_cid()).await? {
      self.remove_tixel_if_latest(cid.as_cid()).await
    } else {
      Ok(())
    }
  }
}
