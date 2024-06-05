use biscuit::jws::{Header, Secret};
use crate::signer::SigningError;

pub fn sign<P: AsRef<[u8]>>(jwk: &Secret, payload: P) -> Result<String, SigningError> {
  let jws = biscuit::jws::Compact::<_, ()>::new_decoded(Header::default(), payload.as_ref().to_vec());
  let signature = jws.encode(jwk)
    .map_err(|e| SigningError(format!("Failed to sign: {}", e)))?;
  Ok(signature.encoded().unwrap().encode())
}
