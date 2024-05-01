use std::{ops::RangeBounds, sync::Arc};
use thiserror::Error;
use libipld::Cid;
use async_trait::async_trait;
use crate::{prelude::{AnyTwine, Stitch, Strand, Tixel, Twine, VerificationError}, as_cid::AsCid};

#[derive(Error, Debug)]
pub enum ResolutionError {
  #[error("Twine not found")]
  NotFound,
  #[error("Twine is invalid")]
  Invalid(#[from] VerificationError),
  #[error("Twine has wrong type: expected {expected}, found {found}")]
  WrongType {
    expected: String,
    found: String,
  },
  #[error("Bad data: {0}")]
  BadData(String),
  #[error("Problem fetching data: {0}")]
  Fetch(String),
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum Query {
  Stitch(Stitch),
  Index(Cid, u64),
  Latest(Cid),
}

impl From<Stitch> for Query {
  fn from(stitch: Stitch) -> Self {
    Self::Stitch(stitch)
  }
}

impl From<Tixel> for Query {
  fn from(tixel: Tixel) -> Self {
    tixel.into()
  }
}

impl From<Strand> for Query {
  fn from(strand: Strand) -> Self {
    Self::Latest(strand.into())
  }
}

impl<C> From<(C, u64)> for Query where C: AsCid {
  fn from((strand, index): (C, u64)) -> Self {
    Self::Index(strand.as_cid().clone(), index)
  }
}

#[async_trait]
pub trait Resolver {
  async fn resolve_cid<C: AsCid + Send>(&self, cid: C) -> Result<AnyTwine, ResolutionError>;
  async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: u64) -> Result<Twine, ResolutionError>;
  async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError>;

  async fn resolve<Q: Into<Query> + Send>(&self, query: Q) -> Result<Twine, ResolutionError> {
    let query = query.into();
    match query {
      Query::Stitch(stitch) => {
        let strand = self.resolve_strand(stitch.strand);
        let tixel = self.resolve_tixel(stitch.tixel);
        let (strand, tixel) = futures::try_join!(strand, tixel)?;
        Ok(Twine::try_new_from_shared(strand, tixel)?)
      }
      Query::Index(strand, index) => self.resolve_index(strand, index).await,
      Query::Latest(strand) => self.resolve_latest(strand).await,
    }
  }
  async fn resolve_tixel<C: AsCid + Send>(&self, tixel: C) -> Result<Arc<Tixel>, ResolutionError> {
    let twine = self.resolve_cid(tixel).await?;
    match twine {
      AnyTwine::Tixel(tixel) => Ok(tixel),
      AnyTwine::Strand(_) => Err(ResolutionError::WrongType {
        expected: "Tixel".to_string(),
        found: "Strand".to_string(),
      }),
    }
  }
  async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
    let task = self.resolve_cid(strand);
    let twine = task.await?;
    match twine {
      AnyTwine::Strand(strand) => Ok(strand),
      AnyTwine::Tixel(_) => Err(ResolutionError::WrongType {
        expected: "Strand".to_string(),
        found: "Twine".to_string(),
      }),
    }
  }

  async fn resolve_range<C: AsCid + Send, R: RangeBounds<u64> + Send>(&self, strand: C, range: R) -> Result<Vec<Twine>, ResolutionError>;
}

