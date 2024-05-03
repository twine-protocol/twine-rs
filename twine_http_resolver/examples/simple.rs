use futures::{StreamExt, TryStreamExt};
use twine_http_resolver::*;
use twine_core::prelude::*;
use futures_time::prelude::*;
use futures_time::time::Duration;
use futures_time::stream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cfg = HttpResolverOptions::default()
    .url("https://random.colorado.edu/api");
  let resolver = HttpResolver::new(reqwest::Client::new(), cfg);
  let cid = Cid::try_from("bafyriqa5k2d3t3r774geicueaed2wc2fosjwqeexfhwbptfgq7rcn5mwucnhfeuxu2nxbrch3rl6yqjlozhuswo5ln3xwjm35iftt3tpqlcgs").unwrap();
  let twine = resolver.resolve_strand(cid).await?;
  println!("strand: {}", twine);

  let tenth = resolver.resolve_index(&twine, 10).await?;
  println!("tenth: {}", tenth);

  let latest = resolver.resolve_latest(&twine).await?;
  println!("latest: {}", latest);

  resolver.resolve_range((&twine, 100..3)).await?
    .inspect_ok(|twine| println!("index: {}, cid: {}", twine.index(), twine.cid()))
    .inspect_err(|err| eprintln!("error: {}", err))
    .for_each(|_| async {}).await;

  Ok(())
}
