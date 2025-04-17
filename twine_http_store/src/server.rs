//! This module provides an v2 HTTP API backed by a Twine store.
use twine_lib::{store::Store, resolver::Resolver};

/// Options for the API
#[derive(Debug, Clone)]
pub struct ApiOptions {
  /// The maximum length of a query
  /// If the length of the query exceeds this value, a 400 error will be returned
  /// Default: 1000
  pub max_query_length: u64,

  /// If true (default), the API will not allow any write operations
  pub read_only: bool,
}

impl Default for ApiOptions {
  fn default() -> Self {
    Self {
      max_query_length: 1000,
      read_only: true,
    }
  }
}

pub use api::ApiService;

/// Create a hyper service for the Twine API
pub fn api<S> (
  store: S,
  options: ApiOptions,
) -> api::ApiService<S> where S: Store + Resolver + 'static  {
  api::ApiService::new(store, options)
}

mod api {
  use super::models::{Car, Json};
  use super::*;
  use http_body_util::combinators::BoxBody;
  use http_body_util::{BodyExt, Full};
  use hyper::body::Bytes;
  use hyper::service::Service;
  use hyper::{HeaderMap, Method, Request, Response, StatusCode};
  use http_body::Body;
  use twine_lib::store::Store;
  use twine_lib::resolver::Resolver;
  use twine_lib::Cid;

  use std::convert::Infallible;
  use std::future::Future;
  use std::pin::Pin;
  use std::sync::Arc;

  use twine_lib::errors::{ConversionError, ResolutionError, StoreError, VerificationError};

  const MAX_BODY_SIZE: u64 = 1024 * 1024; // 1MB

  fn mk_response<C: Into<Bytes>>(content: C, status_code: StatusCode) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
      .status(status_code)
      .header("X-Spool-Version", "2")
      .body(BoxBody::new(Full::new(content.into())))
      .unwrap()
  }

  #[allow(unused)]
  #[derive(Debug, thiserror::Error)]
  pub enum ApiError {
    #[error("Malformed cid: {0}")]
    MalformedCid(#[from] twine_lib::cid::Error),
    #[error("Bad Data: {0}")]
    BadRequestData(String),
    #[error("Server error: {0}")]
    ServerError(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Verification error: {0}")]
    VerificationError(#[from] VerificationError),
    #[error("Resolution error: {0}")]
    ResolutionError(#[from] ResolutionError),
    #[error("Store Error: {0}")]
    StoreError(#[from] StoreError),
    #[error("Not found")]
    NotFound,
    #[error("No content")]
    NoContent,
    #[error("Payload too large")]
    PayloadTooLarge,
    // #[error("Unauthorized")]
    // Unauthorized,
  }

  impl From<ConversionError> for ApiError {
    fn from(e: ConversionError) -> Self {
      ApiError::BadRequestData(e.to_string())
    }
  }

  impl ApiError {
    fn as_response(self) -> Response<BoxBody<Bytes, Infallible>> {
      match self {
        ApiError::ServerError(e) => mk_response(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
        ApiError::VerificationError(e) => mk_response(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
        ApiError::NotFound => mk_response("Not found", StatusCode::NOT_FOUND),
        ApiError::MalformedCid(e) => mk_response(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
        ApiError::BadRequestData(e) => mk_response(e.to_string(), StatusCode::BAD_REQUEST),
        // ApiError::Unauthorized mk_response=AUTHORIZED, "Un, > (StatusCode::authorized"),
        ApiError::ResolutionError(e) => match e {
          ResolutionError::NotFound => mk_response("Not found", StatusCode::NOT_FOUND),
          _ => mk_response(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
        },
        ApiError::StoreError(e) => match e {
          StoreError::Fetching(e) => match e {
            ResolutionError::NotFound => mk_response("Not found", StatusCode::NOT_FOUND),
            _ => mk_response(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
          },
          _ => mk_response(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR),
        },
        ApiError::NoContent => mk_response("", StatusCode::NO_CONTENT),
        ApiError::PayloadTooLarge => mk_response("Payload too large", StatusCode::PAYLOAD_TOO_LARGE),
      }
    }
  }

  fn wants_car(headers: &HeaderMap) -> bool {
    headers.get("accept").map_or(false, |h| {
      h.to_str()
        .map_or(false, |s|
          s.contains("application/octet-stream") ||
          s.contains("application/vnd.ipld.car")
        )
    })
  }

  /// A hyper service for the Twine API
  #[derive(Debug, Clone)]
  pub struct ApiService<S> where S: Store + Resolver {
    store: Arc<S>,
    options: ApiOptions,
  }


  impl<S> ApiService<S> where S: Store + Resolver {
    /// Create a new instance of API service
    pub fn new(store: S, options: ApiOptions) -> Self {
      Self {
        store: Arc::new(store),
        options,
      }
    }
  }

  impl<S, B: Body + Send + 'static> Service<Request<B>> for ApiService<S> where S: Store + Resolver + 'static, <B as http_body::Body>::Error: Send, <B as http_body::Body>::Data: Send {
    type Response = Response<BoxBody<Bytes, Infallible>>;
    type Error = Infallible;
    #[cfg(target_arch = "wasm32")]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    #[cfg(not(target_arch = "wasm32"))]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<B>) -> Self::Future {
      let as_car = wants_car(&req.headers());
      let route: (Method, String) = (req.method().clone(), req.uri().path().to_string());
      let store = self.store.clone();
      let full = req.uri().query().unwrap_or_default()
        .split('&')
        .any(|q| q.starts_with("full") && q != "full=false");
      let options = self.options.clone();

      let map_result = move |res| {
        if as_car {
          mk_response(Car(res), StatusCode::OK)
        } else {
          mk_response(Json(res), StatusCode::OK)
        }
      };

      let get_body_bytes = |req: Request<B>| {
        let max = req.body().size_hint().upper().unwrap_or(u64::MAX);
        async move {
          if max > MAX_BODY_SIZE as u64 {
            return Err(ApiError::PayloadTooLarge);
          }
          let body = req
            .collect()
            .await
            .map_err(|_| ApiError::BadRequestData("Failed to read body".to_string()))?;
          Ok(body.to_bytes())
        }
      };

      Box::pin(async move {
        let res = match (route.0, route.1.as_str()) {
          (Method::HEAD, "/") => Ok(mk_response("", StatusCode::OK)),
          (Method::HEAD, path) => {
            let q = path.trim_start_matches('/');
            let res = handlers::has(store, q.to_string()).await;
            match res {
              Ok(true) => Ok(mk_response("", StatusCode::OK)),
              Ok(false) => Err(ApiError::NotFound),
              Err(e) => Err(e),
            }
          },
          (Method::GET, "/") => handlers::list_strands(store).await.map(map_result),
          (Method::GET, path) => {
            let q = path.trim_start_matches('/');
            handlers::query(store, q.to_string(), full, options).await.map(map_result)
          },
          (Method::PUT, "/") => {
            if options.read_only {
              return Ok(mk_response("This API is read-only", StatusCode::FORBIDDEN));
            }
            match get_body_bytes(req).await {
              Ok(body) => {
                handlers::save_strands(store, body).await
                  .map(|_| mk_response("", StatusCode::CREATED))
              },
              Err(e) => Err(e),
            }
          },
          (Method::PUT, path) => {
            if options.read_only {
              return Ok(mk_response("This API is read-only", StatusCode::FORBIDDEN));
            }
            let strand_cid = path.trim_start_matches('/').parse::<Cid>();
            match strand_cid {
              Ok(cid) => {
                match get_body_bytes(req).await {
                  Ok(body) => {
                    handlers::save_tixels(store, cid, body).await
                      .map(|_| mk_response("", StatusCode::CREATED))
                  },
                  Err(e) => Err(e),
                }
              },
              Err(_) => Err(ApiError::BadRequestData("Invalid strand cid".into())),
            }
          },
          _ => Err(ApiError::NotFound),
        };

        let res = match res {
          Ok(res) => res,
          Err(e) => e.as_response(),
        };
        Ok(res)
      })
    }
  }
}

mod handlers {
  use super::models::AnyResult;
  use super::ApiOptions;
  use hyper::body::Bytes;
  use std::sync::Arc;
  use twine_lib::Cid;
  use twine_lib::{resolver::AnyQuery, store::Store};
  use twine_lib::resolver::Resolver;
  use futures::TryStreamExt;
  use super::api::ApiError;

  pub async fn list_strands<S: Store + Resolver + 'static>(store: Arc<S>) -> Result<AnyResult, ApiError> {
    let strands = store.strands().await?.map_ok(|s| s.into()).try_collect().await?;
    Ok(AnyResult::Strands {
      items: strands,
    })
  }

  pub async fn has(store: Arc<impl Store + Resolver>, q: String) -> Result<bool, ApiError> {
    let query = match q.parse::<AnyQuery>() {
      Ok(query) => query,
      Err(_) => return Err(ApiError::BadRequestData("Invalid query".to_string())),
    };
    let res = match query {
      AnyQuery::Strand(strand) => store.has_strand(&strand).await?,
      AnyQuery::One(single) => store.has(single).await?,
      AnyQuery::Many(_) => return Err(ApiError::BadRequestData("Range queries are not supported for HEAD".to_string())),
    };
    Ok(res)
  }

  pub async fn query<S: Store + Resolver + 'static>(store: Arc<S>, q: String, full: bool, options: ApiOptions) -> Result<AnyResult, ApiError> {
    let result = match q.parse::<AnyQuery>() {
      Ok(query) => match query {
        AnyQuery::Strand(strand_cid) => {
          let strand = store.resolve_strand(&strand_cid).await?;
          AnyResult::Strands {
            items: vec![strand.unpack().clone().into()],
          }
        }
        AnyQuery::One(query) => {
          let twine = store.resolve(query).await?;
          let strand = if full {
            Some(twine.strand().clone().into())
          } else {
            None
          };
          AnyResult::Tixels {
            items: vec![(*twine.unpack()).clone().into()],
            strand,
          }
        }
        AnyQuery::Many(range) => {
          use futures::TryStreamExt;
          let absolute = range.try_to_absolute(store.as_ref()).await?;
          if absolute.is_none() {
            return Err(ApiError::NoContent);
          }
          let range = absolute.unwrap();
          if range.len() > options.max_query_length {
            return Err(ApiError::BadRequestData(format!("Query length exceeds max length of {}", options.max_query_length)));
          }
          let tixels: Vec<_> = store.resolve_range(range).await?.try_collect().await?;
          let strand = if full && tixels.len() > 0 {
            Some((*tixels[0].strand()).clone().into())
          } else {
            None
          };
          AnyResult::Tixels {
            items: tixels.into_iter().map(|t| (*t).clone().into()).collect(),
            strand,
          }
        }
      },
      Err(_) => return Err(ApiError::BadRequestData("Invalid query".to_string())),
    };

    Ok(result)
  }

  pub async fn save_strands<S: Store + Resolver + 'static>(store: Arc<S>, bytes: Bytes) -> Result<(), ApiError> {
    let strands = twine_lib::car::from_car_bytes(&mut std::io::Cursor::new(bytes))
      .map_err(|e| ApiError::BadRequestData(e.to_string()))?;

    if !strands.iter().all(|s| s.is_strand()) {
      return Err(ApiError::BadRequestData("Not all items are strands".to_string()));
    }

    store.save_many(strands).await?;
    Ok(())
  }

  pub async fn save_tixels<S: Store + Resolver + 'static>(store: Arc<S>, strand_cid: Cid, bytes: Bytes) -> Result<(), ApiError> {
    let things = twine_lib::car::from_car_bytes(&mut std::io::Cursor::new(bytes))
      .map_err(|e| ApiError::BadRequestData(e.to_string()))?;

    let mut tixels: Vec<_> = things.into_iter()
      .map(|t| {
        if !t.is_tixel() {
          return Err(ApiError::BadRequestData("Not all items are tixels".to_string()));
        }
        if t.strand_cid() != strand_cid {
          return Err(ApiError::BadRequestData("Not all items are from the same strand".to_string()));
        }
        Ok(t.unwrap_tixel())
      })
      .collect::<Result<_, _>>()?;

    tixels.sort_by_key(|t| t.index());
    store.save_many(tixels).await?;
    Ok(())
  }
}

mod models {
  use serde::{Deserialize, Serialize};
  use twine_lib::serde::dag_json;
  use twine_lib::twine::AnyTwine;
  use twine_lib::{car::to_car_bytes, twine::{Strand, Tagged, Tixel}, Cid};

  #[derive(Debug, Serialize, Deserialize)]
  #[serde(untagged)]
  pub enum AnyResult {
    Tixels {
      #[serde(with = "dag_json")]
      items: Vec<Tagged<Tixel>>,
      #[serde(with = "dag_json")]
      #[serde(skip_serializing_if = "Option::is_none")]
      strand: Option<Tagged<Strand>>,
    },
    Strands {
      #[serde(with = "dag_json")]
      items: Vec<Tagged<Strand>>,
    },
  }

  pub struct Json(pub AnyResult);

  impl From<Json> for hyper::body::Bytes {
    fn from(json: Json) -> Self {
      let json_bytes = serde_json::to_vec(&json.0).unwrap();
      json_bytes.into()
    }
  }

  pub struct Car(pub AnyResult);

  impl From<Car> for hyper::body::Bytes {
    fn from(car: Car) -> Self {
      let items = match car.0 {
        AnyResult::Tixels { items, strand } => items
          .into_iter()
          .map(|t| AnyTwine::from(t.unpack()))
          .chain(strand.into_iter().map(|s| AnyTwine::from(s.unpack())))
          .collect::<Vec<_>>(),
        AnyResult::Strands { items } => items
          .into_iter()
          .map(|s| AnyTwine::from(s.unpack()))
          .collect::<Vec<_>>(),
      };
      let car_bytes = to_car_bytes(items, vec![Cid::default()]);
      car_bytes.into()
    }
  }
}

#[cfg(test)]
mod test {
  use std::convert::Infallible;

  use crate::v2;
  use super::*;
  use http_body_util::combinators::BoxBody;
  use hyper::{service::Service, StatusCode};
  use twine_builder::{RingSigner, TwineBuilder};
  use twine_lib::{ipld_core::ipld, store::MemoryStore, Cid, twine::AnyTwine};

  async fn make_strand<S: Store + Resolver>(
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

    for i in 1..10 {
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

  async fn parse_response(
    response: hyper::Response<BoxBody<hyper::body::Bytes, Infallible>>,
  ) -> Result<Vec<AnyTwine>, Box<dyn std::error::Error>> {
    use futures::TryStreamExt;
    let (parts, body) = response.into_parts();

    use http_body_util::BodyExt;
    let bytes = body.into_data_stream()
      .try_fold(Vec::new(), |mut acc, chunk| {
        acc.extend_from_slice(&chunk);
        futures::future::ok::<_, _>(acc)
      })
      .await?;
    let response = axum::http::Response::from_parts(parts, bytes);

    let things: Vec<AnyTwine> = v2::parse_response(response.into()).await?.try_collect().await?;
    Ok(things)
  }

  struct TestService{
    pub api: ApiService<MemoryStore>,
  }

  impl TestService {
    pub async fn has(&mut self, query: &str) -> bool {
      let request = axum::http::Request::builder()
        .method("HEAD")
        .uri(format!("/{}", query))
        .header("accept", "application/vnd.ipld.car")
        .body(axum::body::Body::empty())
        .unwrap();

      let response = self.api.call(request).await.unwrap();
      response.status() == StatusCode::OK
    }

    pub async fn get_one(&mut self, query: &str) -> AnyTwine {
      let request = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/{}", query))
        .header("accept", "application/vnd.ipld.car")
        .body(axum::body::Body::empty())
        .unwrap();

      let response = self.api.call(request).await.unwrap();

      assert_eq!(response.status(), StatusCode::OK);

      let mut things = parse_response(response).await.unwrap();

      assert_eq!(things.len(), 1);
      things.pop().unwrap()
    }

    pub async fn get_many(&mut self, query: &str) -> Vec<AnyTwine> {
      let request = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/{}", query))
        .header("accept", "application/vnd.ipld.car")
        .body(axum::body::Body::empty())
        .unwrap();

      let response = self.api.call(request).await.unwrap();
      assert_eq!(response.status(), StatusCode::OK);
      let things = parse_response(response).await.unwrap();
      things
    }

    pub async fn put(&mut self, query: &str, things: Vec<AnyTwine>) -> StatusCode {
      let request = axum::http::Request::builder()
        .method("PUT")
        .uri(format!("/{}", query))
        .header("accept", "application/vnd.ipld.car")
        .body(axum::body::Body::from(twine_lib::car::to_car_bytes(things, vec![Cid::default()])))
        .unwrap();

      let response = self.api.call(request).await.unwrap();
      response.status()
    }
  }

  #[tokio::test]
  async fn test_get_strands() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let mut service = TestService {
      api: api(store.clone(), Default::default()),
    };

    let strands = service.get_many("").await;

    assert_eq!(strands.len(), 1);
    let strand = strands[0].unwrap_strand();
    assert_eq!(strand.cid(), strand_cid);

    Ok(())
  }

  #[tokio::test]
  async fn test_get_single_strand() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let mut service = TestService {
      api: api(store.clone(), Default::default()),
    };

    let strand = service.get_one(&format!("{}", strand_cid)).await;
    let strand = strand.unwrap_strand();
    assert_eq!(strand.cid(), strand_cid);

    Ok(())
  }

  #[tokio::test]
  async fn test_get_range() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let mut service = TestService {
      api: api(store.clone(), Default::default()),
    };

    let twines = service.get_many(&format!("{}:1:=4", strand_cid)).await;
    assert_eq!(twines.len(), 4);
    let twines = twines.into_iter().map(|t| t.unwrap_tixel()).collect::<Vec<_>>();
    let indices = twines.iter().map(|t| t.index()).collect::<Vec<_>>();
    assert_eq!(indices, vec![1, 2, 3, 4]);
    Ok(())
  }

  #[tokio::test]
  async fn test_get_single() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let mut service = TestService {
      api: api(store.clone(), Default::default()),
    };
    let twine = service.get_one(&format!("{}:1", strand_cid)).await;
    let tixel = twine.unwrap_tixel();
    assert_eq!(tixel.index(), 1);

    Ok(())
  }

  #[tokio::test]
  async fn test_get_by_cid() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let index = 6;
    let tixel_cid = store.resolve_index(strand_cid, index).await.unwrap().cid();

    let mut service = TestService {
      api: api(store.clone(), ApiOptions::default()),
    };
    let twine = service.get_one(&format!("{}:{}", strand_cid, tixel_cid)).await;
    let tixel = twine.unwrap_tixel();
    assert_eq!(tixel.index(), index);
    assert_eq!(tixel.cid(), tixel_cid);

    Ok(())
  }

  #[tokio::test]
  async fn test_get_latest() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();
    let latest = store.resolve_latest(strand_cid).await.unwrap();

    let mut service = TestService {
      api: api(store.clone(), ApiOptions::default()),
    };
    let twine = service.get_one(&format!("{}:", strand_cid)).await;
    let tixel = twine.unwrap_tixel();
    assert_eq!(tixel.index(), latest.index());
    assert_eq!(tixel.cid(), latest.cid());

    Ok(())
  }

  #[tokio::test]
  async fn test_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let service = TestService {
      api: api(store.clone(), ApiOptions::default()),
    };
    let request = axum::http::Request::builder()
      .method("GET")
      .uri(format!("/{}:1000", strand_cid))
      .header("accept", "application/vnd.ipld.car")
      .body(axum::body::Body::empty())
      .unwrap();

    let response = service.api.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
  }

  #[tokio::test]
  async fn test_bad_query() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let service = TestService {
      api: api(store.clone(), ApiOptions::default()),
    };
    let request = axum::http::Request::builder()
      .method("GET")
      .uri(format!("/{}:1000:bad", strand_cid))
      .header("accept", "application/vnd.ipld.car")
      .body(axum::body::Body::empty())
      .unwrap();

    let response = service.api.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
  }

  #[tokio::test]
  async fn test_has() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let mut service = TestService {
      api: api(store.clone(), ApiOptions::default()),
    };

    let result = service.has(&format!("{}:1", strand_cid)).await;
    assert!(result);
    let result = service.has(&format!("{}:1000", strand_cid)).await;
    assert!(!result);

    Ok(())
  }

  #[tokio::test]
  async fn test_max_query_length() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let service = TestService {
      api: api(store.clone(), ApiOptions {
        max_query_length: 5,
        ..ApiOptions::default()
      }),
    };

    let request = axum::http::Request::builder()
      .method("GET")
      .uri(format!("/{}:0:=100", strand_cid))
      .header("accept", "application/vnd.ipld.car")
      .body(axum::body::Body::empty())
      .unwrap();

    let response = service.api.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
  }

  #[tokio::test]
  async fn test_saving() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();
    let other_store = MemoryStore::default();

    let mut service = TestService {
      api: api(other_store.clone(), ApiOptions {
        read_only: false,
        ..ApiOptions::default()
      }),
    };

    use futures::TryStreamExt;
    let strand = store.resolve_strand(&strand_cid).await.unwrap().unpack();
    let tixels: Vec<AnyTwine> = store.resolve_range((strand.clone(), ..)).await?
      .and_then(|t| async { Ok(t.into()) })
      .try_collect().await?;

    let ret = service.put("", vec![
      strand.clone().into()
    ]).await;

    assert_eq!(ret, StatusCode::CREATED);

    let ret = service.put(&format!("{}", strand_cid), tixels).await;
    assert_eq!(ret, StatusCode::CREATED);

    let fourth = store.resolve_index(strand_cid, 4).await.unwrap();
    let tixel = other_store.resolve_index(strand_cid, 4).await.unwrap();
    assert_eq!(fourth.cid(), tixel.cid());

    Ok(())
  }

  #[tokio::test]
  async fn check_header() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();
    let service = TestService {
      api: api(store.clone(), ApiOptions::default()),
    };
    let request = axum::http::Request::builder()
      .method("GET")
      .uri(format!("/{}:1", strand_cid))
      .header("accept", "application/vnd.ipld.car")
      .body(axum::body::Body::empty())
      .unwrap();

    let response = service.api.call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      response.headers().get("X-Spool-Version").unwrap(),
      "2".parse::<axum::http::HeaderValue>().unwrap()
    );
    Ok(())
  }
}
