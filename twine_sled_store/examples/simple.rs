use futures::{StreamExt, TryStreamExt};
use twine_core::twine::Twine;
use twine_sled_store::*;
use twine_core::resolver::*;
use twine_core::store::Store;
use twine_builder::{TwineBuilder, BiscuitSigner, biscuit::jws::Secret};
use twine_builder::ring::{rand, signature::{EcdsaKeyPair, ECDSA_P256_SHA256_FIXED_SIGNING}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let tmp_dir = tempfile::tempdir()?;
  let db = sled::Config::new().temporary(true).path(tmp_dir.path()).open()?;
  let store = SledStore::new(db, SledStoreOptions::default());

  let rng = rand::SystemRandom::new();
  let alg = &ECDSA_P256_SHA256_FIXED_SIGNING;
  let pkcs = EcdsaKeyPair::generate_pkcs8(alg, &rng).unwrap();
  let key = EcdsaKeyPair::from_pkcs8(alg, pkcs.as_ref(), &rng).unwrap();
  let secret = Secret::EcdsaKeyPair(key.into());
  let signer = BiscuitSigner::new(secret, "ES256".to_string());
  let builder = TwineBuilder::new(signer);
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

  store.strands().await?
    .inspect_ok(|strand| {
      println!("strand: {}", strand.cid());
    })
    .inspect_err(|err| {
      println!("Error: {:?}", err);
    })
    .try_collect::<Vec<_>>().await?;

  let s = store.resolve_strand(&strand).await?;
  assert_eq!(*s, strand);

  let latest = store.resolve(strand.clone()).await?;
  assert_eq!(latest, next.clone());

  let count = 1000;
  let next_n: Vec<Twine> = (latest.index()..count).into_iter().scan(latest, |prev, _| {
    let next = builder.build_next(prev.clone()).done().unwrap();
    *prev = next.clone();
    Some(next)
  }).collect();

  println!("next_n");
  next_n.iter().for_each(|twine| {
    println!("index: {}", twine.index());
  });

  store.save_many(next_n.clone()).await?;
  println!("saved next_n");

  let start_time = std::time::Instant::now();
  store.resolve_range((strand.clone(), 0..=count as i64)).await?
    // .inspect_ok(|twine| {
    //   println!("Resolved twine: {}", twine.index());
    // })
    // .inspect_err(|err| {
    //   println!("Error: {:?}", err);
    // })
    .collect::<Vec<_>>().await;

  // check that they're in order
  // results.iter().rev().enumerate().for_each(|(i, twine)| {
  //   match twine {
  //     Ok(twine) => assert_eq!(twine.index(), i as u64),
  //     Err(_) => {},
  //   }
  // });


  println!("Resolved {} twines in {}ms", count, start_time.elapsed().as_millis());

  // try just using resolve_index
  // let start_time = std::time::Instant::now();
  // futures::stream::iter(0..=count as u64)
  //   .map(|i| store.resolve_index(strand.as_cid(), i))
  //   .buffered(100)
  //   .inspect_ok(|twine| {
  //     // println!("Resolved twine: {}", twine.index());
  //   })
  //   .inspect_err(|err| {
  //     // println!("Error: {:?}", err);
  //   })
  //   .collect::<Vec<_>>().await;

  // println!("Resolved {} twines in {}ms", count, start_time.elapsed().as_millis());

  Ok(())
}
