use rsa::pkcs1::EncodeRsaPrivateKey;
use twine_builder::TwineBuilder;
use twine_core::ipld_core::ipld;

fn main() {
  let mut rng = rand::thread_rng();
  let rsa = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate a key");
  let keypair = ring::signature::RsaKeyPair::from_der(rsa.to_pkcs1_der().unwrap().as_bytes()).unwrap();
  let signer = twine_builder::BiscuitSigner::new(
    biscuit::jws::Secret::RsaKeyPair(keypair.into()),
    "RS256".to_string(),
  );
  let builder = TwineBuilder::new(signer);
  let strand = builder.build_strand()
    .details(ipld!({
      "foo": "bar",
    }))
    .done()
    .unwrap();

  let mut prev = builder.build_first(strand.clone())
    .payload(ipld!({
      "baz": null,
    }))
    .done()
    .unwrap();

  let n = 1000;
  let start_time = std::time::Instant::now();
  for i in 1..n {
    prev = builder.build_next(prev)
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
