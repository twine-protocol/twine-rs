use std::str::FromStr;

use rocket::request::FromParam;
use twine_core::{Cid, ipld_core::cid::Error};
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

#[macro_export]
macro_rules! map {
    // map-like
    ($($k:expr => $v:expr),* $(,)?) => {{
        core::convert::From::from([$(($k, $v),)*])
    }};
}
