use twine::prelude::*;
use twine_builder::{TwineBuilder, RingSigner};
use twine::twine_core::multihash_codetable::Code;
use twine_builder::pkcs8::{DecodePrivateKey, SecretDocument};
use twine_core::ipld_core::ipld;
use twine_core::multihash_codetable::Multihash;
use twine_core::verify::{Verifiable, Verified};
use twine_core::{serde_ipld_dagjson, Bytes};

#[allow(dead_code)]
fn to_dag_json_string(item: impl serde::Serialize) -> String {
  let bytes = serde_ipld_dagjson::to_vec(&item).unwrap();
  String::from_utf8(bytes).unwrap()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RandomnessPayloadRaw {
  salt: Bytes,
  pre: Multihash,
  timestamp: u64,
}

impl Verifiable for RandomnessPayloadRaw {

  fn verify(&self) -> Result<(), VerificationError> {
    if self.salt.len() != self.pre.size() as usize {
      return Err(VerificationError::Payload("Salt length does not match pre hash size".to_string()));
    }
    Ok(())
  }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RandomnessPayload(Verified<RandomnessPayloadRaw>);

impl RandomnessPayload {
  fn try_new(salt: Bytes, pre: Multihash, timestamp: u64) -> Result<Self, VerificationError> {
    Verified::try_new(RandomnessPayloadRaw { salt, pre, timestamp }).map(Self)
  }

  fn try_new_now(salt: Bytes, pre: Multihash) -> Result<Self, VerificationError> {
    Self::try_new(
      salt,
      pre,
      std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    )
  }

  fn from_rand(rand: Vec<u8>, pre: Multihash, prev: Tixel) -> Result<Self, VerificationError> {
    if prev.cid().hash().size() != pre.size() {
      return Err(VerificationError::Payload("Pre hash size does not match previous tixel hash size".to_string()));
    }
    // we xor the random bytes with previous cid hash digest
    let salt = Bytes(
      rand.iter()
        .zip(prev.cid().hash().digest().iter())
        .map(|(a, b)| a ^ b).collect()
    );
    Self::try_new_now(salt, pre)
  }

  fn new_start(pre: Multihash) -> Result<Self, VerificationError> {
    let num_bytes = pre.size();
    let salt = Bytes((0..num_bytes).collect());
    Self::try_new_now(salt, pre)
  }

  fn validate_randomness(&self, prev: Tixel) -> Result<(), VerificationError> {
    if prev.cid().hash().size() != self.0.pre.size() {
      return Err(VerificationError::Payload("Pre hash size does not match previous tixel hash size".to_string()));
    }
    let prev_payload = prev.extract_payload::<RandomnessPayload>()?;
    if self.0.timestamp < prev_payload.0.timestamp {
      return Err(VerificationError::Payload("Timestamp is less than previous tixel timestamp".to_string()));
    }
    // check that the precommitment from the previous tixel matches the xor rand value
    let rand = self.0.salt.iter()
      .zip(prev.cid().hash().digest().iter())
      .map(|(a, b)| a ^ b).collect::<Vec<u8>>();

    use twine_core::multihash_codetable::MultihashDigest;
    let code = Code::try_from(prev_payload.0.pre.code()).map_err(|_| VerificationError::UnsupportedHashAlgorithm)?;
    let pre = code.digest(&rand);
    if pre != self.0.pre {
      return Err(VerificationError::Payload("Pre hash does not match previous tixel pre hash".to_string()));
    }
    Ok(())
  }

  fn extract_randomness(current: Tixel, prev: Tixel) -> Result<Vec<u8>, VerificationError> {
    let payload = current.extract_payload::<RandomnessPayload>()?;
    if let Err(e) = payload.validate_randomness(prev) {
      return Err(e);
    }
    Ok(
      current.cid().hash().digest().to_vec()
    )
  }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let private_key_json = r#"
  {
    "/":{
      "bytes":"MFECAQEwBQYDK2VwBCIEIK2Ebms4gzrSxe+OxpxvGIxTdpOZFQYQvlipxYxR6sXqgSEAq0Th03lW3omSuQQSMKZZewQgmCalQLmAo3DN3M4PizM"
    }
  }
  "#;
  // let signer = RingSigner::generate_ed25519().unwrap();
  // let private_key_bytes = Bytes(signer.pkcs8().as_bytes().to_vec());
  // println!("Keys (dag_json): {}", to_dag_json_string(private_key_bytes));
  let private_key : Bytes = serde_ipld_dagjson::from_slice(private_key_json.as_bytes()).unwrap();
  let signer = RingSigner::new(
    twine_core::crypto::SignatureAlgorithm::Ed25519,
    SecretDocument::from_pkcs8_der(&private_key.0)?
  ).unwrap();

  let builder = TwineBuilder::new(signer);
  let strand = builder.build_strand()
    .hasher(Code::Sha3_256)
    .subspec("nist-rng/1.0.0".to_string())
    .details(ipld!({
      "foo": "bar",
    }))
    .done()
    .unwrap();

  println!("Strand: {}", strand);

  use rand::Rng;
  let mut rng = rand::thread_rng();
  let mut next_rand = rng.gen::<[u8; 32]>();

  use twine_core::multihash_codetable::MultihashDigest;
  let pre = Code::Sha3_256.digest(&next_rand);
  let payload = RandomnessPayload::new_start(pre)?;

  let mut prev = builder.build_first(strand.clone())
    .payload(payload)
    .done()
    .unwrap();

  let n = 10;
  let start_time = std::time::Instant::now();

  for _ in 1..n {
    let rand = next_rand;
    let pre = Code::Sha3_256.digest(&rand);
    next_rand = rng.gen::<[u8; 32]>();
    let payload = RandomnessPayload::from_rand(rand.to_vec(), pre, prev.tixel())?;
    let next = builder.build_next(&prev)
      .payload(payload)
      .done()
      .unwrap();
    println!("Twine: {}", next);
    let rand_hex = RandomnessPayload::extract_randomness(next.tixel(), prev.tixel())?;
    println!("Randomness: {}", rand_hex.iter().map(|b| format!("{:02x}", b)).collect::<String>());
    prev = next;
  }

  let elapsed = start_time.elapsed();
  let twines_per_second = n as f64 / elapsed.as_secs_f64();
  println!(
    "Built {} twines in {:.2} seconds ({:.2} twines per second, {:.2} microsec per twine)",
    n, elapsed.as_secs_f64(), twines_per_second, elapsed.as_micros() as f64 / n as f64
  );

  Ok(())
}
