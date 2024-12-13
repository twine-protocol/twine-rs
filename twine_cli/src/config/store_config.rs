use std::{collections::HashSet, hash::Hash, ops::Deref, str::FromStr, sync::Arc};
use futures::executor;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use twine_core::{errors::StoreError, resolver::unchecked_base, store::Store};
use twine_http_store::reqwest;
use twine_sled_store::SledStore;

#[derive(Debug, Clone)]
pub(crate) enum AnyStore {
  HttpV1(twine_http_store::v1::HttpStore),
  HttpV2(twine_http_store::v2::HttpStore),
  Sled(Arc<SledStore>),
}

impl Deref for AnyStore {
  type Target = dyn unchecked_base::BaseResolver;

  fn deref(&self) -> &Self::Target {
    match self {
      Self::HttpV1(s) => s,
      Self::HttpV2(s) => s,
      Self::Sled(s) => s,
    }
  }
}

impl AnyStore {
  async fn save<T: Into<twine_core::twine::AnyTwine> + Send>(&self, twine: T) -> std::result::Result<(), StoreError> {
    match self {
      Self::HttpV1(s) => s.save(twine).await,
      Self::HttpV2(s) => s.save(twine).await,
      Self::Sled(s) => s.save(twine).await,
    }
  }

  async fn save_many<I: Into<twine_core::twine::AnyTwine> + Send, S: Iterator<Item = I> + Send, T: IntoIterator<Item = I, IntoIter = S> + Send>(&self, twines: T) -> std::result::Result<(), StoreError> {
    match self {
      Self::HttpV1(s) => s.save_many(twines).await,
      Self::HttpV2(s) => s.save_many(twines).await,
      Self::Sled(s) => s.save_many(twines).await,
    }
  }

  async fn save_stream<I: Into<twine_core::twine::AnyTwine> + Send, T: futures::stream::Stream<Item = I> + Send + Unpin>(&self, twines: T) -> std::result::Result<(), StoreError> {
    match self {
      Self::HttpV1(s) => s.save_stream(twines).await,
      Self::HttpV2(s) => s.save_stream(twines).await,
      Self::Sled(s) => s.save_stream(twines).await,
    }
  }

  async fn delete<C: twine_core::as_cid::AsCid + Send>(&self, cid: C) -> std::result::Result<(), StoreError> {
    match self {
      Self::HttpV1(s) => s.delete(cid).await,
      Self::HttpV2(s) => s.delete(cid).await,
      Self::Sled(s) => s.delete(cid).await,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoreKind {
  HttpV1,
  HttpV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoreRecord {
  pub uri: String,
  pub kind: StoreKind,
  pub name: Option<String>,
  pub default: bool,
}

impl StoreRecord {
  pub(crate) fn try_new(uri: String, name: Option<String>, default: bool) -> Result<Self> {
    // determine the kind
    let kind = match uri.split("://").next().unwrap_or_default() {
      "http"|"https" => {
        executor::block_on(twine_http_store::determine_version(&uri)).map_or(StoreKind::HttpV1, |v| {
          if v == 2 {
            StoreKind::HttpV2
          } else {
            StoreKind::HttpV1
          }
        })
      },
      _ => return Err(anyhow::anyhow!("Unknown store type: {}", uri)),
    };
    Ok(Self { uri, kind, name, default })
  }

  pub(crate) fn as_store(&self) -> Result<AnyStore> {
    match self.kind {
      StoreKind::HttpV1 => {
        let cfg = twine_http_store::v1::HttpStoreOptions::default()
          .concurency(20)
          .url(&self.uri);
        let r = twine_http_store::v1::HttpStore::new(reqwest::Client::new(), cfg);
        Ok(AnyStore::HttpV1(r))
      },
      StoreKind::HttpV2 => {
        let r = twine_http_store::v2::HttpStore::new(reqwest::Client::new())
          .with_url(&self.uri);
        Ok(AnyStore::HttpV2(r))
      },
    }
  }
}

impl Hash for StoreRecord {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.uri.hash(state);
  }
}

impl PartialEq for StoreRecord {
  fn eq(&self, other: &Self) -> bool {
    self.uri == other.uri
  }
}

impl Eq for StoreRecord {}

impl FromStr for StoreRecord {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self> {
    StoreRecord::try_new(s.to_string(), None, false)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Stores(HashSet<StoreRecord>);

impl Default for Stores {
  fn default() -> Self {
    Self(HashSet::new())
  }
}

impl Stores {
  pub(crate) fn add_store(&mut self, uri: String, name: Option<String>, default: bool) -> Result<()> {
    if let Some(name) = name.as_deref() {
      // name should only contain alphanumeric characters, dashes, and underscores
      if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(anyhow::anyhow!("Store name can only contain alphanumeric characters, dashes, and underscores"));
      }
      // Check if store with name already exists... and return error. (None doesn't count)
      if self.0.iter().any(|r| r.name.as_deref() == Some(name)) {
        return Err(anyhow::anyhow!("Store with name {} already exists", name));
      }

      // can't be named "local"
      if name == "local" {
        return Err(anyhow::anyhow!("Store name cannot be 'local'"));
      }
    }

    if uri.starts_with("http:") {
      log::warn!("Using HTTP without TLS is insecure. Consider using HTTPS.");
    }

    let mut record = StoreRecord::try_new(uri, name, default)?;
    let existing = self.0.get(&record).clone();
    if let Some(existing) = existing {
      record.name = record.name.or(existing.name.clone());
      record.default = existing.default;
    }

    self.0.replace(record.clone());

    if default {
      record.default = true;
      self.set_default(&record);
    }

    match &record.name {
      Some(name) => log::info!("Added store {} with name {}", record.uri, name),
      None => log::info!("Added store {}", record.uri),
    }

    Ok(())
  }

  pub(crate) fn remove_store(&mut self, uri_or_name_or_index: &str) -> Result<()> {
    let maybe_index = uri_or_name_or_index.parse::<usize>().ok();
    if let Some(index) = maybe_index {
      if index >= self.0.len() {
        return Err(anyhow::anyhow!("Index out of bounds"));
      }
      let record = self.0.iter().nth(index).unwrap().clone();
      self.0.remove(&record);
    } else {
      self.0.retain(|r|
        r.uri != uri_or_name_or_index &&
        r.name.as_deref() != Some(uri_or_name_or_index)
      );
    }
    Ok(())
  }

  pub(crate) fn get_default(&self) -> Option<&StoreRecord> {
    self.0.iter().find(|r| r.default)
  }

  pub(crate) fn get(&self, name_or_uri: &str) -> Option<&StoreRecord> {
    self.0.iter().find(|r| r.name.as_deref() == Some(name_or_uri) || r.uri == name_or_uri)
  }

  pub(crate) fn set_default(&mut self, record: &StoreRecord) {
    let mut vec: Vec<StoreRecord> = self.0.iter().cloned().collect();
    vec.iter_mut()
      .for_each(|r| {
        r.default = r == record;
      });
    vec.into_iter()
      .for_each(|r| { self.0.replace(r); });
  }

  pub(crate) fn iter(&self) -> impl Iterator<Item = &StoreRecord> {
    self.0.iter()
  }

  pub(crate) fn len(&self) -> usize {
    self.0.len()
  }
}
