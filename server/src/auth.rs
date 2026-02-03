use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiKeyLayer {
    pub api_key: Arc<String>,
}

impl ApiKeyLayer {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Arc::new(api_key),
        }
    }
}

impl<S> tower::Layer<S> for ApiKeyLayer {
    type Service = ApiKeyService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ApiKeyService {
            inner,
            api_key: self.api_key.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ApiKeyService<S> {
    inner: S,
    api_key: Arc<String>,
}

impl<S, B> tower::Service<axum::http::Request<B>> for ApiKeyService<S>
where
    S: tower::Service<axum::http::Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<B>) -> Self::Future {
        let api_key = self.api_key.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let provided = req
                .headers()
                .get("X-API-Key")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if provided != api_key.as_str() {
                let body = json!({
                    "data": null,
                    "error": {
                        "code": 401,
                        "message": "Invalid or missing API key"
                    }
                });
                return Ok((StatusCode::UNAUTHORIZED, axum::Json(body)).into_response());
            }

            inner.call(req).await
        })
    }
}
