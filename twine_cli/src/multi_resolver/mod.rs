use std::ops::Deref;

use twine_http_store::{HttpStore, HttpStoreOptions, reqwest};
use twine_sled_store::{SledStore, SledStoreOptions, sled};
use anyhow::Result;

pub enum MultiResolver {
  Http(HttpStore),
  Sled(SledStore),
}

impl MultiResolver {
  pub fn new_from_string(s: &str) -> Result<Self> {
    match s {
      "http" => {
        let cfg = HttpStoreOptions::default()
          .url(s);
        let r = HttpStore::new(reqwest::Client::new(), cfg);
        Ok(Self::Http(r))
      },
      "sled" => {
        let path = s.split_at(5).1;
        let db = sled::Config::new().path(path).open()?;
        let r = SledStore::new(db, SledStoreOptions::default());
        Ok(Self::Sled(r))
      },
      _ => Err(anyhow::anyhow!("Unknown resolver type: {}", s)),
    }
  }
}


