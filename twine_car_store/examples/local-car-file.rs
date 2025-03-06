use futures::TryStreamExt;
use tempfile::NamedTempFile;
use twine_builder::{RingSigner, TwineBuilder};
use twine_car_store::CarStore;
use twine_core::{ipld_core::ipld, multihash_codetable::Code, resolver::Resolver, store::Store};

#[tokio::main]
async fn main() {
  let f = NamedTempFile::new().unwrap();
  let filename = f.path().to_str().unwrap();
  println!("filename: {}", filename);

  let signer = RingSigner::generate_ed25519().unwrap();
  let builder = TwineBuilder::new(signer);
  let strand = builder
    .build_strand()
    .hasher(Code::Sha3_256)
    .details(ipld!({
      "foo": "bar",
    }))
    .done()
    .unwrap();

  {
    let store = CarStore::new(filename).unwrap();
    store.save(strand.clone()).await.unwrap();

    let mut prev = builder
      .build_first(strand.clone())
      .payload(ipld!({
        "baz": null,
      }))
      .done()
      .unwrap();

    store.save(prev.clone()).await.unwrap();

    let n = 1000;
    for i in 1..n {
      prev = builder
        .build_next(&prev)
        .payload(ipld!({
          "baz": "qux",
          "index": i,
        }))
        .done()
        .unwrap();

      store.save(prev.clone()).await.unwrap();
    }

    println!("saving {} tixels", n);

    // saved on drop
    // store.flush().await.unwrap();
  }

  let store2 = CarStore::new(filename).unwrap();
  let strands: Vec<_> = store2.strands().await.unwrap().try_collect().await.unwrap();
  println!("strands: {:?}", strands);
  let strand2 = store2.resolve_strand(&strand.cid()).await.unwrap();
  let latest = store2.resolve_latest(&strand.cid()).await.unwrap();
  println!("strand: {}", strand2.unpack());
  println!("latest: {}", latest.unpack());
}
