//! This example shows how to read data from a v1 http store
//! and write it to a memory store.
use std::time::Duration;

use futures::{StreamExt, TryStreamExt};
use tokio::pin;
use twine_lib::resolver::*;
use twine_lib::store::MemoryCache;
use twine_lib::store::Store;
use twine_lib::Cid;
use twine_http_store::*;
// use futures_time::prelude::*;
// use futures_time::time::Duration;
// use futures_time::stream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cfg = v1::HttpStoreOptions::default().url("https://random.colorado.edu/api");
  let resolver = v1::HttpStore::new(
    reqwest::Client::builder()
      .timeout(Duration::from_secs(10))
      .build()?,
    cfg,
  );
  let resolver = MemoryCache::new(resolver);
  let store = twine_lib::store::MemoryStore::new();

  println!("strands:");
  let strands = resolver.strands().await?;
  strands
    .inspect_ok(|strand| {
      println!(
        "> cid: {}\n> description: {:?}",
        strand.cid(),
        strand.details().get("description").unwrap()
      );
    })
    .inspect_err(|err| {
      eprintln!("error: {}", err);
    })
    .for_each(|_| async {})
    .await;

  let cid = Cid::try_from("bafyriqa5k2d3t3r774geicueaed2wc2fosjwqeexfhwbptfgq7rcn5mwucnhfeuxu2nxbrch3rl6yqjlozhuswo5ln3xwjm35iftt3tpqlcgs").unwrap();
  let twine = resolver.resolve_strand(cid).await?.unpack();
  println!("specific strand resolved: {}", twine.cid());

  let tenth = resolver.resolve_index(&twine, 10).await?;
  println!("tenth: {}", tenth.cid());

  let latest = resolver.resolve_latest(&twine).await?;
  println!("latest: {}", latest.cid());

  let twine_stream = resolver
    .resolve_range((&twine, 100..=0))
    .await?
    .inspect_ok(|twine| println!("index: {}, cid: {}", twine.index(), twine.cid()))
    .inspect_err(|err| eprintln!("error: {}", err))
    .filter_map(|twine| async { twine.ok() });

  pin!(twine_stream);

  store.save(twine.clone()).await?;
  println!("saved twine");
  store.save_stream(twine_stream).await?;

  // try sequentially
  // for i in 0..100 {
  //   let twine = resolver.resolve_index(&twine, i).await?;
  //   println!("(cached?) index: {}, cid: {}", twine.index(), twine.cid());
  // }

  Ok(())
}
