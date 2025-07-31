use crate::{context::Context, utils::client::Client};
use axum::{
    extract::Request,
    http::{HeaderValue, header::HOST},
    response::{IntoResponse, Response},
};
use futures_util::future::BoxFuture;
use hyper::{StatusCode, Uri};
use std::{convert::Infallible, fmt, pin::Pin, sync::Arc, task::Poll};
use tower::{Layer, Service};

#[derive(Debug, Clone)]
pub struct ProxyLayer {
    proxies: Arc<Vec<crate::config::Proxy>>,
    client: Arc<Client>,
}

impl ProxyLayer {
    pub fn new(context: &Context) -> Self {
        Self {
            proxies: Arc::new(context.config.server.proxy.clone()),
            client: context.client.clone(),
        }
    }
}

impl<S> Layer<S> for ProxyLayer {
    type Service = Proxy<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Proxy {
            inner,
            proxies: self.proxies.clone(),
            client: self.client.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Proxy<I> {
    inner: I,
    proxies: Arc<Vec<crate::config::Proxy>>,
    client: Arc<Client>,
}

impl<I> Clone for Proxy<I>
where
    I: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            proxies: self.proxies.clone(),
            client: self.client.clone(),
        }
    }
}

impl<I> Service<Request> for Proxy<I>
where
    I: Service<Request, Error = Infallible> + Clone + Send + Sync + 'static,
    I::Response: IntoResponse,
    I::Future: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = ResponseFuture;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        if self.proxies.is_empty() {
            let mut inner = self.inner.clone();
            let future = Box::pin(async move { inner.call(req).await.into_response() });
            return ResponseFuture { inner: future };
        }

        let uri = req.uri();
        let path = uri.path();
        for proxy in self.proxies.iter() {
            if path.starts_with(&proxy.proxy) {
                // uri
                let path_and_query = uri.path_and_query().map(|v| v.as_str()).unwrap_or_default();
                let new_uri = format!("{}{}", proxy.target, path_and_query);
                match Uri::try_from(new_uri) {
                    Ok(uri) => {
                        *req.uri_mut() = uri;
                    }
                    Err(_) => continue,
                }
                // change_origin
                if proxy.change_origin {
                    let host = HeaderValue::from_str(&host(req.uri()).unwrap()).unwrap();
                    req.headers_mut().insert(HOST, host);
                }

                let client = self.client.clone();
                let future = Box::pin(async move {
                    match client.request(req).await {
                        Ok(response) => response.into_response(),
                        Err(_) => {
                            (StatusCode::BAD_GATEWAY, "Backend service unavailable").into_response()
                        }
                    }
                });
                return ResponseFuture { inner: future };
            }
        }

        let mut inner = self.inner.clone();
        let future = Box::pin(async move { inner.call(req).await.into_response() });
        ResponseFuture { inner: future }
    }
}

pub struct ResponseFuture {
    inner: BoxFuture<'static, Response>,
}

impl Future for ResponseFuture {
    type Output = Result<Response, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.inner.as_mut().poll(cx).map(Ok)
    }
}

impl fmt::Debug for ResponseFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseFuture").finish()
    }
}

fn host(uri: &Uri) -> Option<String> {
    let host = uri.host()?;

    let Some(scheme) = uri.scheme_str() else {
        return Some(host.to_string());
    };
    let Some(port) = uri.port_u16() else {
        return Some(host.to_string());
    };
    let required = match scheme {
        "http" | "ws" => port != 80,

        "https" | "wss" => port != 443,
        _ => port != 0,
    };

    if required {
        Some(format!("{host}:{port}"))
    } else {
        Some(host.to_string())
    }
}
