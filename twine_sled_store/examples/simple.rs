use futures::TryStreamExt;
use twine_core::twine::Twine;
use twine_sled_store::*;
use twine_core::resolver::*;
use twine_core::store::Store;
use twine_builder::{TwineBuilder, Jwk};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let tmp_dir = tempfile::tempdir()?;
  let db = sled::Config::new().temporary(true).path(tmp_dir.path()).open()?;
  let store = SledStore::new(db, SledStoreOptions::default());

  let key = Jwk::generate_rsa_key(2048).unwrap();
  let builder = TwineBuilder::new(key);
  let strand = builder.build_strand()
    .radix(2)
    .done()?;

  let first = builder.build_first(strand.clone()).done()?;
  let next = builder.build_next(first.clone()).done()?;

  println!("first: {}", first);
  println!("next: {}", next);

  store.save(strand.clone()).await?;
  store.save(first.clone()).await?;
  store.save(next.clone()).await?;

  let latest = store.resolve(strand.clone()).await?;
  assert_eq!(latest, next.clone());

  let next_10: Vec<Twine> = (latest.index()..10).into_iter().scan(latest, |prev, _| {
    let next = builder.build_next(prev.clone()).done().unwrap();
    *prev = next.clone();
    Some(next)
  }).collect();

  println!("next_10");

  store.save_many(next_10.clone()).await?;
  println!("saved next_10");

  store.resolve_range((strand.clone(), 0..10)).await?
    .inspect_ok(|twine| {
      println!("Resolved twine: {}", twine);
    })
    .inspect_err(|err| {
      println!("Error: {:?}", err);
    })
    .try_collect::<Vec<_>>().await?;

  Ok(())
}
