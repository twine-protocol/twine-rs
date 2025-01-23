use twine_builder::{TwineBuilder, RingSigner};
use twine_core::{ipld_core::ipld, multihash_codetable::Code, resolver::Resolver, store::Store};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

  let store = twine_sql_store::SqlStore::open("sqlite:file:foo?mode=memory&cache=shared").await?;
  store.create_tables().await?;

  let signer = RingSigner::generate_ed25519().unwrap();
  let builder = TwineBuilder::new(signer);
  let strand = builder.build_strand()
    .hasher(Code::Sha3_256)
    .details(ipld!({
      "foo": "bar",
    }))
    .done()?;

  {
    store.save(strand.clone()).await?;

    let mut prev = builder.build_first(strand.clone())
      .payload(ipld!({
        "baz": null,
      }))
      .done()?;

    store.save(prev.clone()).await?;

    let n = 1000;
    for i in 1..n {
      prev = builder.build_next(&prev)
        .payload(ipld!({
          "baz": "qux",
          "index": i,
        }))
        .done()?;

      store.save(prev.clone()).await?;
    }
  }

  use futures::TryStreamExt;
  let strands: Vec<_> = store.strands().await?.try_collect().await?;
  println!("{} strands", strands.len());
  let strand2 = store.resolve_strand(&strand.cid()).await?;
  let latest = store.resolve_latest(&strand.cid()).await?;
  let latest_index = latest.index();
  println!("strand: {}", strand2.unpack());
  println!("latest index: {}", latest_index);

  println!("deleting latest");
  store.delete(latest.cid()).await?;
  let latest = store.resolve_latest(&strand.cid()).await?;
  println!("latest index: {}", latest.unpack().index());
  println!("deleting strand");
  store.delete(strand.cid()).await?;

  match store.resolve_latest(strand.cid()).await {
    Ok(_) => println!("unexpectedly found strand"),
    Err(e) => println!("expected error: {}", e),
  }

  Ok(())
}
