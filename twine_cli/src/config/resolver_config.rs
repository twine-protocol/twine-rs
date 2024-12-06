use std::{collections::HashSet, hash::Hash, str::FromStr};
use futures::executor;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use twine_core::resolver::unsafe_base;
use twine_http_store::reqwest;
use twine_sled_store::{SledStore, SledStoreOptions, sled};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolverKind {
  HttpV1,
  HttpV2,
  Sled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ResolverRecord {
  pub uri: String,
  pub kind: ResolverKind,
  pub name: Option<String>,
  pub priority: Option<u8>,
  pub default: bool,
}

impl ResolverRecord {
  pub(crate) fn try_new(uri: String, name: Option<String>, priority: Option<u8>, default: bool) -> Result<Self> {
    // determine the kind
    let kind = match uri.split("://").next().unwrap_or_default() {
      "http"|"https" => {
        executor::block_on(twine_http_store::determine_version(&uri)).map_or(ResolverKind::HttpV1, |v| {
          if v == 2 {
            ResolverKind::HttpV2
          } else {
            ResolverKind::HttpV1
          }
        })
      },
      "sled" => ResolverKind::Sled,
      _ => return Err(anyhow::anyhow!("Unknown resolver type: {}", uri)),
    };
    Ok(Self { uri, kind, name, priority, default })
  }

  pub(crate) fn as_resolver(&self) -> Result<Box<dyn unsafe_base::BaseResolver>> {
    match self.kind {
      ResolverKind::HttpV1 => {
        let cfg = twine_http_store::v1::HttpStoreOptions::default()
          .concurency(20)
          .url(&self.uri);
        let r = twine_http_store::v1::HttpStore::new(reqwest::Client::new(), cfg);
        Ok(Box::new(r))
      },
      ResolverKind::HttpV2 => {
        let r = twine_http_store::v2::HttpStore::new(reqwest::Client::new())
          .with_url(&self.uri);
        Ok(Box::new(r))
      },
      ResolverKind::Sled => {
        let path = self.uri.split_at(5).1;
        let db = sled::Config::new().path(path).open()?;
        let r = SledStore::new(db, SledStoreOptions::default());
        Ok(Box::new(r))
      },
    }
  }
}

impl Hash for ResolverRecord {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.uri.hash(state);
  }
}

impl PartialEq for ResolverRecord {
  fn eq(&self, other: &Self) -> bool {
    self.uri == other.uri
  }
}

impl Eq for ResolverRecord {}
impl PartialOrd for ResolverRecord {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    // Higher priority should come first
    // None is considered priority 0
    // If priority is the same, sort by uri
    match self.priority.partial_cmp(&other.priority).map(|o| o.reverse()){
      Some(std::cmp::Ordering::Equal)|None => self.uri.partial_cmp(&other.uri),
      order => order,
    }
  }
}

impl Ord for ResolverRecord {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.partial_cmp(other).unwrap()
  }
}

impl FromStr for ResolverRecord {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self> {
    ResolverRecord::try_new(s.to_string(), None, None, false)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Resolvers(HashSet<ResolverRecord>);

impl Default for Resolvers {
  fn default() -> Self {
    Self(HashSet::new())
  }
}

impl Resolvers {
  pub(crate) fn add_resolver(&mut self, uri: String, name: Option<String>, priority: Option<u8>, default: bool) -> Result<()> {
    if let Some(name) = name.as_deref() {
      // name should only contain alphanumeric characters, dashes, and underscores
      if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(anyhow::anyhow!("Resolver name can only contain alphanumeric characters, dashes, and underscores"));
      }
      // Check if resolver with name already exists... and return error. (None doesn't count)
      if self.0.iter().any(|r| r.name.as_deref() == Some(name)) {
        return Err(anyhow::anyhow!("Resolver with name {} already exists", name));
      }

      // can't be named "local"
      if name == "local" {
        return Err(anyhow::anyhow!("Resolver name cannot be 'local'"));
      }
    }

    if uri.starts_with("http:") {
      log::warn!("Using HTTP without TLS is insecure. Consider using HTTPS.");
    }

    let mut record = ResolverRecord::try_new(uri, name, priority, default)?;
    let existing = self.0.get(&record).clone();
    if let Some(existing) = existing {
      record.priority = record.priority.or(existing.priority);
      record.name = record.name.or(existing.name.clone());
      record.default = existing.default;
    }

    self.0.replace(record.clone());

    if default {
      record.default = true;
      self.set_default(&record);
    }

    match &record.name {
      Some(name) => log::info!("Added resolver {} with name {} (priority: {})", record.uri, name, record.priority.unwrap_or(0)),
      None => log::info!("Added resolver {} (priority: {})", record.uri, record.priority.unwrap_or(0)),
    }

    Ok(())
  }

  pub(crate) fn remove_resolver(&mut self, uri_or_name_or_index: &str) -> Result<()> {
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

  pub(crate) fn get_default(&self) -> Option<&ResolverRecord> {
    self.0.iter().find(|r| r.default)
  }

  pub(crate) fn get(&self, name_or_uri: &str) -> Option<&ResolverRecord> {
    self.0.iter().find(|r| r.name.as_deref() == Some(name_or_uri) || r.uri == name_or_uri)
  }

  pub(crate) fn set_default(&mut self, record: &ResolverRecord) {
    let mut vec: Vec<ResolverRecord> = self.0.iter().cloned().collect();
    vec.iter_mut()
      .for_each(|r| {
        r.default = r == record;
      });
    vec.into_iter()
      .for_each(|r| { self.0.replace(r); });
  }

  pub(crate) fn iter(&self) -> impl Iterator<Item = &ResolverRecord> {
    use itertools::Itertools;
    self.0.iter().sorted()
  }

  pub(crate) fn len(&self) -> usize {
    self.0.len()
  }
}
