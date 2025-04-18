#![doc = include_str!("../README.md")]

use async_trait::async_trait;
use futures::stream::Stream;
use futures::StreamExt;
use futures::TryStreamExt;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use twine_lib::resolver::RangeQuery;
use twine_lib::resolver::{unchecked_base::BaseResolver, AbsoluteRange, Resolver};
use twine_lib::store::MemoryStore;
use twine_lib::{as_cid::AsCid, errors::*, store::Store, twine::*, Cid};

/// A store that saves twines to a single file in CARv1 format
///
/// The store is completely loaded into memory and then
/// flushed to disk whenever a [`Store`] operation is called.
#[derive(Debug, Clone)]
pub struct CarStore {
  memstore: MemoryStore,
  filename: PathBuf,
}

impl Drop for CarStore {
  fn drop(&mut self) {
    if let Err(e) = async_std::task::block_on(self.flush()) {
      eprintln!("Error flushing store: {:?}", e);
    }
  }
}

impl CarStore {
  /// Create a new store that saves to the given file
  pub fn new<S: AsRef<Path>>(filename: S) -> Result<Self, StoreError> {
    let s = Self {
      memstore: MemoryStore::new(),
      filename: filename.as_ref().to_path_buf(),
    };

    s.load()?;

    Ok(s)
  }

  fn load(&self) -> Result<(), StoreError> {
    // check the file isn't empty first
    if let Ok(metadata) = std::fs::metadata(&self.filename) {
      if metadata.len() == 0 {
        return Ok(());
      }
    } else {
      return Ok(());
    }

    let file = std::fs::File::open(&self.filename)
      .map_err(|e| StoreError::Fetching(ResolutionError::Fetch(e.to_string())))?;
    let mut reader = std::io::BufReader::new(file);
    let twines = twine_lib::car::from_car_bytes(&mut reader)
      .map_err(|e| StoreError::Fetching(ResolutionError::BadData(e.to_string())))?;

    for twine in twines {
      self.memstore.save_sync(twine.into())?;
    }

    Ok(())
  }

  /// Flush the store to disk
  pub async fn flush(&self) -> Result<(), StoreError> {
    let strands: Vec<Strand> = self.memstore.fetch_strands().await?.try_collect().await?;
    let latests: Vec<Tixel> = futures::stream::iter(strands.iter())
      .then(|s| async move {
        let cid = s.cid();
        self.memstore.fetch_latest(&cid).await
      })
      // filter out notfounds
      .filter_map(|r| match r {
        Ok(_) => futures::future::ready(Some(r)),
        Err(e) => match e {
          ResolutionError::NotFound => futures::future::ready(None),
          _ => futures::future::ready(Some(Err(e))),
        }
      })
      .try_collect().await?;
    let roots = strands
      .iter()
      .map(|s| s.cid())
      .chain(latests.iter().map(|t| t.cid()))
      .collect::<Vec<_>>();

    let all_tixels = futures::stream::iter(strands.iter())
      .filter_map(|strand| async {
        let q = match RangeQuery::from((strand.cid(), ..))
          .try_to_absolute(&self.memstore)
          .await
        {
          Ok(q) => q,
          Err(e) => return Some(Err(e)),
        };
        if q.is_none() {
          return None;
        }
        let twines = self.memstore.range_stream(q.unwrap()).await;
        Some(twines)
      })
      .try_flatten()
      .filter_map(|r| async { r.ok() });

    let strands = futures::stream::iter(strands.iter()).map(|s| AnyTwine::Strand(s.clone()));
    let all_tixels = all_tixels.map(|t| AnyTwine::Tixel(t));
    let all = strands.chain(all_tixels);

    let mut bytes = twine_lib::car::to_car_stream(all, roots).boxed();

    let map_err = |e: std::io::Error| StoreError::Saving(e.to_string());
    let mut file = std::fs::File::create(&self.filename).map_err(map_err)?; // Create or truncate the file
    use std::io::Write;
    while let Some(chunk) = bytes.next().await {
      file.write_all(&chunk).map_err(map_err)?; // Write each chunk
    }
    file.flush().map_err(map_err)?; // Ensure all data is written
    Ok(())
  }
}

#[async_trait]
impl BaseResolver for CarStore {
  async fn fetch_strands(
    &self,
  ) -> Result<
    Pin<Box<dyn Stream<Item = Result<Strand, ResolutionError>> + Send + '_>>,
    ResolutionError,
  > {
    self.memstore.fetch_strands().await
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    self.memstore.has_strand(cid).await
  }

  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    self.memstore.has_index(strand, index).await
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    self.memstore.has_twine(strand, cid).await
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
    self.memstore.fetch_strand(strand).await
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
    self.memstore.fetch_tixel(strand, tixel).await
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
    self.memstore.fetch_index(strand, index).await
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
    self.memstore.fetch_latest(strand).await
  }

  async fn range_stream(
    &self,
    range: AbsoluteRange,
  ) -> Result<
    Pin<Box<dyn Stream<Item = Result<Tixel, ResolutionError>> + Send + '_>>,
    ResolutionError,
  > {
    self.memstore.range_stream(range).await
  }
}

impl Resolver for CarStore {}

#[async_trait]
impl Store for CarStore {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
    self.memstore.save(twine).await?;
    self.flush().await?;
    Ok(())
  }

  async fn save_many<
    I: Into<AnyTwine> + Send,
    S: Iterator<Item = I> + Send,
    T: IntoIterator<Item = I, IntoIter = S> + Send,
  >(
    &self,
    twines: T,
  ) -> Result<(), StoreError> {
    self.memstore.save_many(twines).await?;
    self.flush().await?;
    Ok(())
  }

  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send + Unpin>(
    &self,
    twines: T,
  ) -> Result<(), StoreError> {
    twines
      .chunks(100)
      .then(|chunk| async {
        self.memstore.save_many(chunk).await?;
        Ok::<_, StoreError>(())
      })
      .try_collect::<Vec<_>>()
      .await?;
    Ok(())
  }

  async fn delete<C: AsCid + Send>(&self, cid: C) -> Result<(), StoreError> {
    self.memstore.delete(cid).await?;
    self.flush().await?;
    Ok(())
  }
}
