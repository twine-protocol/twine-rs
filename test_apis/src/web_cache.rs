use std::collections::HashMap;
use std::sync::Mutex;

use rocket::State;
use rocket::response::Responder;
use rocket::response::content::{RawJson};
use rocket::fairing::AdHoc;
use rocket::{get, routes};
use thiserror::Error;
use twine_builder::{PulseBuilder, ChainBuilder};
use twine_core::josekit::jwk::alg::ed::EdCurve::Ed25519;
use twine_core::josekit::jws::alg::eddsa::EddsaJwsAlgorithm;
use twine_core::libipld::{Cid, ipld, multihash};
use twine_core::twine::{Pulse, Chain};
use crate::helpers::ParamCid;
use crate::map;
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
    AdHoc::on_ignite("Chain/Pulse", |rocket| async {
        let keys = EddsaJwsAlgorithm::Eddsa.generate_key_pair(Ed25519).expect("Can make keys");
        let signer = EddsaJwsAlgorithm::Eddsa.signer_from_jwk(&keys.to_jwk_private_key()).expect("Can make signer");
        let verifier = EddsaJwsAlgorithm::Eddsa.verifier_from_jwk(&keys.to_jwk_public_key()).expect("Can make verifier");
        let hasher = multihash::Code::Sha3_512;

        let chain = ChainBuilder::new(
            "test".into(),
            HashMap::new(),
            keys.to_jwk_public_key()
        )
        .finalize( &signer, &verifier, hasher)
        .expect("Should be able to make chains");
        
        let pulse = PulseBuilder::first(&chain)
            .payload(map!{ String::from("Hello") => ipld!{ "world" } })
            .finalize(&signer, &verifier)
            .expect("Should be able to make pulses");
        
        println!("chain {:?} : pulse {:?}", chain.cid, pulse.cid);

        let state = PulseCache::new(map!{ chain.cid => map!{ pulse.cid => pulse } });
        rocket.mount("/", routes![index])
            .manage(state)
    })
}

