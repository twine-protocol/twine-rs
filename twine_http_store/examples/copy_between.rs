use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
// use tokio::pin;
use twine_core::twine::Strand;
use twine_http_store::*;
use twine_core::resolver::*;
use twine_core::store::MemoryCache;
use twine_core::store::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cfg = v1::HttpStoreOptions::default()
    .url("https://random.colorado.edu/api");
  let resolver = v1::HttpStore::new(reqwest::Client::new(), cfg);
  let resolver = MemoryCache::new(resolver);
  let store = v2::HttpStore::new(
      reqwest::Client::builder()
        .default_headers({
          use reqwest::header::{HeaderValue, AUTHORIZATION};
          let mut headers = reqwest::header::HeaderMap::new();
          headers.insert(AUTHORIZATION, HeaderValue::from_static("ApiKey dev"));
          headers
        })
        .build()?,
    )
    .with_url("http://localhost:8787/");

  println!("strands:");
  let strands = resolver.strands().await?;
  let strands: Vec<Arc<Strand>> = strands.inspect_ok(|strand| {
    println!("> cid: {}\n> description: {:?}",
      strand.cid(),
      strand.details().get("description").unwrap()
    );
  })
  .inspect_err(|err| {
    eprintln!("error: {}", err);
  })
  .try_collect().await?;

  let strand = &(*strands[0]);
  // check if the strand is already in the store
  if !store.has(strand.clone()).await? {
    store.save(strand.clone()).await?;
  }

  // now save first 10 twines
  let stream = resolver.resolve_range((&strands[0], 0..=1000)).await?
    .inspect_ok(|twine| println!("index: {}, cid: {}", twine.index(), twine.cid()))
    .inspect_err(|err| eprintln!("error: {}", err))
    .filter_map(|twine| async {
      twine.ok()
    })
    .boxed();

  // pin!(stream);

  store.save_stream(stream).await?;

  Ok(())
}
