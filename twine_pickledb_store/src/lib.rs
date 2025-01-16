use async_trait::async_trait;
use futures::stream::Stream;
use futures::StreamExt;
use futures::TryStreamExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::pin::Pin;
use std::time::Duration;
use twine_core::{twine::*, errors::*, as_cid::AsCid, store::Store, Cid};
use twine_core::resolver::{AbsoluteRange, unchecked_base::BaseResolver, Resolver};
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};
use serde::{Serialize, Deserialize};

pub use pickledb;

fn copy_policy(p: &PickleDbDumpPolicy) -> PickleDbDumpPolicy {
  match p {
    PickleDbDumpPolicy::NeverDump => PickleDbDumpPolicy::NeverDump,
    PickleDbDumpPolicy::AutoDump => PickleDbDumpPolicy::AutoDump,
    PickleDbDumpPolicy::DumpUponRequest => PickleDbDumpPolicy::DumpUponRequest,
    PickleDbDumpPolicy::PeriodicDump(n) => PickleDbDumpPolicy::PeriodicDump(*n),
  }
}

fn push_list<'a, V: Serialize>(db: &'a mut PickleDb, key: &str, value: &V) -> Result<pickledb::PickleDbListExtender<'a>, StoreError> {
  if !db.lexists(key) {
    db.lcreate(key).map_err(|e| StoreError::Saving(e.to_string()))?;
  }
  db.ladd(key, value).ok_or(StoreError::Saving(format!("Could not add list for key {}", key)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlockRecord {
  cid: Cid,
  #[serde(with = "serde_bytes")]
  bytes: Vec<u8>,
}

impl From<Arc<Tixel>> for BlockRecord {
  fn from(tixel: Arc<Tixel>) -> Self {
    Self {
      cid: tixel.cid(),
      bytes: tixel.bytes().to_vec(),
    }
  }
}

impl From<Arc<Strand>> for BlockRecord {
  fn from(strand: Arc<Strand>) -> Self {
    Self {
      cid: strand.cid(),
      bytes: strand.bytes().to_vec(),
    }
  }
}

impl TryFrom<BlockRecord> for Arc<Tixel> {
  type Error = VerificationError;

  fn try_from(value: BlockRecord) -> Result<Self, Self::Error> {
    Ok(Arc::new(Tixel::from_block(value.cid, value.bytes)?))
  }
}

impl TryFrom<BlockRecord> for Arc<Strand> {
  type Error = VerificationError;

  fn try_from(value: BlockRecord) -> Result<Self, Self::Error> {
    Ok(Arc::new(Strand::from_block(value.cid, value.bytes)?))
  }
}

impl TryFrom<BlockRecord> for AnyTwine {
  type Error = VerificationError;

  fn try_from(value: BlockRecord) -> Result<Self, Self::Error> {
    AnyTwine::from_block(value.cid, value.bytes)
  }
}

pub struct PickleDbStore {
  pickle: Arc<Mutex<PickleDb>>,
}

impl PickleDbStore {
  pub fn new(p: impl AsRef<Path>) -> pickledb::error::Result<Self> {
    Self::new_with_policy(p, PickleDbDumpPolicy::PeriodicDump(Duration::from_millis(500)))
  }

  pub fn new_with_policy(p: impl AsRef<Path>, dump_policy: PickleDbDumpPolicy) -> pickledb::error::Result<Self> {
    let path = p.as_ref();
    let pickle = match PickleDb::load(path, copy_policy(&dump_policy), SerializationMethod::Bin) {
      Ok(pl) => pl,
      Err(e) => {
        if let pickledb::error::ErrorType::Io = e.get_type() {
          PickleDb::new(path, dump_policy, SerializationMethod::Bin)
        } else {
          return Err(e);
        }
      },
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

  fn all_strands(&self) -> Result<Vec<Arc<Strand>>, ResolutionError> {
    let lock = self.pickle.lock().expect("Lock on pickle db");
    if !lock.lexists("strands") {
      return Ok(vec![]);
    }
    lock.liter("strands")
      .map(|v| v.get_item::<BlockRecord>().ok_or(ResolutionError::BadData("Could not deserialize from DB correctly".to_string())))
      .map(|v| v.and_then(|v| v.try_into().map_err(|e: VerificationError| e.into())))
      .collect()
  }

  fn get_strand(&self, cid: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    self.all_strands()?
      .into_iter()
      .find(|s| s.cid() == *cid)
      .ok_or(ResolutionError::NotFound)
  }

  fn get_tixel(&self, cid: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    let record: BlockRecord = self.pickle.lock().expect("Lock on pickle db").get(&format!("{}", cid)).ok_or(ResolutionError::NotFound)?;
    record.try_into().map_err(|e: VerificationError| e.into())
  }

  fn has_tixel(&self, cid: &Cid) -> bool {
    self.pickle.lock().expect("Lock on pickle db").exists(&format!("{}", cid))
  }

  fn cid_for_index<S: AsCid>(&self, strand: S, index: u64) -> Option<Cid> {
    self.pickle.lock().expect("Lock on pickle db").lget(&format!("tixels:{}", strand.as_cid()), index as usize)
  }

  // Inclusive range
  fn cid_range<S: AsCid>(&self, strand: S, start: u64, end: u64) -> Result<Vec<Cid>, ResolutionError> {
    self.pickle.lock().expect("Lock on pickle db").liter(&format!("tixels:{}", strand.as_cid()))
      .skip(start as usize)
      .take((end - start + 1) as usize)
      .map(|v| v.get_item::<Cid>().ok_or(ResolutionError::BadData("Could not deserialize from DB correctly".to_string())))
      .collect()
  }

  fn latest_entry<S: AsCid>(&self, strand: S) -> Result<Arc<Tixel>, ResolutionError> {
    let cid = {
      let lock = self.pickle.lock().expect("Lock on pickle db");
      let item = lock.liter(&format!("tixels:{}", strand.as_cid()))
        .last()
        .ok_or(ResolutionError::NotFound)?;
      item.get_item::<Cid>().ok_or(ResolutionError::BadData("Could not deserialize from DB correctly".to_string()))?
    };
    self.get_tixel(&cid)
  }

  fn latest_index<S: AsCid>(&self, strand: S) -> Option<u64> {
    let len = self.pickle.lock().expect("Lock on pickle db").llen(&format!("tixels:{}", strand.as_cid()));
    if len == 0 {
      return None;
    }
    Some(len as u64 - 1)
  }

  fn save_tixel(&self, tixel: Arc<Tixel>) -> Result<(), StoreError> {
    // ensure we have the strand
    if self.get_strand(&tixel.strand_cid()).is_err() {
      return Err(StoreError::Saving("Strand must be saved before tixels".to_string()));
    }
    if tixel.index() != 0 && !self.has_tixel(&tixel.previous().unwrap().tixel) {
      return Err(StoreError::Saving("Previous tixel must be saved before this one".to_string()));
    }
    if self.has_tixel(tixel.as_cid()) {
      return Ok(());
    }
    let cid = tixel.as_cid();
    let mut lock = self.pickle.lock().expect("Lock on pickle db");
    push_list(&mut lock, &format!("tixels:{}", tixel.strand_cid()), &cid)?;
    lock.set(&format!("{}", cid), &BlockRecord::from(tixel)).map_err(|e| StoreError::Saving(e.to_string()))?;
    Ok(())
  }

  fn save_strand(&self, strand: Arc<Strand>) -> Result<(), StoreError> {
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
    self.pickle.lock().expect("Lock on pickle db").liter(&format!("tixels:{}", cid))
      .map(|v| {
        let block = v.get_item::<BlockRecord>().ok_or(ResolutionError::BadData("Could not deserialize from DB correctly".to_string()))?;
        lock.rem(&format!("{}", block.cid)).map_err(|e| StoreError::Saving(e.to_string()))?;
        Ok::<_, StoreError>(())
      })
      .collect::<Result<_, _>>()?;
    lock.lrem_value("strands", &BlockRecord::from(strand)).map_err(|e| StoreError::Saving(e.to_string()))?;
    Ok(())
  }

}

#[async_trait]
impl BaseResolver for PickleDbStore {

  async fn fetch_strands(&self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + '_>>, ResolutionError> {
    let strands = self.all_strands()?;
    Ok(Box::pin(futures::stream::iter(strands.into_iter().map(Ok))))
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    self.get_strand(cid).map(|_| true).or_else(|_| Ok(self.has_tixel(cid)))
  }

  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    Ok(self.cid_for_index(strand, index).is_some())
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    Ok(self.has_tixel(cid) && self.get_strand(strand).is_ok())
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError> {
    self.get_strand(strand)
  }

  async fn fetch_tixel(&self, _strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    self.get_tixel(tixel)
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError> {
    let cid = self.cid_for_index(strand, index).ok_or(ResolutionError::NotFound)?;
    self.get_tixel(&cid)
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError> {
    self.latest_entry(strand)
  }

  async fn range_stream(&self, range: AbsoluteRange) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + '_>>, ResolutionError> {
    let cids = self.cid_range(range.strand, range.start, range.end)?;
    Ok(Box::pin(futures::stream::iter(cids.into_iter().map(|cid| self.get_tixel(&cid)))))
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

  async fn save_many<I: Into<AnyTwine> + Send, S: Iterator<Item = I> + Send, T: IntoIterator<Item = I, IntoIter = S> + Send>(&self, twines: T) -> Result<(), StoreError> {
    for twine in twines {
      self.save(twine).await?;
    }
    Ok(())
  }

  async fn save_stream<I: Into<AnyTwine> + Send, T: Stream<Item = I> + Send + Unpin>(&self, twines: T) -> Result<(), StoreError> {
    twines
      .then(|t| self.save(t))
      .try_collect::<Vec<_>>()
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
