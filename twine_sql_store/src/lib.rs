use async_trait::async_trait;
use futures::stream::{unfold, Stream};
use futures::stream::{StreamExt, TryStreamExt};
use sqlx::any::install_default_drivers;
use twine_core::as_cid::AsCid;
use twine_core::twine::{AnyTwine, TwineBlock};
use std::pin::Pin;
use std::sync::Arc;
use twine_core::errors::{ResolutionError, StoreError};
use twine_core::{twine::{Strand, Tixel}, Cid};
use twine_core::resolver::{unchecked_base, Resolver};
use twine_core::store::Store;
use twine_core::resolver::AbsoluteRange;

pub use sqlx;

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

type Block = (Vec<u8>, Vec<u8>);

fn to_resolution_error(err: sqlx::Error) -> ResolutionError {
  match err {
    sqlx::Error::RowNotFound => ResolutionError::NotFound,
    _ => ResolutionError::Fetch(err.to_string())
  }
}

#[derive(Debug, Clone)]
pub struct SqlStore {
  pool: sqlx::Pool<sqlx::Any>,
}

impl SqlStore {
  pub fn new(pool: sqlx::Pool<sqlx::Any>) -> Self {
    Self { pool }
  }

  pub async fn open(uri: &str) -> Result<Self, sqlx::Error> {
    install_default_drivers();
    let pool = sqlx::any::AnyPoolOptions::new().connect(uri).await?;
    Ok(Self::new(pool))
  }

  pub async fn create_sqlite_tables(&self) -> Result<(), sqlx::Error> {
    let mut conn = self.pool.acquire().await?;
    sqlx::query(SCHEMA).execute(&mut *conn).await?;
    Ok(())
  }

  async fn all_strands(&self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + '_>>, ResolutionError> {
    // pull 10 at a time then flatten
    let stream = unfold(0, move |offset|{
      async move {
        let mut conn = match self.pool.acquire().await.map_err(to_resolution_error) {
          Ok(conn) => conn,
          Err(e) => return Some((Err(e), offset)),
        };
        let strands: Result<Vec<_>, ResolutionError> = sqlx::query_as::<_, Block>("SELECT cid, data FROM Strands LIMIT 10 OFFSET $1")
          .bind(offset)
          .fetch(&mut *conn)
          .map_err(to_resolution_error)
          .map_ok(|(cid, data)| {
            let cid = Cid::try_from(cid).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
            Ok::<_, ResolutionError>(Arc::new(Strand::from_block(cid, data)?))
          })
          .try_collect()
          .await;
        if let Ok(strands) = &strands {
          if strands.is_empty() {
            return None;
          }
        }
        Some((strands, offset + 10))
      }
    })
    .map_ok(|v| futures::stream::iter(v.into_iter()))
    .try_flatten()
    .boxed();

    Ok(stream)
  }

  async fn get_strand(&self, cid: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let block: Block = sqlx::query_as("
      SELECT cid, data FROM Strands WHERE cid = $1
    ")
      .bind(cid.to_bytes())
      .fetch_one(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    let cid = Cid::try_from(block.0).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(Arc::new(Strand::from_block(cid, block.1)?))
  }

  async fn has_tixel(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let exists: Option<i64> = sqlx::query_scalar("
      SELECT 1 FROM Tixels WHERE cid = $1 LIMIT 1
    ")
      .bind(cid.to_bytes())
      .fetch_optional(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    Ok(exists.is_some())
  }

  async fn has_strand_cid(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let exists: Option<i64> = sqlx::query_scalar("
      SELECT 1 FROM Strands WHERE cid = $1 LIMIT 1
    ")
      .bind(cid.to_bytes())
      .fetch_optional(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    Ok(exists.is_some())
  }

  async fn cid_for_index(&self, strand: &Cid, index: u64) -> Result<Cid, ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let cid: Option<Vec<u8>> = sqlx::query_scalar("
      SELECT t.cid
      FROM Tixels t
      JOIN Strands s ON t.strand = s.id
      WHERE s.cid = $1 AND t.idx = $2
    ")
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

  async fn get_tixel(&self, cid: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let block: Block = sqlx::query_as("
      SELECT cid, data FROM Tixels WHERE cid = $1
    ")
      .bind(cid.to_bytes())
      .fetch_one(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    let cid = Cid::try_from(block.0).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(Arc::new(Tixel::from_block(cid, block.1)?))
  }

  async fn get_tixel_by_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let block: Block = sqlx::query_as("
      SELECT t.cid, t.data
      FROM Tixels t
      JOIN Strands s ON t.strand = s.id
      WHERE s.cid = $1 AND t.idx = $2
    ")
      .bind(strand.to_bytes())
      .bind(index as i64)
      .fetch_one(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    let cid = Cid::try_from(block.0).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(Arc::new(Tixel::from_block(cid, block.1)?))
  }

  async fn latest_tixel(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let block: Block = sqlx::query_as("
      SELECT t.cid, t.data
      FROM Tixels t
      JOIN Strands s ON t.strand = s.id
      WHERE s.cid = $1
      ORDER BY t.idx DESC
      LIMIT 1;
    ")
      .bind(strand.to_bytes())
      .fetch_one(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    let cid = Cid::try_from(block.0).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    Ok(Arc::new(Tixel::from_block(cid, block.1)?))
  }

  async fn save_strand(&self, strand: &Strand) -> Result<(), ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let cid = strand.cid().to_bytes();
    let data = strand.bytes().to_vec();

    let _ret = sqlx::query("
      INSERT OR IGNORE INTO Strands (cid, data, spec) VALUES ($1, $2, $3);
    ")
      .bind(&cid)
      .bind(&data)
      .bind(strand.spec_str())
      .execute(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    Ok(())
  }

  async fn save_tixel(&self, tixel: &Tixel) -> Result<(), ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let cid = tixel.cid().to_bytes();
    let data = tixel.bytes().to_vec();

    let _ret = sqlx::query("
      INSERT OR IGNORE INTO Tixels (cid, data, strand, idx)
      SELECT $1, $2, s.id, $4
      FROM Strands s
      WHERE s.cid = $3
      AND ($4 = 0 OR EXISTS (
        SELECT 1 FROM Tixels
        WHERE strand = s.id
        AND idx = $4 - 1
      ));
    ")
      .bind(&cid)
      .bind(&data)
      .bind(tixel.strand_cid().to_bytes())
      .bind(tixel.index() as i64)
      .execute(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    Ok(())
  }

  async fn remove_strand(&self, cid: &Cid) -> Result<(), ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let _ret = sqlx::query("
      DELETE FROM Strands WHERE cid = $1
    ")
      .bind(cid.to_bytes())
      .execute(&mut *conn)
      .await
      .map_err(to_resolution_error)?;

    Ok(())
  }

  async fn remove_tixel_if_latest(&self, cid: &Cid) -> Result<(), ResolutionError> {
    let mut conn = self.pool.acquire().await.map_err(to_resolution_error)?;
    let _ret = sqlx::query("
        DELETE FROM Tixels
        WHERE cid = ?
        AND idx = (
          SELECT MAX(idx)
          FROM Tixels
          WHERE strand = Tixels.strand
        );
    ")
    .bind(cid.to_bytes())
    .execute(&mut *conn)
    .await
    .map_err(to_resolution_error)?;

    Ok(())
  }
}

#[async_trait]
impl unchecked_base::BaseResolver for SqlStore {

  async fn fetch_strands(&self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + '_>>, ResolutionError> {
    self.all_strands().await
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    self.has_strand_cid(cid).await
  }

  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    self.cid_for_index(strand, index).await.map(|_| true).or_else(|e| {
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

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    self.get_strand(strand).await
  }

  async fn fetch_tixel(&self, _strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    self.get_tixel(tixel).await
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    self.get_tixel_by_index(strand, index).await
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    self.latest_tixel(strand).await
  }

  async fn range_stream(&self, range: AbsoluteRange) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + '_>>, ResolutionError> {
    let batches = range.batches(100);
    let stream = unfold(batches.into_iter(), move |mut batches| {
      async move {
        let batch = batches.next()?;
        let mut conn = match self.pool.acquire().await.map_err(to_resolution_error) {
          Ok(conn) => conn,
          Err(e) => return Some((Err(e), batches)),
        };
        let tixels: Result<Vec<_>, ResolutionError> = sqlx::query_as::<_, Block>("
          SELECT cid, data FROM Tixels WHERE strand = $1 AND idx >= $2 AND idx <= $3
        ")
          .bind(range.strand.to_bytes())
          .bind(batch.start as i64)
          .bind(batch.end as i64)
          .fetch(&mut *conn)
          .map_err(to_resolution_error)
          .map_ok(|(cid, data)| {
            let cid = Cid::try_from(cid).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
            Ok::<_, ResolutionError>(Arc::new(Tixel::from_block(cid, data)?))
          })
          .try_collect()
          .await;
        Some((tixels, batches))
      }
    })
    .map_ok(|v| futures::stream::iter(v.into_iter()))
    .try_flatten()
    .boxed();

    Ok(stream)
  }
}

impl Resolver for SqlStore {}

#[async_trait]
impl Store for SqlStore {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
    match twine.into() {
      AnyTwine::Tixel(t) => self.save_tixel(&t).await.map_err(|e| e.into()),
      AnyTwine::Strand(s) => self.save_strand(&s).await.map_err(|e| e.into()),
    }
  }

  async fn save_many<I: Into<AnyTwine> + Send, S: Iterator<Item = I> + Send, T: IntoIterator<Item = I, IntoIter = S> + Send>(&self, twines: T) -> Result<(), StoreError> {
    for twine in twines {
      self.save(twine).await?;
    }
    Ok(())
  }

  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send + Unpin>(&self, twines: T) -> Result<(), StoreError> {
    twines
      .chunks(100)
      .then(|chunk| self.save_many(chunk))
      .try_for_each(|_| async { Ok(()) })
      .await?;
    Ok(())
  }

  async fn delete<C: AsCid + Send>(&self, cid: C) -> Result<(), StoreError> {
    if self.has_strand_cid(cid.as_cid()).await? {
      self.remove_strand(cid.as_cid()).await.map_err(|e| e.into())
    } else if self.has_tixel(cid.as_cid()).await? {
      self.remove_tixel_if_latest(cid.as_cid()).await.map_err(|e| e.into())
    } else {
      Ok(())
    }
  }
}
