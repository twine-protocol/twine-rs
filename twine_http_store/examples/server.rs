//! This example shows how to setup a simple v2 http server
use axum::{
  http::StatusCode, extract::Request, middleware::Next, response::Response
};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use twine_builder::{RingSigner, TwineBuilder};
use twine_http_store::server;
use twine_lib::ipld_core::ipld;
use twine_lib::{resolver::*, Cid};
use twine_lib::store::{MemoryStore, Store};
use hyper::server::conn::http1;

async fn make_strand_data<S: Store + Resolver>(
    store: &S,
  ) -> Result<Cid, Box<dyn std::error::Error>> {
  let signer = RingSigner::generate_ed25519().unwrap();
  let builder = TwineBuilder::new(signer);
  let strand = builder.build_strand().done()?;
  store.save(strand.clone()).await?;

  let mut prev = builder.build_first(strand.clone())
    .payload(ipld!({
      "i": 0
    }))
    .done()?;
  store.save(prev.clone()).await?;

  for i in 1..100 {
    let tixel = builder
      .build_next(&prev)
      .payload(ipld!({
        "i": i
      }))
      .done()?;
    store.save(tixel.clone()).await?;
    prev = tixel;
  }

  Ok(strand.cid())
}

const VALID_API_KEY: &str = "ApiKey dev";

async fn api_key_middleware(
  req: Request,
  next: Next,
) -> Result<Response, StatusCode> {
  if let Some(api_key) = req.headers().get(axum::http::header::AUTHORIZATION) {
    if api_key == VALID_API_KEY {
      return Ok(next.run(req).await)
    }
  }

  Err(StatusCode::UNAUTHORIZED)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
  let store = MemoryStore::default();
  let strand_cid = make_strand_data(&store).await.unwrap();
  println!("created strand: {}", strand_cid);

  let options = server::ApiOptions {
    max_query_length: 1000,
    read_only: false,
    ..server::ApiOptions::default()
  };

  let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3000).into();
  let listener = TcpListener::bind(addr).await?;

  let service = server::api(store, options);

  loop {
      let (stream, _) = listener.accept().await?;
      let io = TokioIo::new(stream);
      let svc_clone = service.clone();
      tokio::task::spawn(async move {
          if let Err(err) = http1::Builder::new().serve_connection(io, svc_clone).await {
              println!("Failed to serve connection: {:?}", err);
          }
      });
  }
}
