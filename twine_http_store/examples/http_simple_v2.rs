//! This example shows how to read data from a v2 http store
//! and write it to a memory store.
use futures::{StreamExt, TryStreamExt};
use twine_lib::resolver::*;
use twine_lib::twine::Strand;
use twine_http_store::*;
// use twine_lib::store::MemoryCache;
// use twine_lib::store::Store;
// use futures_time::prelude::*;
// use futures_time::time::Duration;
// use futures_time::stream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let resolver = v2::HttpStore::new(reqwest::Client::new()).with_url("http://localhost:3000/");
  // let store = twine_lib::store::MemoryStore::new();

  println!("strands:");
  let strands: Vec<Strand> = resolver
    .strands()
    .await?
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
    .try_collect()
    .await?;

  let cid = strands[0].cid().clone();
  let twine = resolver.resolve_strand(cid).await?.unpack();
  println!("specific strand resolved: {}", twine.cid());

  let tenth = resolver.resolve_index(&twine, 10).await?;
  println!("tenth: {}", tenth.cid());

  let latest = resolver.resolve_latest(&twine).await?;
  println!("latest: {}", latest.cid());

  // resolve 50
  resolver
    .resolve_range((&twine, 0..=50))
    .await?
    .inspect_ok(|twine| println!("index: {}, cid: {}", twine.index(), twine.cid()))
    .inspect_err(|err| eprintln!("error: {}", err))
    .filter_map(|twine| async { twine.ok() })
    .for_each(|_| async {})
    .await;

  // store.save(twine.clone()).await?;
  // println!("saved twine");
  // let twine_stream = resolver.resolve_range((&twine, ..)).await?
  //   .inspect_ok(|twine| println!("index: {}, cid: {}", twine.index(), twine.cid()))
  //   .inspect_err(|err| eprintln!("error: {}", err))
  //   .filter_map(|twine| async {
  //     twine.ok()
  //   });

  // pin!(twine_stream);

  // store.save_stream(twine_stream).await?;

  // try sequentially
  // for i in 0..100 {
  //   let twine = resolver.resolve_index(&twine, i).await?;
  //   println!("(cached?) index: {}, cid: {}", twine.index(), twine.cid());
  // }

  Ok(())
}
