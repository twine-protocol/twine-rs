use twine_http_store::{HttpStore, HttpStoreOptions, reqwest};
use twine_sled_store::{SledStore, SledStoreOptions, sled};
use anyhow::Result;
use twine_core::{resolver::{Resolver, Query, RangeQuery}, errors::ResolutionError, as_cid::AsCid, twine::{AnyTwine, Twine, Strand, Tixel}};
use async_trait::async_trait;
use futures::stream::Stream;
use std::{pin::Pin, sync::Arc};


macro_rules! poly_resolver {
  ($name:ident, [$($resolver:ident),*]) => {
    #[derive(Debug, Clone)]
    pub enum $name {
      $($resolver($resolver),)*
    }

    #[async_trait]
    impl Resolver for $name {
      async fn resolve_cid<C: AsCid + Send>(&self, cid: C) -> Result<AnyTwine, ResolutionError> {
        match self {
          $(Self::$resolver(r) => r.resolve_cid(cid).await,)*
        }
      }
      async fn resolve_index<C: AsCid + Send>(&self, strand: C, index: i64) -> Result<Twine, ResolutionError> {
        match self {
          $(Self::$resolver(r) => r.resolve_index(strand, index).await,)*
        }
      }
      async fn resolve_latest<C: AsCid + Send>(&self, strand: C) -> Result<Twine, ResolutionError> {
        match self {
          $(Self::$resolver(r) => r.resolve_latest(strand).await,)*
        }
      }
      async fn strands<'a>(&'a self) -> Result<Pin<Box<dyn Stream<Item = Result<Arc<Strand>, ResolutionError>> + Send + 'a>>, ResolutionError> {
        match self {
          $(Self::$resolver(r) => r.strands().await,)*
        }
      }

      async fn has<C: AsCid + Send>(&self, cid: C) -> bool {
        match self {
          $(Self::$resolver(r) => r.has(cid).await,)*
        }
      }

      async fn resolve<Q: Into<Query> + Send>(&self, query: Q) -> Result<Twine, ResolutionError> {
        match self {
          $(Self::$resolver(r) => r.resolve(query).await,)*
        }
      }

      async fn resolve_tixel<C: AsCid + Send>(&self, tixel: C) -> Result<Arc<Tixel>, ResolutionError> {
        match self {
          $(Self::$resolver(r) => r.resolve_tixel(tixel).await,)*
        }
      }

      async fn resolve_strand<C: AsCid + Send>(&self, strand: C) -> Result<Arc<Strand>, ResolutionError> {
        match self {
          $(Self::$resolver(r) => r.resolve_strand(strand).await,)*
        }
      }

      async fn resolve_range<'a, R: Into<RangeQuery> + Send>(&'a self, range: R) -> Result<Pin<Box<dyn Stream<Item = Result<Twine, ResolutionError>> + Send + 'a>>, ResolutionError> {
        match self {
          $(Self::$resolver(r) => r.resolve_range(range).await,)*
        }
      }
    }
  };
}

poly_resolver!(PolyResolver, [HttpStore, SledStore]);

impl PolyResolver {
  pub fn new_from_string(s: &str) -> Result<Self> {
    match s.split("://").next().unwrap_or_default() {
      "http"|"https" => {
        let cfg = HttpStoreOptions::default()
          .url(s);
        let r = HttpStore::new(reqwest::Client::new(), cfg);
        Ok(Self::HttpStore(r))
      },
      "sled" => {
        let path = s.split_at(5).1;
        let db = sled::Config::new().path(path).open()?;
        let r = SledStore::new(db, SledStoreOptions::default());
        Ok(Self::SledStore(r))
      },
      _ => Err(anyhow::anyhow!("Unknown resolver type: {}", s)),
    }
  }
}
