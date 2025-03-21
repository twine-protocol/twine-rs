# twine_protocol

[![Crates.io Version](https://img.shields.io/crates/v/twine_protocol)](https://crates.io/crates/twine_protocol)
[![docs.rs (with version)](https://img.shields.io/docsrs/twine_protocol/latest)](https://docs.rs/twine_protocol/latest/twine_protocol/)

Official rust library for the [Twine Protocol](https://docs.twine.world).

The [twine_protocol](.) crate is a meta crate for ease of use of
[twine_lib](twine_lib) and optionally [twine_builder](twine_builder) and [twine_http_store](twine_http_store) through feature flags. Its main purpose
is to provide a prelude module to be used as: `use twine_protocol::prelude::*;`.

## Packages

[twine-rs](https://github.com/twine-protocol/twine-rs) monorepo provides
many crates for various purposes:

### Core libraries

- [twine_protocol](.) meta library for ease of use
- [twine_lib](twine_lib) core library for reading twine data
- [twine_builder](twine_builder) utilities for constructing twine data

### Stores

These provide various ways to store twine data, all implementing the
`Store` trait.

- [twine_http_store](twine_http_store) saving twine data to a twine http api
- [twine_sql_store](twine_sql_store) saving twine data to sqlite or mysql dbs
- [twine_sled_store](twine_sled_store) saving twine data to a Sled KV db
- [twine_car_store](twine_car_store) saving twine data to a CAR file
- [twine_pickledb_store](twine_pickledb_store) saving twine data to a simple binary data file

### CLI

The [twine_cli](twine_cli) is a work-in-progress commandline interface for
fetching, reading, and copying twine data.

## Quickstart

### Reading twine data

```rust
use twine_protocol::prelude::*;

const STRAND_JSON: &'static str = r#"{"cid":{"/":"bafyrmieej3j3sprtnbfziv6vhixzr3xxrcabnma43ajb5grhsixdvxzdvu"},"data":{"c":{"h":22,"v":"twine/2.0.0/time/1.0.0","k":{"a":"ED25519","k":{"/":{"bytes":"q0Th03lW3omSuQQSMKZZewQgmCalQLmAo3DN3M4PizM"}}},"r":32,"d":{},"g":"2024-12-20T00:00:00Z","e":null},"s":{"/":{"bytes":"hN5hlT+3+zwJzgmrej8LvtPrAnRsf0c2Qo8xZE0Bj0uY0Tudhi9CbBx/5AjPmceyYGifWb0uw5SZRLMDS15YBA"}}}}"#;

fn main(){
  let strand = Strand::from_tagged_dag_json(STRAND_JSON).unwrap();
  println!("Strand cid: {}", strand.cid());
  if let Some(subspec) = strand.subspec() {
    println!("has subspec {}", subspec);
  }
}
```

### Writing twine data

```rust,no_run
use twine_protocol::prelude::*;
use twine_lib::{ipld_core::ipld, multihash_codetable::Code};
use twine_builder::{TwineBuilder, RingSigner};

fn main() {
  // generate a signer from a newly generated key
  let signer = RingSigner::generate_ed25519().unwrap();
  println!("Private key (PEM):\n{}", signer.private_key_pem().unwrap());

  let builder = TwineBuilder::new(signer);
  let strand = builder
    .build_strand()
    .hasher(Code::Sha3_256) // use sha3 256
    .details(ipld!({
      "foo": "bar",
    }))
    .done()
    .unwrap();

  println!("strand: {}", strand);

  let mut prev = builder
    .build_first(strand.clone())
    .payload(ipld!({
      "baz": null,
    }))
    .done()
    .unwrap();

  println!("first tixel cid: {}", prev.cid());

  // build up to index 9
  for i in 1..10 {
    prev = builder
      .build_next(&prev)
      .payload(ipld!({
        "baz": "qux",
        "index": i,
      }))
      .done()
      .unwrap();

    println!("tixel (index: {}) cid: {}", prev.index(), prev.cid());
  }
}
```

### Retrieving data from a store (an http store)

```rust,no_run
use twine_protocol::prelude::*;
use twine_http_store::{v1, reqwest};
use futures::{StreamExt, TryStreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let cfg = v1::HttpStoreOptions::default().url("https://random.colorado.edu/api");
  let resolver = v1::HttpStore::new(
    reqwest::Client::builder()
      .timeout(std::time::Duration::from_secs(10))
      .build()?,
    cfg,
  );

  let strands: Vec<_> = resolver.strands().await?.try_collect().await?;
  for strand in &strands {
    println!("{}", strand.cid());
  }

  let latest = resolver.resolve_latest(&strands[0]).await?;
  println!("latest: {}", latest.cid());

  let tenth = resolver.resolve_index(&strands[0], 10).await?;
  println!("tenth: {}", tenth.cid());

  Ok(())
}
```

## Feature flags

- `sha3`(default): enables the sha3 family of hash functions
- `blake3`(default): enables the blake3 family of hash functions
- `http`: enables functionality of the twine_http_store
- `build`: enables functionality for constructing twine data
- `ripemd`: enables the ripemd hash functions
- `blake2s`: enables the blake2s hash functions
- `blake2b`: enables the blake2b hash functions
- `rsa`: enables RSA functionality with the `build` feature

## License

The rust twine library is distributed under the MIT license.

[LICENSE-MIT](LICENSE-MIT)
