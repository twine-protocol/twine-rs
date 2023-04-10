#[derive(Debug, Clone)]
pub struct SignerError;

pub trait Signer {
    fn from_keys(public_key: &[u8], private_key: &[u8]) -> Result<Self, SignerError>;
    fn from_random() -> Result<Self, SignerError>;
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignerError>;
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool, SignerError>;
    fn public_key(&self) -> &[u8];
    fn private_key(&self) -> &[u8];
}

pub struct DefaultSigner {

}

impl Signer for DefaultSigner {
    fn from_keys(public_key: &[u8], private_key: &[u8]) -> Result<dyn Signer, SignerError> {}
    fn from_random() -> Result<Self, SignerError>{}
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignerError> {}
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool, SignerError> {}
    fn public_key(&self) -> &[u8] {}
    fn private_key(&self) -> &[u8] {}
}