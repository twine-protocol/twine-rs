
pub struct Signer {
    // ...
}

#[derive(Debug, Clone)]
pub struct SignerError;

impl Signer {
    fn from_keys(public_key: &[u8], private_key: &[u8]) -> Result<Signer, SignerError> {}
    fn from_random() -> Result<Signer, SignerError> {}
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignerError> {}
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool, SignerError> {}
    fn public_key(&self) -> &[u8] {}
    fn private_key(&self) -> &[u8] {}
}