use twine_builder::{TwineBuilder, RingSigner};
use twine_core::{ipld_core::ipld, multihash_codetable::Code};

fn main() {
  let signer = RingSigner::generate_ed25519().unwrap();
  println!("Private key (PEM):\n{}", signer.private_key_pem().unwrap());

  let builder = TwineBuilder::new(signer);
  let strand = builder.build_strand()
    .hasher(Code::Sha3_256)
    .details(ipld!({
      "foo": "bar",
    }))
    .done()
    .unwrap();

  println!("strand: {}", strand);

  let mut prev = builder.build_first(strand.clone())
    .payload(ipld!({
      "baz": null,
    }))
    .done()
    .unwrap();

  let n = 1000;
  let start_time = std::time::Instant::now();
  for i in 1..n {
    prev = builder.build_next(&prev)
      .payload(ipld!({
        "baz": "qux",
        "index": i,
      }))
      .done()
      .unwrap();
  }

  let elapsed = start_time.elapsed();
  let twines_per_second = n as f64 / elapsed.as_secs_f64();
  println!(
    "Built {} twines in {:.2} seconds ({:.2} twines per second, {:.2} microsec per twine)",
    n, elapsed.as_secs_f64(), twines_per_second, elapsed.as_micros() as f64 / n as f64
  );
}
