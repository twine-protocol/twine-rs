//! Utilities for retrieving twine data
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

/// A module containing the [`BaseResolver`] trait that is
/// meant to be implemented by any type that wants to be
/// used as a Twine Resolver.
pub mod unchecked_base;
use unchecked_base::*;

/// Optional Send trait which is not Send on wasm32
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
/// Optional Send trait which is not Send on wasm32
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}

#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T where T: {}

#[cfg(not(target_arch = "wasm32"))]
impl<T> MaybeSend for T where T: Send {}

/// This is a standardized interface for retrieving Twine objects
///
/// Datastores will implement this trait (indirectly through [`BaseResolver`])
/// to provide a consistent interface for retrieving Twine objects.
///
/// By retrieving Twine objects through this interface, the caller can
/// be sure that the data is consistent and verified for integrity and authenticity.
///
/// The methods of this trait accept arguments that implement the [`Into`] trait
/// or similar. This allows for a variety of ways of using the methods
/// making them more accessible and understandable.
///
/// # Example
///
/// ```no_run
/// # use twine_lib::{resolver::{Resolver, SingleQuery}, errors::ResolutionError, Cid};
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// use twine_lib::store::MemoryStore;
/// let some_resolver = MemoryStore::default();
/// let some_strand_cid: Cid = "bafyrmieej3j3sprtnbfziv6vhixzr3xxrcabnma43ajb5grhsixdvxzdvu".parse().unwrap();
/// // resolve the latest on the Strand
/// let latest = some_resolver.resolve_latest(some_strand_cid).await?;
/// // or do the same but use a Strand object
/// let strand = some_resolver.resolve_strand(some_strand_cid).await?.unpack();
/// let latest = some_resolver.resolve_latest(&strand).await?;
/// // resolve the 31st Tixel on the Strand
/// let thirty_first = some_resolver.resolve_index(some_strand_cid, 31).await?;
/// // or use an argument that implements Into<Query> like a tuple
/// let thirty_first = some_resolver.resolve((some_strand_cid, 31)).await?;
/// // or parse a Query from a string
/// let query: SingleQuery = "bafyrmieej3j3sprtnbfziv6vhixzr3xxrcabnma43ajb5grhsixdvxzdvu:31".parse().unwrap();
/// let thirty_first = some_resolver.resolve(query).await?;
/// // resolve a stitch
/// let linked = thirty_first.cross_stitches();
/// let some_other_twine = some_resolver.resolve(linked.stitches().get(0).unwrap().clone()).await?;
/// # Ok::<_, ResolutionError>(())
/// # });
/// ```
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Resolver: BaseResolver {
  /// Resolve a Twine object from a query
  ///
  /// # Example
  ///
  /// ```no_run
  /// # use twine_lib::{resolver::{Resolver, SingleQuery}, errors::ResolutionError, Cid};
  /// # let strand_cid = Cid::default();
  /// use twine_lib::store::MemoryStore;
  /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
  /// let some_storage = MemoryStore::default();
  /// // ...
  /// let thirty_first = some_storage.resolve((strand_cid, 31)).await?;
  /// # Ok::<_, ResolutionError>(())
  /// # });
  /// ```
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

  /// Check if a Twine is available for a given query
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

  /// Resolve the Twine data with the highest index for a given Strand
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

  /// Resolve a Twine object by its index on a Strand
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

  /// Resolve a Twine object by a Stitch (a Strand CID and a Tixel CID)
  ///
  /// # Example
  ///
  /// ```no_run
  /// # use twine_lib::{resolver::{Resolver, SingleQuery}, errors::ResolutionError, Cid};
  /// tokio::runtime::Runtime::new().unwrap().block_on(async {
  /// # use twine_lib::store::MemoryStore;
  /// # let resolver = MemoryStore::default();
  /// let strand_cid: Cid = "bafyrmieej3j3sprtnbfziv6vhixzr3xxrcabnma43ajb5grhsixdvxzdvu".parse().unwrap();
  /// let tixel_cid: Cid = "bafyriqgafbhhudahpnzrdvuzjjczro43i4mnv7637vq4oh6m6lfdccpazmmurfu4vluy7iddrhwbbfvjs62uo2wrzx4axaxx5lv7pfmqveqt2".parse().unwrap();
  /// let twine = resolver.resolve_stitch(strand_cid, tixel_cid).await?;
  /// Ok::<_, ResolutionError>(())
  /// # });
  /// ```
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

  /// Resolve a Strand object by its CID
  ///
  /// # Example
  ///
  /// ```no_run
  /// # use twine_lib::{resolver::{Resolver, SingleQuery}, errors::ResolutionError, Cid};
  /// tokio::runtime::Runtime::new().unwrap().block_on(async {
  /// # use twine_lib::store::MemoryStore;
  /// # let resolver = MemoryStore::default();
  /// let cid: Cid = "bafyrmieej3j3sprtnbfziv6vhixzr3xxrcabnma43ajb5grhsixdvxzdvu".parse().unwrap();
  /// let latest = resolver.resolve_latest(cid).await?;
  /// # Ok::<_, ResolutionError>(())
  /// # });
  /// ```
  async fn resolve_strand<C: AsCid + MaybeSend>(
    &self,
    strand: C,
  ) -> Result<StrandResolution, ResolutionError> {
    let strand_cid = strand.as_cid();
    StrandResolution::try_new(*strand_cid, self.fetch_strand(strand_cid).await?)
  }

  /// Resolve a range of Twine objects on a Strand
  ///
  /// This can be supplied as a RangeQuery or any type that implements [`Into<RangeQuery>`]
  ///
  /// # Example
  ///
  /// ```no_run
  /// # use twine_lib::{twine::Twine, resolver::{Resolver, RangeQuery}, errors::ResolutionError, Cid};
  /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
  /// # use twine_lib::store::MemoryStore;
  /// # let resolver = MemoryStore::default();
  /// let cid_strand: Cid = "bafyrmieej3j3sprtnbfziv6vhixzr3xxrcabnma43ajb5grhsixdvxzdvu".parse().unwrap();
  /// // resolve the first 11 (because ranges are inclusive) Tixels on the Strand
  /// let stream = resolver.resolve_range((cid_strand, 0, 10)).await?;
  /// // dump them into a Vec
  /// use futures::stream::TryStreamExt;
  /// let records: Vec<Twine> = stream.try_collect().await?;
  /// # Ok::<_, ResolutionError>(())
  /// # });
  /// ```
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

  /// Get a stream of all available Strand objects
  async fn strands<'a>(&'a self) -> Result<TwineStream<'a, Strand>, ResolutionError> {
    self.fetch_strands().await
  }

  /// Get the latest index of a Strand
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

/// A set of resolvers that are tried in series until one succeeds
///
/// # Example
///
/// ```no_run
/// # use twine_lib::{resolver::{Resolver, ResolverSetSeries}, errors::ResolutionError, Cid};
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// # use twine_lib::store::MemoryStore;
/// use futures::stream::TryStreamExt;
/// # let resolver1 = MemoryStore::default();
/// # let resolver2 = MemoryStore::default();
/// # let resolver3 = MemoryStore::default();
/// let mut resolver = ResolverSetSeries::new_boxed(vec![resolver1, resolver2]);
/// resolver.add_boxed(resolver3);
/// let strands: Vec<_> = resolver.strands().await?.try_collect().await?;
/// # Ok::<_, ResolutionError>(())
/// # });
/// ```
#[derive(Clone)]
pub struct ResolverSetSeries<T>(Vec<T>)
where
  T: BaseResolver;

impl<T> ResolverSetSeries<T>
where
  T: BaseResolver,
{
  /// Create a new ResolverSetSeries from a Vec
  pub fn new(resolvers: Vec<T>) -> Self {
    Self(resolvers)
  }

  /// Add a new resolver to the series
  pub fn add(&mut self, resolver: T) {
    self.0.push(resolver);
  }
}

impl ResolverSetSeries<Box<dyn BaseResolver>> {
  /// Create a new ResolverSetSeries of [`Box`]ed resolvers from a Vec
  pub fn new_boxed<T: BaseResolver + 'static>(resolvers: Vec<T>) -> Self {
    Self(
      resolvers
        .into_iter()
        .map(|r| Box::new(r) as Box<dyn BaseResolver>)
        .collect(),
    )
  }

  /// Add a new resolver to the series by boxing it
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
