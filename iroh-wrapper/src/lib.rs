#![feature(slice_as_array)]

use std::ops::Deref;

use iroh::Watcher;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Endpoint(iroh::Endpoint);

#[wasm_bindgen]
impl Endpoint {
    pub async fn new() -> Result<Endpoint, AnyhowError> {
        let endpoint = iroh::Endpoint::builder().discovery_n0().bind().await?;
        Ok(Endpoint(endpoint))
    }

    pub fn node_id(&self) -> String {
        self.0.node_id().to_string()
    }

    pub async fn initialized(&self) {
        self.0.node_addr().initialized().await;
    }
}

#[wasm_bindgen]
pub struct AnyhowError(anyhow::Error);

impl<ErrType> From<ErrType> for AnyhowError
where
    ErrType: std::error::Error + Send + Sync + 'static,
{
    fn from(value: ErrType) -> Self {
        Self(value.into())
    }
}

impl Deref for AnyhowError {
    type Target = anyhow::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
