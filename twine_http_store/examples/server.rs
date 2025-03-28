//! This example shows how to setup a simple v2 http server
use twine_builder::{RingSigner, TwineBuilder};
use twine_http_store::server;
use twine_lib::ipld_core::ipld;
use twine_lib::{resolver::*, Cid};
use twine_lib::store::{MemoryStore, Store};

async fn make_strand_data<S: Store + Resolver>(
    store: &S,
  ) -> Result<Cid, Box<dyn std::error::Error>> {
  let signer = RingSigner::generate_ed25519().unwrap();
  let builder = TwineBuilder::new(signer);
  let strand = builder.build_strand().done()?;
  store.save(strand.clone()).await?;

  let mut prev = builder.build_first(strand.clone())
    .payload(ipld!({
      "i": 0
    }))
    .done()?;
  store.save(prev.clone()).await?;

  for i in 1..10 {
    let tixel = builder
      .build_next(&prev)
      .payload(ipld!({
        "i": i
      }))
      .done()?;
    store.save(tixel.clone()).await?;
    prev = tixel;
  }

  Ok(strand.cid())
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
  let store = MemoryStore::default();
  let strand_cid = make_strand_data(&store).await.unwrap();
  println!("created strand: {}", strand_cid);

  let app = server::api(store, server::ApiOptions::default());

  // run our app with hyper, listening globally on port 3000
  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  println!("listening on {}", listener.local_addr().unwrap());
  axum::serve(listener, app).await
}
