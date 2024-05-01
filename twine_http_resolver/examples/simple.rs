use twine_http_resolver::*;
use twine_core::prelude::*;

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

  let latest = resolver.resolve_latest(twine).await?;
  println!("latest: {}", latest);

  Ok(())
}
