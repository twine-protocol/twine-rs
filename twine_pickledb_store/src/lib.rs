use async_trait::async_trait;
use futures::stream::Stream;
use futures::StreamExt;
use futures::TryStreamExt;
use pickledb::PickleDbListIterator;
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use twine_core::resolver::{unchecked_base::BaseResolver, AbsoluteRange, Resolver};
use twine_core::{as_cid::AsCid, errors::*, store::Store, twine::*, Cid};

pub use pickledb;

fn copy_policy(p: &PickleDbDumpPolicy) -> PickleDbDumpPolicy {
  match p {
    PickleDbDumpPolicy::NeverDump => PickleDbDumpPolicy::NeverDump,
    PickleDbDumpPolicy::AutoDump => PickleDbDumpPolicy::AutoDump,
    PickleDbDumpPolicy::DumpUponRequest => PickleDbDumpPolicy::DumpUponRequest,
    PickleDbDumpPolicy::PeriodicDump(n) => PickleDbDumpPolicy::PeriodicDump(*n),
  }
}

fn push_list<'a, V: Serialize>(
  db: &'a mut PickleDb,
  key: &str,
  value: &V,
) -> Result<pickledb::PickleDbListExtender<'a>, StoreError> {
  if !db.lexists(key) {
    db.lcreate(key)
      .map_err(|e| StoreError::Saving(e.to_string()))?;
  }
  db.ladd(key, value).ok_or(StoreError::Saving(format!(
    "Could not add list for key {}",
    key
  )))
}

fn get_list_iter<'a>(db: &'a PickleDb, key: &str) -> Option<PickleDbListIterator<'a>> {
  if !db.lexists(key) {
    return None;
  }
  Some(db.liter(key))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlockRecord {
  cid: Cid,
  #[serde(with = "serde_bytes")]
  bytes: Vec<u8>,
}

impl From<Tixel> for BlockRecord {
  fn from(tixel: Tixel) -> Self {
    Self {
      cid: tixel.cid(),
      bytes: tixel.bytes().to_vec(),
    }
  }
}

impl From<Strand> for BlockRecord {
  fn from(strand: Strand) -> Self {
    Self {
      cid: strand.cid(),
      bytes: strand.bytes().to_vec(),
    }
  }
}

impl TryFrom<BlockRecord> for Tixel {
  type Error = VerificationError;

  fn try_from(value: BlockRecord) -> Result<Self, Self::Error> {
    Ok(Tixel::from_block(value.cid, value.bytes)?)
  }
}

impl TryFrom<BlockRecord> for Strand {
  type Error = VerificationError;

  fn try_from(value: BlockRecord) -> Result<Self, Self::Error> {
    Ok(Strand::from_block(value.cid, value.bytes)?)
  }
}

impl TryFrom<BlockRecord> for AnyTwine {
  type Error = VerificationError;

  fn try_from(value: BlockRecord) -> Result<Self, Self::Error> {
    AnyTwine::from_block(value.cid, value.bytes)
  }
}

#[derive(Clone)]
pub struct PickleDbStore {
  pickle: Arc<Mutex<PickleDb>>,
}

impl Debug for PickleDbStore {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let nkeys = self.pickle.lock().expect("Lock on pickle db").total_keys();
    f.debug_struct("PickleDbStore")
      .field("nkeys", &nkeys)
      .finish()
  }
}

impl PickleDbStore {
  pub fn new(p: impl AsRef<Path>) -> pickledb::error::Result<Self> {
    Self::new_with_policy(p, PickleDbDumpPolicy::DumpUponRequest)
  }

  pub fn new_with_policy(
    p: impl AsRef<Path>,
    dump_policy: PickleDbDumpPolicy,
  ) -> pickledb::error::Result<Self> {
    let path = p.as_ref();
    let pickle = match PickleDb::load(path, copy_policy(&dump_policy), SerializationMethod::Bin) {
      Ok(pl) => pl,
      Err(e) => {
        if let pickledb::error::ErrorType::Io = e.get_type() {
          PickleDb::new(path, dump_policy, SerializationMethod::Bin)
        } else {
          return Err(e);
        }
      }
    };
    Ok(Self {
      pickle: Arc::new(Mutex::new(pickle)),
    })
  }

  pub fn load_read_only(p: impl AsRef<Path>) -> pickledb::error::Result<Self> {
    let pickle = PickleDb::load_read_only(p, SerializationMethod::Bin);
    Ok(Self {
      pickle: Arc::new(Mutex::new(pickle?)),
    })
  }

  fn all_strands(&self) -> Result<Vec<Strand>, ResolutionError> {
    let lock = self.pickle.lock().expect("Lock on pickle db");
    match get_list_iter(&lock, "strands") {
      Some(iter) => iter
        .map(|v| {
          v.get_item::<BlockRecord>().ok_or(ResolutionError::BadData(
            "Could not deserialize from DB correctly".to_string(),
          ))
        })
        .map(|v| v.and_then(|v| v.try_into().map_err(|e: VerificationError| e.into())))
        .collect(),
      None => Ok(vec![]),
    }
  }

  fn get_strand(&self, cid: &Cid) -> Result<Strand, ResolutionError> {
    self
      .all_strands()?
      .into_iter()
      .find(|s| s.cid() == *cid)
      .ok_or(ResolutionError::NotFound)
  }

  fn get_tixel(&self, cid: &Cid) -> Result<Tixel, ResolutionError> {
    let record: BlockRecord = self
      .pickle
      .lock()
      .expect("Lock on pickle db")
      .get(&format!("{}", cid))
      .ok_or(ResolutionError::NotFound)?;
    record.try_into().map_err(|e: VerificationError| e.into())
  }

  fn has_tixel(&self, cid: &Cid) -> bool {
    self
      .pickle
      .lock()
      .expect("Lock on pickle db")
      .exists(&format!("{}", cid))
  }

  fn cid_for_index<S: AsCid>(&self, strand: S, index: u64) -> Option<Cid> {
    self
      .pickle
      .lock()
      .expect("Lock on pickle db")
      .lget(&format!("tixels:{}", strand.as_cid()), index as usize)
  }

  // Inclusive range
  fn cid_range<S: AsCid>(
    &self,
    strand: S,
    start: u64,
    end: u64,
  ) -> Result<Vec<Cid>, ResolutionError> {
    get_list_iter(
      &self.pickle.lock().expect("Lock on pickle db"),
      &format!("tixels:{}", strand.as_cid()),
    )
    .ok_or(ResolutionError::NotFound)?
    .skip(start as usize)
    .take((end - start + 1) as usize)
    .map(|v| {
      v.get_item::<Cid>().ok_or(ResolutionError::BadData(
        "Could not deserialize from DB correctly".to_string(),
      ))
    })
    .collect()
  }

  fn latest_entry<S: AsCid>(&self, strand: S) -> Result<Tixel, ResolutionError> {
    let cid = {
      let lock = self.pickle.lock().expect("Lock on pickle db");
      match get_list_iter(&lock, &format!("tixels:{}", strand.as_cid())) {
        Some(iter) => {
          let last = iter.last().ok_or(ResolutionError::NotFound)?;
          last.get_item::<Cid>().ok_or(ResolutionError::BadData(
            "Could not deserialize from DB correctly".to_string(),
          ))?
        }
        None => return Err(ResolutionError::NotFound),
      }
    };
    self.get_tixel(&cid)
  }

  pub fn latest_index<S: AsCid>(&self, strand: S) -> Option<u64> {
    let len = self
      .pickle
      .lock()
      .expect("Lock on pickle db")
      .llen(&format!("tixels:{}", strand.as_cid()));
    if len == 0 {
      return None;
    }
    Some(len as u64 - 1)
  }

  fn save_tixel(&self, tixel: Tixel) -> Result<(), StoreError> {
    // ensure we have the strand
    if self.get_strand(&tixel.strand_cid()).is_err() {
      return Err(StoreError::Saving(
        "Strand must be saved before tixels".to_string(),
      ));
    }
    if tixel.index() != 0 && !self.has_tixel(&tixel.previous().unwrap().tixel) {
      return Err(StoreError::Saving(
        "Previous tixel must be saved before this one".to_string(),
      ));
    }
    let tixel_cid = tixel.cid();
    let strand_cid = tixel.strand_cid();
    let mut lock = self.pickle.lock().expect("Lock on pickle db");
    lock
      .set(&format!("{}", tixel_cid), &BlockRecord::from(tixel))
      .map_err(|e| StoreError::Saving(e.to_string()))?;
    push_list(&mut lock, &format!("tixels:{}", strand_cid), &tixel_cid)?;
    self.flush()?;
    Ok(())
  }

  fn save_strand(&self, strand: Strand) -> Result<(), StoreError> {
    let mut lock = self.pickle.lock().expect("Lock on pickle db");
    push_list(&mut lock, "strands", &BlockRecord::from(strand))?;
    Ok(())
  }

  fn remove_strand(&self, cid: &Cid) -> Result<(), StoreError> {
    let strand = match self.get_strand(cid) {
      Ok(s) => s,
      Err(_) => return Ok(()),
    };
    let mut lock = self.pickle.lock().expect("Lock on pickle db");
    let _: () = self
      .pickle
      .lock()
      .expect("Lock on pickle db")
      .liter(&format!("tixels:{}", cid))
      .map(|v| {
        let block = v.get_item::<BlockRecord>().ok_or(ResolutionError::BadData(
          "Could not deserialize from DB correctly".to_string(),
        ))?;
        lock
          .rem(&format!("{}", block.cid))
          .map_err(|e| StoreError::Saving(e.to_string()))?;
        Ok::<_, StoreError>(())
      })
      .collect::<Result<_, _>>()?;
    lock
      .lrem_value("strands", &BlockRecord::from(strand))
      .map_err(|e| StoreError::Saving(e.to_string()))?;
    Ok(())
  }

  pub fn flush(&self) -> Result<(), StoreError> {
    self
      .pickle
      .lock()
      .expect("Lock on pickle db")
      .dump()
      .map_err(|e| StoreError::Saving(e.to_string()))?;
    Ok(())
  }
}

#[async_trait]
impl BaseResolver for PickleDbStore {
  async fn fetch_strands(
    &self,
  ) -> Result<
    Pin<Box<dyn Stream<Item = Result<Strand, ResolutionError>> + Send + '_>>,
    ResolutionError,
  > {
    let strands = self.all_strands()?;
    Ok(Box::pin(futures::stream::iter(strands.into_iter().map(Ok))))
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    self
      .get_strand(cid)
      .map(|_| true)
      .or_else(|_| Ok(self.has_tixel(cid)))
  }

  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    Ok(self.cid_for_index(strand, index).is_some())
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(self.has_tixel(cid) && self.get_strand(strand).is_ok())
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
    self.get_strand(strand)
  }

  async fn fetch_tixel(&self, _strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
    self.get_tixel(tixel)
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
    let cid = self
      .cid_for_index(strand, index)
      .ok_or(ResolutionError::NotFound)?;
    self.get_tixel(&cid)
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
    self.latest_entry(strand)
  }

  async fn range_stream(
    &self,
    range: AbsoluteRange,
  ) -> Result<
    Pin<Box<dyn Stream<Item = Result<Tixel, ResolutionError>> + Send + '_>>,
    ResolutionError,
  > {
    let cids = self.cid_range(range.strand, range.start, range.end)?;
    Ok(Box::pin(futures::stream::iter(
      cids.into_iter().map(|cid| self.get_tixel(&cid)),
    )))
  }
}

impl Resolver for PickleDbStore {}

#[async_trait]
impl Store for PickleDbStore {
  async fn save<T: Into<AnyTwine> + Send>(&self, twine: T) -> Result<(), StoreError> {
    match twine.into() {
      AnyTwine::Tixel(t) => self.save_tixel(t).map_err(|e| e.into()),
      AnyTwine::Strand(s) => self.save_strand(s).map_err(|e| e.into()),
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
    if self.has_strand(cid.as_cid()).await? {
      self.remove_strand(cid.as_cid()).map_err(|e| e.into())
    } else if self.has_tixel(cid.as_cid()) {
      unimplemented!()
    } else {
      Ok(())
    }
  }
}
