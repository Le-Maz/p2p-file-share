#![feature(slice_as_array)]

use std::ops::Deref;

use anyhow::anyhow;
use iroh::{SecretKey, Watcher};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Endpoint(iroh::Endpoint);

#[wasm_bindgen]
impl Endpoint {
    pub async fn new() -> Result<Endpoint, AnyhowError> {
        let endpoint = iroh::Endpoint::builder().discovery_n0().bind().await?;
        Ok(Endpoint(endpoint))
    }

    pub async fn new_with_secret_key(secret_key: Vec<u8>) -> Result<Endpoint, AnyhowError> {
        let secret_key = secret_key
            .as_array()
            .ok_or(anyhow!("Bad secret key"))
            .map_err(AnyhowError)?;
        let endpoint = iroh::Endpoint::builder()
            .discovery_n0()
            .secret_key(SecretKey::from_bytes(secret_key))
            .bind()
            .await?;
        Ok(Endpoint(endpoint))
    }

    pub fn node_id(&self) -> String {
        self.0.node_id().to_string()
    }

    pub fn secret_key(&self) -> Vec<u8> {
        self.0.secret_key().to_bytes().to_vec()
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
