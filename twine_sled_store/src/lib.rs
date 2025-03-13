use async_trait::async_trait;
use futures::Stream;
use itertools::Itertools;
use sled::transaction::TransactionError;
use sled::Db;
use std::collections::{HashMap, HashSet};
use std::{pin::Pin, sync::Arc};
use twine_core::resolver::{unchecked_base::BaseResolver, AbsoluteRange, Resolver};
use twine_core::{as_cid::AsCid, errors::*, store::Store, twine::TwineBlock, twine::*, Cid};
use zerocopy::{FromZeros, KnownLayout};
use zerocopy::{
  byteorder::{BigEndian, U64},
  IntoBytes, FromBytes, Unaligned, Immutable
};

pub use sled;

#[derive(FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable)]
#[repr(C)]
struct LatestRecord {
  index: U64<BigEndian>,
  cid: [u8; 68],
}

#[derive(FromBytes, IntoBytes, Unaligned, KnownLayout, Immutable)]
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
    Self { buffer_size: 100 }
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
  db: Arc<Db>,
  options: SledStoreOptions,
}

impl SledStore {
  pub fn new(db: Db, options: SledStoreOptions) -> Self {
    Self {
      db: Arc::new(db),
      options,
    }
  }
}

fn get_index_key(strand: &Cid, index: u64) -> Vec<u8> {
  let mut key = IndexKey::new_zeroed();
  let cid = strand.to_bytes();
  key.strand[..cid.len()].copy_from_slice(&cid);
  key.index.set(index);
  key.as_bytes().to_vec()
}

fn get_latest_key(strand: &Cid) -> Vec<u8> {
  let mut key = "latest:".as_bytes().to_vec();
  key.extend_from_slice(&strand.to_bytes());
  key
}

fn get_strand_prefix() -> Vec<u8> {
  "strand:".as_bytes().to_vec()
}

fn get_strand_key(strand: &Cid) -> Vec<u8> {
  let mut key = get_strand_prefix();
  key.extend_from_slice(&strand.to_bytes());
  key
}

fn get_strand_from_key(key: &[u8]) -> Cid {
  let pfx = get_strand_prefix();
  Cid::try_from(key[pfx.len()..].to_vec()).unwrap()
}

impl SledStore {
  pub fn flush(&self) -> sled::Result<usize> {
    self.db.flush()
  }

  async fn get(&self, cid: &Cid) -> Result<AnyTwine, ResolutionError> {
    let bytes = self
      .db
      .get(cid.to_bytes())
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?
      .ok_or(ResolutionError::NotFound)?;
    Ok(AnyTwine::from_block(*cid, bytes)?)
  }

  async fn get_tixel(&self, strand: &Cid, cid: &Cid) -> Result<Tixel, ResolutionError> {
    let bytes = self
      .db
      .get(cid.to_bytes())
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?
      .ok_or(ResolutionError::NotFound)?;
    let tixel = Tixel::from_block(*cid, bytes)?;
    if tixel.strand_cid() != *strand {
      return Err(ResolutionError::BadData(
        "Tixel does not belong to strand".to_string(),
      ));
    }
    Ok(tixel)
  }

  fn latest_index(&self, strand: &Cid) -> Result<Option<u64>, ResolutionError> {
    let latest = self
      .db
      .get(get_latest_key(strand))
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    match latest {
      None => return Ok(None),
      Some(latest) => {
        let record = LatestRecord::ref_from_bytes(&latest).map_err(|e| ResolutionError::BadData(
          e.to_string(),
        ))?;
        let index = record.index.get();
        Ok(Some(index))
      }
    }
  }

  fn latest_cid(&self, strand: &Cid) -> Result<Option<Cid>, ResolutionError> {
    let latest = self
      .db
      .get(get_latest_key(strand))
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    match latest {
      None => return Ok(None),
      Some(latest) => {
        let record = LatestRecord::ref_from_bytes(&latest).map_err(|e| ResolutionError::BadData(
          e.to_string(),
        ))?;
        let cid =
          Cid::try_from(record.cid.to_vec()).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        Ok(Some(cid))
      }
    }
  }

  fn check_update(&self, twine: &Tixel) -> Result<(), StoreError> {
    let cid = twine.strand_cid();
    let latest_index = self
      .latest_index(&cid)
      .map_err(|e| StoreError::Saving(e.to_string()))?;
    if latest_index.map(|i| twine.index() > i).unwrap_or(true) {
      // update latest
      let mut cid_slice = [0u8; 68];
      cid_slice.copy_from_slice(&twine.cid().to_bytes());
      let record = LatestRecord {
        index: U64::new(twine.index()),
        cid: cid_slice,
      };
      self
        .db
        .insert(get_latest_key(&cid), record.as_bytes())
        .map_err(|e| StoreError::Saving(e.to_string()))?;
      log::debug!("Updated latest for strand {}: {}", cid, twine.index());
    }
    Ok(())
  }
}

#[async_trait]
impl BaseResolver for SledStore {
  async fn fetch_strands(
    &self,
  ) -> Result<
    Pin<Box<dyn Stream<Item = Result<Strand, ResolutionError>> + Send + '_>>,
    ResolutionError,
  > {
    let iter = self.db.scan_prefix(get_strand_prefix());
    use futures::stream::StreamExt;
    let stream = futures::stream::iter(iter).then(|item| async {
      let (key, _) = item.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
      let cid = get_strand_from_key(&key);
      self.fetch_strand(&cid).await
    });

    Ok(Box::pin(stream))
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(
      self
        .db
        .contains_key(cid.as_cid().to_bytes())
        .unwrap_or(false),
    )
  }

  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    Ok(
      self
        .db
        .contains_key(get_index_key(strand, index))
        .unwrap_or(false),
    )
  }

  async fn has_twine(&self, _strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(
      self
        .db
        .contains_key(cid.as_cid().to_bytes())
        .unwrap_or(false),
    )
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
    let bytes = self
      .db
      .get(strand.to_bytes())
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?
      .ok_or(ResolutionError::NotFound)?;
    Ok(Strand::from_block(strand.clone(), bytes)?)
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
    let tixel = self.get_tixel(strand, tixel).await?;
    Ok(tixel)
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
    let cid = self
      .db
      .get(get_index_key(&strand, index))
      .map_err(|e| ResolutionError::Fetch(e.to_string()))?
      .ok_or(ResolutionError::NotFound)?;
    let cid = Cid::try_from(cid.to_vec()).map_err(|e| ResolutionError::Fetch(e.to_string()))?;
    let tixel = self.get_tixel(strand, &cid).await?;

    if tixel.index() != index {
      return Err(ResolutionError::BadData(format!(
        "Expected index {}, found {}",
        index,
        tixel.index()
      )));
    }

    Ok(tixel)
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
    let cid = self.latest_cid(&strand)?.ok_or(ResolutionError::NotFound)?;
    match self.get_tixel(strand, &cid).await {
      Ok(tixel) => Ok(tixel),
      Err(ResolutionError::NotFound) => {
        // we have a latest record but no entry for cid... so remove the latest entry
        self
          .db
          .remove(get_latest_key(strand))
          .map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        Err(ResolutionError::NotFound)
      }
      Err(e) => Err(e),
    }
  }

  async fn range_stream(
    &self,
    range: AbsoluteRange,
  ) -> Result<
    Pin<Box<dyn Stream<Item = Result<Tixel, ResolutionError>> + Send + '_>>,
    ResolutionError,
  > {
    use futures::stream::StreamExt;
    let strand_cid = range.strand;
    let sled_range =
      get_index_key(&strand_cid, range.start)..=get_index_key(&strand_cid, range.end);
    use either::Either;
    let iter = if range.is_decreasing() {
      Either::Left(self.db.range(sled_range).rev())
    } else {
      Either::Right(self.db.range(sled_range))
    };
    let stream = futures::stream::iter(iter)
      .map(move |item| async move {
        let (_, cid) = item.map_err(|e| ResolutionError::Fetch(e.to_string()))?;
        let cid =
          Cid::try_from(cid.to_vec()).map_err(|e| ResolutionError::BadData(e.to_string()))?;
        let tixel = self.get_tixel(&strand_cid, &cid).await?;
        Ok(tixel)
      })
      .buffered(self.options.buffer_size);
    Ok(stream.boxed())
  }
}

impl Resolver for SledStore {}

#[async_trait]
impl Store for SledStore {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
    let twine = twine.into();
    let cid = twine.cid();

    match &twine {
      AnyTwine::Strand(strand) => {
        self
          .db
          .transaction(|db| {
            db.insert(get_strand_key(&strand.cid()), &[])?;
            db.insert(cid.to_bytes(), &*twine.bytes())?;
            Ok(())
          })
          .map_err(|e: TransactionError| StoreError::Saving(e.to_string()))?;
      }
      AnyTwine::Tixel(tixel) => {
        let strand = tixel.strand_cid();
        if !self.has_strand(&strand).await? {
          return Err(StoreError::Saving(format!(
            "Strand {} not saved yet",
            strand
          )));
        }
        self
          .db
          .transaction(|db| {
            let index = tixel.index();
            db.insert(get_index_key(&strand, index), cid.to_bytes())?;
            db.insert(cid.to_bytes(), &*twine.bytes())?;
            Ok(())
          })
          .map_err(|e: TransactionError| StoreError::Saving(e.to_string()))?;

        self.check_update(&tixel)?;
      }
    }

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
    let mut stored_strands = HashSet::new();
    let (strands, tixels) = twines
      .into_iter()
      .map(|i| i.into())
      .partition::<Vec<AnyTwine>, _>(|twine| matches!(twine, AnyTwine::Strand(_)));

    if strands.len() > 0 {
      let mut batch = sled::Batch::default();
      for strand in strands.iter().unique() {
        let cid = strand.cid();
        stored_strands.insert(cid);
        batch.insert(get_strand_key(&cid), &[]);
        batch.insert(cid.to_bytes(), &*strand.bytes());
      }
      self
        .db
        .apply_batch(batch)
        .map_err(|e| StoreError::Saving(e.to_string()))?;
    }

    if tixels.len() > 0 {
      let tixels = tixels.into_iter().map(|t| t.unwrap_tixel());
      let mut latests: HashMap<Cid, Tixel> = HashMap::new();
      let mut batch = sled::Batch::default();
      for tixel in tixels {
        let strand = tixel.strand_cid();
        if !stored_strands.contains(&strand) {
          let has = self.has_strand(&strand).await?;
          if has {
            stored_strands.insert(strand);
          } else {
            return Err(StoreError::Saving(format!(
              "Strand {} not saved yet",
              strand
            )));
          }
        }
        let index = tixel.index();
        latests
          .entry(strand)
          .and_modify(|t| {
            if index > t.index() {
              *t = tixel.clone()
            }
          })
          .or_insert(tixel.clone());
        batch.insert(get_index_key(&strand, index), tixel.cid().to_bytes());
        batch.insert(tixel.cid().to_bytes(), &*tixel.bytes());
      }

      self
        .db
        .apply_batch(batch)
        .map_err(|e| StoreError::Saving(e.to_string()))?;

      // check latests
      for (_, tixel) in latests {
        self.check_update(&tixel)?;
      }
    }

    Ok(())
  }

  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send + Unpin>(
    &self,
    twines: T,
  ) -> Result<(), StoreError> {
    use futures::stream::{StreamExt, TryStreamExt};
    // save in batches
    twines
      .chunks(self.options.buffer_size)
      .then(|chunk| self.save_many(chunk))
      .try_for_each(|_| async { Ok(()) })
      .await?;
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
          self
            .db
            .remove(key)
            .map_err(|e| StoreError::Saving(e.to_string()))?;
        }
        self
          .db
          .remove(get_latest_key(&strand_cid))
          .map_err(|e| StoreError::Saving(e.to_string()))?;
        self
          .db
          .remove(get_strand_key(&strand_cid))
          .map_err(|e| StoreError::Saving(e.to_string()))?;
      }
      AnyTwine::Tixel(tixel) => {
        let strand = tixel.strand_cid();
        let index = tixel.index();
        self
          .db
          .remove(get_index_key(&strand, index))
          .map_err(|e| StoreError::Saving(e.to_string()))?;
      }
    }
    self
      .db
      .remove(twine.cid().to_bytes())
      .map_err(|e| StoreError::Saving(e.to_string()))?;
    Ok(())
  }
}
