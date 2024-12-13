use super::*;

#[async_trait]
pub trait BaseResolver: Send + Sync {
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError>;
  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError>;
  async fn fetch_latest(&self, strand: &Cid) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Arc<Tixel>, ResolutionError>;
  async fn fetch_strand(&self, strand: &Cid) -> Result<Arc<Strand>, ResolutionError>;
  async fn range_stream<'a>(&'a self, range: AbsoluteRange) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Tixel>, ResolutionError>> + Send + 'a>>, ResolutionError>;
  async fn fetch_strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError>;
}
