use axum::{
    extract::{Path, Query, State},
    response::{Json, IntoResponse},
    routing::{get, Router},
    http::StatusCode,
};
use twine_lib::{twine::AnyTwine, serde::dag_json};
use twine_lib::{errors::ResolutionError, store::Store, resolver::Resolver};
use std::sync::Arc;
use serde::Deserialize;

pub use api::api;

mod api {
  use axum::http::HeaderMap;
  use twine_lib::errors::{ConversionError, StoreError, VerificationError};
  use super::*;

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
    // #[error("Unauthorized")]
    // Unauthorized,
  }

  impl From<ConversionError> for ApiError {
    fn from(e: ConversionError) -> Self {
      ApiError::BadRequestData(e.to_string())
    }
  }

  impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
      match self {
        ApiError::ServerError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        ApiError::VerificationError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
        ApiError::MalformedCid(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        ApiError::BadRequestData(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        // ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
        ApiError::ResolutionError(e) => match e {
          ResolutionError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
          _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        ApiError::StoreError(e) => match e {
          StoreError::Fetching(e) => match e {
            ResolutionError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
          },
          _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
      }
    }
  }

  #[derive(Debug, Deserialize, Clone)]
  struct Truthy(Option<String>);

  impl From<Truthy> for bool {
    fn from(t: Truthy) -> bool {
      t.0.map_or(false, |s| s.to_ascii_lowercase() != "false")
    }
  }

  impl Default for Truthy {
    fn default() -> Self {
      Truthy(None)
    }
  }

  #[derive(Debug, Deserialize, Clone)]
  struct QueryParams {
    #[serde(default)]
    full: Truthy,
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

  /// Create a new router for a twine http api using the given store
  pub fn api<S: Store + Resolver + 'static>(
    store: S,
  ) -> Router {
    let store = Arc::new(store);
    Router::new()
      .route("/", get(list_strands))
      .route("/{query}", get(query))
      .with_state(store)
  }

  async fn list_strands<S: Store + Resolver>(
    headers: HeaderMap,
    State(store): State<Arc<S>>,
  ) -> Result<axum::response::Response, ApiError> {
    handlers::list_strands(store, wants_car(&headers)).await
  }

  async fn query<S: Store + Resolver>(
    headers: HeaderMap,
    State(store): State<Arc<S>>,
    Path(query): Path<String>,
    Query(query_params): Query<QueryParams>,
  ) -> Result<axum::response::Response, ApiError> {

    handlers::query(
      query,
      store,
      wants_car(&headers),
      query_params.full.into(),
    ).await
  }
}

mod handlers {
  use super::*;
  use twine_lib::resolver::AnyQuery;
  use axum::response::IntoResponse;

  pub async fn query<S: Store + Resolver>(
    q: String,
    store: Arc<S>,
    as_car: bool,
    full: bool,
  ) -> Result<axum::response::Response, api::ApiError> {
    let result = match q.parse::<AnyQuery>() {
      Ok(query) => match query {
        AnyQuery::Strand(strand_cid) => {
          let strand = store.resolve_strand(&strand_cid).await?;
          models::AnyResult::Strands {
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
          models::AnyResult::Tixels {
            items: vec![(*twine.unpack()).clone().into()],
            strand,
          }
        }
        AnyQuery::Many(range) => {
          use futures::TryStreamExt;
          let tixels: Vec<_> = store.resolve_range(range).await?.try_collect().await?;
          let strand = if full {
            Some((*tixels[0].strand()).clone().into())
          } else {
            None
          };
          models::AnyResult::Tixels {
            items: tixels.into_iter().map(|t| (*t).clone().into()).collect(),
            strand,
          }
        }
      },
      Err(_) => return Err(api::ApiError::BadRequestData("Invalid query".to_string())),
    };
    if as_car {
      Ok(models::Car(result).into_response())
    } else {
      Ok(Json(result).into_response())
    }
  }

  pub async fn list_strands<S: Store + Resolver>(
    store: Arc<S>,
    as_car: bool,
  ) -> Result<axum::response::Response, api::ApiError> {
    use futures::TryStreamExt;
    let strands: Vec<_> = store.strands().await?.try_collect().await?;
    let result = models::AnyResult::Strands {
      items: strands.into_iter().map(|s| s.clone().into()).collect(),
    };
    if as_car {
      Ok(models::Car(result).into_response())
    } else {
      Ok(Json(result).into_response())
    }
  }
}

mod models {
  use super::*;
  use serde::{Deserialize, Serialize};
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

  pub struct Car(pub AnyResult);

  impl IntoResponse for Car {
    fn into_response(self) -> axum::response::Response {
      let items = match self.0 {
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
      car_bytes.into_response()
    }
  }
}

#[cfg(test)]
mod test {
  use crate::v2;
  use super::*;
  use twine_builder::{RingSigner, TwineBuilder};
  use twine_lib::{ipld_core::ipld, store::MemoryStore, Cid};

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
    response: axum::http::Response<axum::body::Body>,
  ) -> Result<Vec<AnyTwine>, Box<dyn std::error::Error>> {
    use futures::TryStreamExt;
    let (parts, body) = response.into_parts();

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
    pub router: Router,
  }

  impl TestService {
    pub async fn get_one(&mut self, query: &str) -> AnyTwine {
      let request = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/{}", query))
        .header("accept", "application/vnd.ipld.car")
        .body(axum::body::Body::empty())
        .unwrap();

      use tower_service::Service;
      let response = self.router.as_service().call(request).await.unwrap();

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

      use tower_service::Service;
      let response = self.router.as_service().call(request).await.unwrap();
      assert_eq!(response.status(), StatusCode::OK);
      let things = parse_response(response).await.unwrap();
      things
    }
  }

  #[tokio::test]
  async fn test_get_strands() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let mut service = TestService {
      router: api::api(store.clone()),
    };

    let strands = service.get_many("").await;

    assert_eq!(strands.len(), 1);
    let strand = strands[0].unwrap_strand();
    assert_eq!(strand.cid(), strand_cid);

    Ok(())
  }

  #[tokio::test]
  async fn test_get_range() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let mut service = TestService {
      router: api::api(store.clone()),
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
      router: api::api(store.clone()),
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
      router: api::api(store.clone()),
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
      router: api::api(store.clone()),
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

    let mut service = TestService {
      router: api::api(store.clone()),
    };
    let request = axum::http::Request::builder()
      .method("GET")
      .uri(format!("/{}:1000", strand_cid))
      .header("accept", "application/vnd.ipld.car")
      .body(axum::body::Body::empty())
      .unwrap();

    use tower_service::Service;
    let response = service.router.as_service().call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    Ok(())
  }

  #[tokio::test]
  async fn test_bad_query() -> Result<(), Box<dyn std::error::Error>> {
    let store = MemoryStore::default();
    let strand_cid = make_strand(&store).await.unwrap();

    let mut service = TestService {
      router: api::api(store.clone()),
    };
    let request = axum::http::Request::builder()
      .method("GET")
      .uri(format!("/{}:1000:bad", strand_cid))
      .header("accept", "application/vnd.ipld.car")
      .body(axum::body::Body::empty())
      .unwrap();

    use tower_service::Service;
    let response = service.router.as_service().call(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    Ok(())
  }
}
