use anyhow::{anyhow, Result};
use futures::executor;
use std::fmt::Display;
use std::{ops::Deref, str::FromStr};
use twine_car_store::CarStore;
use twine_core::resolver::ResolverSetSeries;
use twine_core::{errors::StoreError, resolver::unchecked_base, store::Store};
use twine_http_store::reqwest;
use twine_pickledb_store::PickleDbStore;
use twine_sled_store::{SledStore, SledStoreOptions};

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct StoreUri {
  pub scheme: String,
  pub path: String,
}

impl FromStr for StoreUri {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self> {
    match s.split("://").collect::<Vec<&str>>().as_slice() {
      [scheme, path] => Ok(Self {
        scheme: scheme.to_string(),
        path: path.to_string(),
      }),
      _ => Err(anyhow!("Invalid store uri: {}", s)),
    }
  }
}

impl Display for StoreUri {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}://{}", self.scheme, self.path)
  }
}

#[derive(Debug, Clone)]
pub enum AnyStore {
  Sled(SledStore),
  Car(CarStore),
  Pickle(PickleDbStore),
  HttpV1(twine_http_store::v1::HttpStore),
  HttpV2(twine_http_store::v2::HttpStore),
}

impl TryFrom<StoreUri> for AnyStore {
  type Error = anyhow::Error;

  fn try_from(uri: StoreUri) -> Result<Self> {
    parse_store(&uri.to_string())
  }
}

impl Deref for AnyStore {
  type Target = dyn unchecked_base::BaseResolver;

  fn deref(&self) -> &Self::Target {
    match self {
      Self::Sled(s) => s,
      Self::Car(s) => s,
      Self::Pickle(s) => s,
      Self::HttpV1(s) => s,
      Self::HttpV2(s) => s,
    }
  }
}

impl AsRef<dyn unchecked_base::BaseResolver> for AnyStore {
  fn as_ref(&self) -> &(dyn unchecked_base::BaseResolver + 'static) {
    match self {
      Self::Sled(s) => s,
      Self::Car(s) => s,
      Self::Pickle(s) => s,
      Self::HttpV1(s) => s,
      Self::HttpV2(s) => s,
    }
  }
}

#[allow(dead_code)]
impl AnyStore {
  pub async fn save<T: Into<twine_core::twine::AnyTwine> + Send>(
    &self,
    twine: T,
  ) -> std::result::Result<(), StoreError> {
    match self {
      Self::Sled(s) => s.save(twine).await,
      Self::Car(s) => s.save(twine).await,
      Self::Pickle(s) => s.save(twine).await,
      Self::HttpV1(s) => s.save(twine).await,
      Self::HttpV2(s) => s.save(twine).await,
    }
  }

  pub async fn save_many<
    I: Into<twine_core::twine::AnyTwine> + Send,
    S: Iterator<Item = I> + Send,
    T: IntoIterator<Item = I, IntoIter = S> + Send,
  >(
    &self,
    twines: T,
  ) -> std::result::Result<(), StoreError> {
    match self {
      Self::Sled(s) => s.save_many(twines).await,
      Self::Car(s) => s.save_many(twines).await,
      Self::Pickle(s) => s.save_many(twines).await,
      Self::HttpV1(s) => s.save_many(twines).await,
      Self::HttpV2(s) => s.save_many(twines).await,
    }
  }

  pub async fn save_stream<
    I: Into<twine_core::twine::AnyTwine> + Send,
    T: futures::stream::Stream<Item = I> + Send + Unpin,
  >(
    &self,
    twines: T,
  ) -> std::result::Result<(), StoreError> {
    match self {
      Self::Sled(s) => s.save_stream(twines).await,
      Self::Car(s) => s.save_stream(twines).await,
      Self::Pickle(s) => s.save_stream(twines).await,
      Self::HttpV1(s) => s.save_stream(twines).await,
      Self::HttpV2(s) => s.save_stream(twines).await,
    }
  }

  pub async fn delete<C: twine_core::as_cid::AsCid + Send>(
    &self,
    cid: C,
  ) -> std::result::Result<(), StoreError> {
    match self {
      Self::Sled(s) => s.delete(cid).await,
      Self::Car(s) => s.delete(cid).await,
      Self::Pickle(s) => s.delete(cid).await,
      Self::HttpV1(s) => s.delete(cid).await,
      Self::HttpV2(s) => s.delete(cid).await,
    }
  }
}

pub fn parse_store(uri: &str) -> Result<AnyStore> {
  match uri.split("://").collect::<Vec<&str>>().as_slice() {
    [scheme, path] => match *scheme {
      "sled" => {
        let db = twine_sled_store::sled::Config::new().path(path).open()?;
        Ok(AnyStore::Sled(SledStore::new(
          db,
          SledStoreOptions::default(),
        )))
      }
      "car" => Ok(AnyStore::Car(CarStore::new(path)?)),
      "pickle" => Ok(AnyStore::Pickle(PickleDbStore::new(path)?)),
      "http" | "https" => {
        match executor::block_on(twine_http_store::determine_version(&uri)).unwrap_or(1) {
          1 => {
            let cfg = twine_http_store::v1::HttpStoreOptions::default()
              .concurency(20)
              .url(&uri);
            let r = twine_http_store::v1::HttpStore::new(reqwest::Client::new(), cfg);
            Ok(AnyStore::HttpV1(r))
          }
          2 => {
            let r = twine_http_store::v2::HttpStore::new(reqwest::Client::new()).with_url(&uri);
            Ok(AnyStore::HttpV2(r))
          }
          _ => Err(anyhow!("Invalid HTTP store version: {}", uri)),
        }
      }
      _ => Err(anyhow!("Invalid store specifier: {}", uri)),
    },
    [path] => {
      // try to detect file from extension
      if path.ends_with(".car") {
        Ok(AnyStore::Car(CarStore::new(path)?))
      } else if path.ends_with(".sled") {
        Ok(AnyStore::Sled(SledStore::new(
          twine_sled_store::sled::Config::new().path(path).open()?,
          SledStoreOptions::default(),
        )))
      } else if path.ends_with(".pickle") {
        Ok(AnyStore::Pickle(PickleDbStore::new(path)?))
      } else {
        Err(anyhow!("Could not determine type of store: {}", uri))
      }
    }
    _ => Err(anyhow!("Invalid store specifier: {}", uri)),
  }
}

pub fn resolver_from_args(
  arg: &Option<String>,
  config: &Option<Config>,
) -> Result<ResolverSetSeries<AnyStore>> {
  // Can be either the arg as a store uri (precedence)
  // or the name of the store in the config
  // or default to all stores in config
  if let Some(arg) = arg {
    let store = match parse_store(&arg) {
      Ok(s) => s,
      Err(_) => config
        .as_ref()
        .ok_or_else(|| anyhow!("Could not parse resolver uri and no config present"))?
        .get_named_resolver(&arg)
        .ok_or_else(|| anyhow!("No resolver named: {}", arg))??,
    };
    Ok(ResolverSetSeries::new(vec![store]))
  } else if let Some(config) = config {
    config.all_resolvers()
  } else {
    Err(anyhow!(
      "Must specify a resolver in arguments or in config file"
    ))
  }
}
