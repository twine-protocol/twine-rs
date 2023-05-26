use std::collections::HashMap;
use std::sync::{Mutex, PoisonError, MutexGuard};
use std::sync::atomic::{AtomicUsize, Ordering};

use rocket::State;
use rocket::response::Responder;
use rocket::response::content::{RawHtml, RawJson};
use rocket::fairing::AdHoc;
use rocket::{get, routes};
use thiserror::Error;
use twine_core::libipld::Cid;
use twine_core::twine::{Pulse, Chain};
use crate::helpers::ParamCid;
use twine_core::twine::Twine;

type DangerousChainCache = HashMap<Cid, Chain>;
type ChainCache = Mutex<DangerousChainCache>;
type DangerousPulseCache = HashMap<Cid, HashMap<Cid, Pulse>>;
type PulseCache = Mutex<DangerousPulseCache>; // blocking mutex

#[derive(Debug, Responder, Error)]
enum ResolutionError {
    #[response(status = 500, content_type = "plain")]
    #[error("Failed to lock mutex")]
    MutexLockFailure(String),
    #[response(status = 404, content_type = "plain")]
    #[error("Could not locate items from cache")]
    NotFound(String)
}

#[get("/<chain_cid>/<pulse_cid>")]
fn index(chain_cid: ParamCid, pulse_cid: ParamCid, cache: &State<PulseCache>) -> Result<RawJson<String>, ResolutionError> { // TODO: don't use RawJson; use Json
    let c = match cache.lock() {
        Err(_) => return Err(ResolutionError::MutexLockFailure(String::from("Could not read from cache"))),
        Ok(c) => c
    };

    c
    .get(&chain_cid.0)
    .and_then(|p| p.get(&pulse_cid.0))
    .and_then(|pulse| Some(
        Ok(RawJson(pulse.to_json().expect("Pulse already in cache cannot be serialized to JSON!")))
    ))
    .unwrap_or(Err(ResolutionError::NotFound(String::from("Could not located chain or pulse in the cache"))))
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Managed Hit Count", |rocket| async {
        rocket.mount("/", routes![index])
            .manage(PulseCache::new(HashMap::new()))
    })
}

