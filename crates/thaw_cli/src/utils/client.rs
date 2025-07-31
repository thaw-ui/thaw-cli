use axum::body::Body;
use hyper_util::{
    client::legacy::{Client as HyperClient, connect::HttpConnector},
    rt::TokioExecutor,
};
use std::ops::Deref;

#[derive(Debug)]
pub struct Client(HyperClient<HttpConnector, Body>);

impl Client {
    pub fn new() -> Self {
        Self(HyperClient::<(), ()>::builder(TokioExecutor::new()).build(HttpConnector::new()))
    }
}

impl Deref for Client {
    type Target = HyperClient<HttpConnector, Body>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
