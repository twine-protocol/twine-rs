use async_trait::async_trait;
use futures::{join, Stream};
use zerocopy::FromZeroes;
use std::{pin::Pin, sync::Arc};
use twine_core::{twine::*, twine::TwineBlock, errors::*, as_cid::AsCid, store::Store, resolver::RangeQuery, Cid};
use twine_core::resolver::BaseResolver;
use sled::Db;
use zerocopy::{
  byteorder::{U64, BigEndian}, AsBytes, FromBytes, Unaligned,
};

pub use sled;

#[derive(FromZeroes, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
struct LatestRecord {
  index: U64<BigEndian>,
  cid: [u8; 68],
}

#[derive(FromZeroes, FromBytes, AsBytes, Unaligned)]
#[repr(C)]
struct IndexKey {
  strand: [u8; 68],
  index: U64<BigEndian>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SledStoreOptions {
  buffer_size: usize,
}

impl Default for SledStoreOptions {
  fn default() -> Self {
    Self {
      buffer_size: 100,
    }
  }
}

impl SledStoreOptions {
  pub fn buffer_size(mut self, buffer_size: usize) -> Self {
    self.buffer_size = buffer_size;
    self
  }
}

#[derive(Debug, Clone)]
pub struct SledStore {
  db: Db,
  options: SledStoreOptions,
}

impl SledStore {
  pub fn new(db: Db, options: SledStoreOptions) -> Self {
    Self {
      db,
      options,
    }
  }
}

fn get_index_key(strand: &Cid, index: u64) -> Vec<u8> {
  let mut key = IndexKey::new_zeroed();
  key.strand.copy_from_slice(&strand.to_bytes());
  key.index.set(index);
  key.as_bytes().to_vec()
}

fn get_latest_key(strand: &Cid) -> String {
  format!("latest:{}", strand)
}

fn get_strand_prefix() -> &'static str {
  "strand:"
}

fn get_strand_key(strand: &Cid) -> String {
  format!("{}{}", get_strand_prefix(), strand)
}

fn get_strand_from_key(key: &[u8]) -> Cid {
  Cid::try_from(key[7..].to_vec()).unwrap()
}

impl SledStore {
  async fn get(&self, cid: &Cid) -> Result<AnyTwine, ResolutionError> {
    let bytes = self.db.get(cid.to_bytes())
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?
      .ok_or(ResolutionError::NotFound)?;
    Ok(AnyTwine::from_block(*cid, bytes)?)
  }

  async fn get_tixel(&self, strand: &Cid, cid: &Cid) -> Result<Tixel, ResolutionError> {
    let bytes = self.db.get(cid.to_bytes())
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?
      .ok_or(ResolutionError::NotFound)?;
    let tixel = Tixel::from_block(*cid, bytes)?;
    if tixel.strand_cid() != *strand {
      return Err(ResolutionError::BadData("Tixel does not belong to strand".to_string()));
    }
    Ok(tixel)
  }

  fn latest_index(&self, strand: &Cid) -> Result<Option<u64>, ResolutionError> {
    let latest = self.db.get(get_latest_key(strand))
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    match latest {
      None => return Ok(None),
      Some(latest) => {
        let record = LatestRecord::ref_from(&latest).ok_or(ResolutionError::BadData("Invalid latest record".to_string()))?;
        let index = record.index.get();
        Ok(Some(index))
      }
    }
  }

  fn latest_cid(&self, strand: &Cid) -> Result<Option<Cid>, ResolutionError> {
    let latest = self.db.get(get_latest_key(strand))
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    match latest {
      None => return Ok(None),
      Some(latest) => {
        let record = LatestRecord::ref_from(&latest).ok_or(ResolutionError::BadData("Invalid latest record".to_string()))?;
        let cid = Cid::try_from(record.cid.to_vec()).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        Ok(Some(cid))
      }
    }
  }

  fn check_update(&self, twine: &Tixel) -> Result<(), StoreError> {
    let cid = twine.strand_cid();
    let latest_index = self.latest_index(&cid).map_err(|e| StoreError::Saving(e.to_string()))?;
    if latest_index.map(|i| twine.index() > i).unwrap_or(true) {
      // update latest
      let mut cid_slice = [0u8; 68];
      cid_slice.copy_from_slice(&twine.cid().to_bytes());
      let record = LatestRecord {
        index: U64::new(twine.index()),
        cid: cid_slice,
      };
      self.db.insert(get_latest_key(&cid), record.as_bytes())
        .map_err(|e| StoreError::Saving(e.to_string()))?;
    }
    Ok(())
  }
}

#[async_trait]
impl BaseResolver for SledStore {

  async fn strands(&self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + '_>>, ResolutionError> {
    let iter = self.db.scan_prefix(get_strand_prefix());
    use futures::stream::StreamExt;
    let stream = futures::stream::iter(iter)
      .then(|item| async {
        let (key, _) = item.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        let cid = get_strand_from_key(&key);
        self.fetch_strand(&cid).await
      });

    Ok(Box::pin(stream))
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(self.db.contains_key(cid.as_cid().to_bytes()).unwrap_or(false))
  }

  async fn has_twine(&self, _strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(self.db.contains_key(cid.as_cid().to_bytes()).unwrap_or(false))
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    let key = get_strand_key(&strand);
    let bytes = self.db.get(key)
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?
      .ok_or(ResolutionError::NotFound)?;
    Ok(Arc::new(Strand::from_block(strand.clone(), bytes)?))
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let tixel = self.get_tixel(strand, tixel).await?;
    Ok(Arc::new(tixel))
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    let cid = self.db.get(get_index_key(&strand, index))
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?
      .ok_or(ResolutionError::NotFound)?;
    let cid = Cid::try_from(cid.to_vec()).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    let tixel = self.get_tixel(strand, &cid).await?;

    if tixel.index() != index {
      return Err(ResolutionError::BadData(format!("Expected index {}, found {}", index, tixel.index())));
    }

    Ok(Arc::new(tixel))
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let cid = self.latest_cid(&strand)?.ok_or(ResolutionError::NotFound)?;
    let tixel = self.get_tixel(strand, &cid).await?;
    Ok(Arc::new(tixel))
  }

  async fn range_stream(&self, range: RangeQuery) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + '_>>, ResolutionError> {
    use futures::stream::StreamExt;
    let range = range.try_to_absolute(self).await?;
    let strand_cid = range.strand;
    let mut expecting = range.upper;
    let sled_range = get_index_key(&strand_cid, range.lower)..=get_index_key(&strand_cid, range.upper);
    let iter = self.db.range(sled_range).rev();
    let stream = futures::stream::iter(iter)
      // we're expecting the keys to be all present, but we need to check
      // and return NotFound if they're not
      .map(move |item| {
        let (key, cid) = match item {
          Ok((key, cid)) => (key, cid),
          Err(e) => {
            expecting = expecting.saturating_sub(1);
            return futures::stream::iter(vec![Err(ResolutionError::Fetch(e.to_string()))])
          },
        };
        let index = IndexKey::ref_from(&key).map(|r| r.index.get());
        match index {
          None => {
            expecting = expecting.saturating_sub(1);
            let res = vec![Err(ResolutionError::Fetch("Key record is corrupted".to_string()))];
            futures::stream::iter(res)
          },
          Some(index) => {
            let mut res = Vec::new();
            while index < expecting {
              res.push(Err(ResolutionError::NotFound));
              expecting = expecting.saturating_sub(1);
            }
            expecting = expecting.saturating_sub(1);
            match Cid::try_from(cid.to_vec()) {
              Ok(cid) => {
                res.push(Ok((index, cid)));
                futures::stream::iter(res)
              },
              Err(e) => {
                res.push(Err(ResolutionError::Fetch(e.to_string())));
                futures::stream::iter(res)
              },
            }
          }
        }
      })
      .flatten()
      .map(move |res| {
        let strand_cid = strand_cid.clone();
        async move {
          let (index, cid) = res?;
          let tixel = self.get_tixel(&strand_cid, &cid).await?;
          if tixel.index() != index {
            return Err(ResolutionError::BadData(format!("Expected index {}, found {}", index, tixel.index())));
          }
          Ok(Arc::new(tixel))
        }
      })
      .buffered(self.options.buffer_size);
    Ok(stream.boxed())
  }
}

#[async_trait]
impl Store for SledStore {

  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
    let twine = twine.into();
    let cid = twine.cid();
    match &twine {
      AnyTwine::Strand(strand) => {
        self.db.insert(get_strand_key(&strand.cid()), &*strand.bytes())
          .map_err(|e| StoreError::Saving(e.to_string()))?;
      },
      AnyTwine::Tixel(tixel) => {
        let strand = tixel.strand_cid();
        if !self.has_strand(&strand).await? {
          return Err(StoreError::Saving(format!("Strand {} not saved yet", strand)));
        }
        self.check_update(&tixel)?;
        let index = tixel.index();
        self.db.insert(get_index_key(&strand, index), cid.to_bytes())
          .map_err(|e| StoreError::Saving(e.to_string()))?;
      },
    }

    self.db.insert(cid.to_bytes(), &*twine.bytes())
      .map_err(|e| StoreError::Saving(e.to_string()))?;

    Ok(())
  }

  async fn save_many<I: Into<AnyTwine> + Send, S: Iterator<Item = I> + Send, T: IntoIterator<Item = I, IntoIter = S> + Send>(&self, twines: T) -> Result<(), StoreError> {
    let mut batch = sled::Batch::default();
    for twine in twines {
      let twine = twine.into();
      let cid = twine.cid();
      match &twine {
        AnyTwine::Strand(strand) => {
          batch.insert(get_strand_key(&strand.cid()).as_str(), &[]);
        },
        AnyTwine::Tixel(tixel) => {
          let strand = tixel.strand_cid();
          if !self.has_strand(&strand).await? {
            return Err(StoreError::Saving(format!("Strand {} not saved yet", strand)));
          }
          self.check_update(&tixel)?;
          let index = tixel.index();
          batch.insert(get_index_key(&strand, index), cid.to_bytes());
        },
      }
      batch.insert(cid.to_bytes(), &*twine.bytes());
    }

    self.db.apply_batch(batch)
      .map_err(|e| StoreError::Saving(e.to_string()))?;

    Ok(())
  }

  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send + Unpin>(&self, twines: T) -> Result<(), StoreError> {
    use futures::stream::StreamExt;
    self.save_many(twines.collect::<Vec<_>>().await).await?;
    Ok(())
  }

  async fn delete<C: AsCid + Send>(&self, cid: C) -> Result<(), StoreError> {
    let twine = match self.get(cid.as_cid()).await {
      Ok(twine) => twine,
      Err(ResolutionError::NotFound) => return Ok(()),
      Err(e) => return Err(StoreError::Saving(e.to_string())),
    };
    match &twine {
      AnyTwine::Strand(strand) => {
        let strand_cid = strand.cid();
        let iter = self.db.range(get_index_key(&strand_cid, 0)..);
        for item in iter {
          let (key, _) = item.map_err(|e| StoreError::Saving(e.to_string()))?;
          self.db.remove(key)
            .map_err(|e| StoreError::Saving(e.to_string()))?;
        }
        self.db.remove(get_latest_key(&strand_cid))
          .map_err(|e| StoreError::Saving(e.to_string()))?;
        self.db.remove(get_strand_key(&strand_cid))
          .map_err(|e| StoreError::Saving(e.to_string()))?;
      },
      AnyTwine::Tixel(tixel) => {
        let strand = tixel.strand_cid();
        let index = tixel.index();
        self.db.remove(get_index_key(&strand, index))
          .map_err(|e| StoreError::Saving(e.to_string()))?;
      },

    }
    self.db.remove(twine.cid().to_bytes())
      .map_err(|e| StoreError::Saving(e.to_string()))?;
    Ok(())
  }
}
