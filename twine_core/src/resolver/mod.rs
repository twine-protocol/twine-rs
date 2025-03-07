use crate::as_cid::AsCid;
use crate::errors::ResolutionError;
use crate::twine::{Strand, Tixel, Twine};
use crate::Cid;
use async_trait::async_trait;
use futures::{Stream, StreamExt, TryStreamExt};
use std::collections::HashSet;
use std::pin::Pin;

mod query;
pub use query::*;

mod resolution;
pub use resolution::*;

pub mod unchecked_base;
use unchecked_base::*;

#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}

#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T where T: {}

#[cfg(not(target_arch = "wasm32"))]
impl<T> MaybeSend for T where T: Send {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Resolver: BaseResolver {
  async fn resolve<Q: Into<SingleQuery> + MaybeSend>(
    &self,
    query: Q,
  ) -> Result<TwineResolution, ResolutionError> {
    let query = query.into();
    match query {
      SingleQuery::Stitch(stitch) => self.resolve_stitch(stitch.strand, stitch.tixel).await,
      SingleQuery::Index(strand, index) if index == -1 => self.resolve_latest(strand).await,
      SingleQuery::Index(strand, index) => {
        let index = match index {
          i if i < 0 => self.fetch_latest(strand.as_cid()).await?.index() as i64 + i + 1,
          i => i,
        } as u64;
        self.resolve_index(strand, index).await
      }
      SingleQuery::Latest(strand) => self.resolve_latest(strand).await,
    }
  }

  async fn has<Q: Into<SingleQuery> + MaybeSend>(&self, query: Q) -> Result<bool, ResolutionError> {
    let query = query.into();
    match query {
      SingleQuery::Stitch(stitch) => {
        self
          .has_twine(stitch.strand.as_cid(), stitch.tixel.as_cid())
          .await
      }
      SingleQuery::Index(strand, index) if index == -1 => {
        match self.fetch_latest(strand.as_cid()).await {
          Ok(_) => Ok(true),
          Err(ResolutionError::NotFound) => Ok(false),
          Err(e) => Err(e),
        }
      }
      SingleQuery::Index(strand, index) => {
        let index = match index {
          i if i < 0 => self.fetch_latest(strand.as_cid()).await?.index() as i64 + i + 1,
          i => i,
        } as u64;
        self.has_index(strand.as_cid(), index).await
      }
      SingleQuery::Latest(strand) => match self.fetch_latest(strand.as_cid()).await {
        Ok(_) => Ok(true),
        Err(ResolutionError::NotFound) => Ok(false),
        Err(e) => Err(e),
      },
    }
  }

  async fn resolve_latest<C: AsCid + MaybeSend>(
    &self,
    strand: C,
  ) -> Result<TwineResolution, ResolutionError> {
    use futures::join;
    let strand_cid = strand.as_cid();
    let (strand, tixel) = join!(self.fetch_strand(strand_cid), self.fetch_latest(strand_cid));
    TwineResolution::try_new(
      SingleQuery::Latest(*strand_cid),
      Twine::try_new(strand?, tixel?)?,
    )
  }

  async fn resolve_index<C: AsCid + MaybeSend>(
    &self,
    strand: C,
    index: u64,
  ) -> Result<TwineResolution, ResolutionError> {
    use futures::join;
    let strand_cid = strand.as_cid();
    let (strand, tixel) = join!(
      self.fetch_strand(strand_cid),
      self.fetch_index(strand_cid, index)
    );
    TwineResolution::try_new(
      SingleQuery::Index(*strand_cid, index as i64),
      Twine::try_new(strand?, tixel?)?,
    )
  }

  async fn resolve_stitch<C: AsCid + MaybeSend>(
    &self,
    strand: C,
    tixel: C,
  ) -> Result<TwineResolution, ResolutionError> {
    use futures::join;
    let strand_cid = strand.as_cid();
    let tixel_cid = tixel.as_cid();
    let (strand, tixel) = join!(
      self.fetch_strand(strand_cid),
      self.fetch_tixel(strand_cid, tixel_cid)
    );
    TwineResolution::try_new(
      SingleQuery::Stitch((*strand_cid, *tixel_cid).into()),
      Twine::try_new(strand?, tixel?)?,
    )
  }

  async fn resolve_strand<C: AsCid + MaybeSend>(
    &self,
    strand: C,
  ) -> Result<StrandResolution, ResolutionError> {
    let strand_cid = strand.as_cid();
    StrandResolution::try_new(*strand_cid, self.fetch_strand(strand_cid).await?)
  }

  async fn resolve_range<'a, R: Into<RangeQuery> + MaybeSend>(
    &'a self,
    range: R,
  ) -> Result<TwineStream<'a, Twine>, ResolutionError> {
    let range = range.into();
    let latest = self.resolve_latest(range.strand_cid()).await?.unpack();
    let range = range.to_absolute(latest.index());
    if range.is_none() {
      let s = futures::stream::empty().boxed();
      #[cfg(target_arch = "wasm32")]
      {
        return Ok(s.boxed_local());
      }
      #[cfg(not(target_arch = "wasm32"))]
      {
        return Ok(s.boxed());
      }
    }
    let range = range.unwrap();
    if range.len() == 1 {
      let s = futures::stream::once({
        let strand_cid = range.strand_cid().clone();
        async move {
          let tixel = self.fetch_index(&strand_cid, range.start).await?;
          if tixel.index() != range.start {
            return Err(ResolutionError::Fetch(format!(
              "index mismatch (expected: {}, got: {})",
              tixel.index(),
              range.start
            )));
          }
          Twine::try_new(latest.strand().clone(), tixel).map_err(|e| e.into())
        }
      });
      #[cfg(target_arch = "wasm32")]
      {
        return Ok(s.boxed_local());
      }
      #[cfg(not(target_arch = "wasm32"))]
      {
        return Ok(s.boxed());
      }
    }
    let expected = range.clone().iter();
    let s = self
      .range_stream(range)
      .await?
      .zip(futures::stream::iter(expected))
      .map(move |(tixel, q)| {
        let tixel = tixel?;
        if tixel.index() != q.unwrap_index() as u64 {
          return Err(ResolutionError::Fetch(format!(
            "index mismatch (expected: {}, got: {})",
            q.unwrap_index(),
            tixel.index()
          )));
        }
        Twine::try_new(latest.strand().clone(), tixel).map_err(|e| e.into())
      });
    #[cfg(target_arch = "wasm32")]
    {
      Ok(s.boxed_local())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      Ok(s.boxed())
    }
  }

  async fn strands<'a>(&'a self) -> Result<TwineStream<'a, Strand>, ResolutionError> {
    self.fetch_strands().await
  }

  async fn latest_index(&self, strand: &Cid) -> Result<u64, ResolutionError> {
    Ok(self.fetch_latest(strand).await?.index())
  }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T> BaseResolver for T
where
  T: AsRef<dyn BaseResolver> + BaseResolverBounds,
{
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    self.as_ref().has_index(strand, index).await
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    self.as_ref().has_twine(strand, cid).await
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    self.as_ref().has_strand(cid).await
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
    self.as_ref().fetch_latest(strand).await
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
    self.as_ref().fetch_index(strand, index).await
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
    self.as_ref().fetch_tixel(strand, tixel).await
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
    self.as_ref().fetch_strand(strand).await
  }

  async fn range_stream<'a>(
    &'a self,
    range: AbsoluteRange,
  ) -> Result<TwineStream<'a, Tixel>, ResolutionError> {
    self.as_ref().range_stream(range).await
  }

  async fn fetch_strands<'a>(&'a self) -> Result<TwineStream<'a, Strand>, ResolutionError> {
    self.as_ref().fetch_strands().await
  }
}

impl<T> Resolver for T where T: AsRef<dyn BaseResolver> + BaseResolverBounds {}

#[derive(Clone)]
pub struct ResolverSetSeries<T>(Vec<T>)
where
  T: BaseResolver;

impl<T> ResolverSetSeries<T>
where
  T: BaseResolver,
{
  pub fn new(resolvers: Vec<T>) -> Self {
    Self(resolvers)
  }

  pub fn add(&mut self, resolver: T) {
    self.0.push(resolver);
  }
}

impl ResolverSetSeries<Box<dyn BaseResolver>> {
  pub fn new_boxed<T: BaseResolver + 'static>(resolvers: Vec<T>) -> Self {
    Self(
      resolvers
        .into_iter()
        .map(|r| Box::new(r) as Box<dyn BaseResolver>)
        .collect(),
    )
  }

  pub fn add_boxed<T: BaseResolver + 'static>(&mut self, resolver: T) {
    self.add(Box::new(resolver));
  }
}

impl<T> Default for ResolverSetSeries<T>
where
  T: BaseResolver,
{
  fn default() -> Self {
    Self(Vec::new())
  }
}

impl<T> std::ops::Deref for ResolverSetSeries<T>
where
  T: BaseResolver,
{
  type Target = Vec<T>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

// TODO: Error handling is confusing since if resolvers fail
// for a different reason the result will still be NotFound
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T> BaseResolver for ResolverSetSeries<T>
where
  T: BaseResolver,
{
  async fn has_index(&self, strand: &Cid, index: u64) -> Result<bool, ResolutionError> {
    let res = futures::stream::iter(self.iter())
      .then(|r| r.has_index(strand, index))
      .any(|res| match res {
        Ok(true) => futures::future::ready(true),
        Ok(false) => futures::future::ready(false),
        Err(e) => {
          log::debug!("error from resolver while executing has_index: {}", e);
          futures::future::ready(false)
        }
      })
      .await;
    Ok(res)
  }

  async fn has_twine(&self, strand: &Cid, cid: &Cid) -> Result<bool, ResolutionError> {
    let res = futures::stream::iter(self.iter())
      .then(|r| r.has_twine(strand, cid))
      .any(|res| match res {
        Ok(true) => futures::future::ready(true),
        Ok(false) => futures::future::ready(false),
        Err(e) => {
          log::debug!("error from resolver while executing has_twine: {}", e);
          futures::future::ready(false)
        }
      })
      .await;
    Ok(res)
  }

  async fn has_strand(&self, cid: &Cid) -> Result<bool, ResolutionError> {
    let res = futures::stream::iter(self.iter())
      .then(|r| r.has_strand(cid))
      .any(|res| match res {
        Ok(true) => futures::future::ready(true),
        Ok(false) => futures::future::ready(false),
        Err(e) => {
          log::debug!("error from resolver while executing has_strand: {}", e);
          futures::future::ready(false)
        }
      })
      .await;
    Ok(res)
  }

  async fn fetch_latest(&self, strand: &Cid) -> Result<Tixel, ResolutionError> {
    let tasks = self
      .iter()
      .map(|r| r.fetch_latest(strand))
      .collect::<Vec<_>>();
    let results = futures::future::join_all(tasks)
      .await
      .into_iter()
      .filter_map(|res| match res {
        Ok(t) => Some(t),
        Err(_) => None,
      })
      .max_by(|a, b| a.index().cmp(&b.index()));
    match results {
      Some(t) => Ok(t),
      None => Err(ResolutionError::NotFound),
    }
  }

  async fn fetch_index(&self, strand: &Cid, index: u64) -> Result<Tixel, ResolutionError> {
    for resolver in self.iter() {
      if let Ok(tixel) = resolver.fetch_index(strand, index).await {
        return Ok(tixel);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn fetch_tixel(&self, strand: &Cid, tixel: &Cid) -> Result<Tixel, ResolutionError> {
    for resolver in self.iter() {
      if let Ok(t) = resolver.fetch_tixel(strand, tixel).await {
        return Ok(t);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn fetch_strand(&self, strand: &Cid) -> Result<Strand, ResolutionError> {
    for resolver in self.iter() {
      if let Ok(s) = resolver.fetch_strand(strand).await {
        return Ok(s);
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn range_stream<'a>(
    &'a self,
    range: AbsoluteRange,
  ) -> Result<TwineStream<'a, Tixel>, ResolutionError> {
    for resolver in self.iter() {
      // TODO: should find a way to merge streams
      if resolver.has_index(range.strand_cid(), range.start).await? {
        if let Ok(stream) = resolver.range_stream(range.into()).await {
          return Ok(stream);
        }
      }
    }
    Err(ResolutionError::NotFound)
  }

  async fn fetch_strands<'a>(&'a self) -> Result<TwineStream<'a, Strand>, ResolutionError> {
    let s = futures::stream::iter(self.iter())
      .map(|r| r.fetch_strands())
      .buffered(10)
      .try_flatten()
      .scan(HashSet::new(), |seen, strand| {
        use futures::future::ready;
        let strand = match strand {
          Ok(s) => s,
          Err(e) => return ready(Some(Err(e))),
        };
        if seen.contains(&strand.cid()) {
          return ready(Some(Ok(None)));
        }
        seen.insert(strand.cid());
        ready(Some(Ok(Some(strand))))
      })
      .filter_map(|res| async move {
        match res {
          Ok(Some(s)) => Some(Ok(s)),
          Ok(None) => None,
          Err(e) => {
            log::debug!("error from resolver while executing strands(): {}", e);
            None
          }
        }
      });

    #[cfg(target_arch = "wasm32")]
    {
      Ok(s.boxed_local())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
      Ok(s.boxed())
    }
  }
}

impl<T> Resolver for ResolverSetSeries<T> where T: BaseResolver {}

#[cfg(test)]
mod test {
  use super::*;
  use crate::{
    store::{MemoryCache, MemoryStore},
    twine::TwineBlock,
  };

  #[tokio::test]
  async fn test_resolver_set_series() {
    let mut resolver = ResolverSetSeries::default();
    let r1 = MemoryCache::new(MemoryStore::default());
    let r2 = MemoryStore::default();
    let r3 = MemoryStore::default();

    resolver.add_boxed(r1);
    resolver.add_boxed(r2);
    resolver.add_boxed(r3.clone());

    assert_eq!(resolver.len(), 3);

    let strand = Strand::from_tagged_dag_json(crate::test::STRAND_V2_JSON).unwrap();
    let tixel = Tixel::from_tagged_dag_json(crate::test::TIXEL_V2_JSON).unwrap();

    r3.save_sync(strand.clone().into()).unwrap();
    r3.save_sync(tixel.clone().into()).unwrap();

    let strand_cid = strand.cid();
    let tixel_cid = tixel.cid();

    let res = resolver.resolve(strand_cid).await;
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.strand().cid(), strand_cid);
    assert_eq!(res.tixel().cid(), tixel_cid);
  }
}
