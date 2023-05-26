use std::str::FromStr;

use rocket::request::FromParam;
use twine_core::libipld::{Cid, cid::Error};
use thiserror::Error;

pub struct ParamCid(pub Cid);  // Wrapped CID

#[derive(Debug, Error)]
#[error("can't decode into a string")]
pub struct CIDDecodeError(#[from] Error);

impl FromParam<'_> for ParamCid {
    type Error = CIDDecodeError;

    fn from_param(param: &'_ str) -> Result<Self, Self::Error> {
        Ok(ParamCid(Cid::from_str(param)?))
    }
}