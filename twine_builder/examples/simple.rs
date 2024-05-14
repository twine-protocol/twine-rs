use twine_builder::TwineBuilder;
use twine_core::{libipld::ipld, twine::{Tixel, Twine}};
use josekit::jwk;

fn main() {
  let signer = jwk::Jwk::generate_ed_key(jwk::alg::ed::EdCurve::Ed25519).unwrap();
  let builder = TwineBuilder::new(signer);
  let strand = builder.build_strand()
    .version("1.0.0".to_string())
    .details(ipld!({
      "foo": "bar",
    }))
    .done()
    .unwrap();

  let mut signatures = vec![];
  let mut prev = builder.build_first(strand.clone())
    .payload(ipld!({
      "baz": null,
    }))
    .done()
    .unwrap();

  signatures.push(prev.signature().to_string());

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

    signatures.push(prev.signature().to_string());
  }

  let elapsed = start_time.elapsed();
  let twines_per_second = n as f64 / elapsed.as_secs_f64();
  println!(
    "Built {} twines in {:.2} seconds ({:.2} twines per second, {:.2} microsec per twine)",
    n, elapsed.as_secs_f64(), twines_per_second, elapsed.as_micros() as f64 / n as f64
  );
}
