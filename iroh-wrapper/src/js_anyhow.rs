use std::ops::Deref;

use anyhow::Error;
use js_sys::{Object, wasm_bindgen::JsValue};
use wasm_bindgen::prelude::*;

pub trait JsAnyhow<T> {
    fn js_anyhow(self) -> Result<T, AnyhowError>;
}

impl<T> JsAnyhow<T> for Result<T, JsValue> {
    fn js_anyhow(self) -> Result<T, AnyhowError> {
        self.map_err(|js_value| {
            let js_string = Object::to_string(&js_value.into());
            let string = String::from(js_string);
            AnyhowError(Error::msg(string))
        })
    }
}

#[wasm_bindgen]
pub struct AnyhowError(anyhow::Error);

#[wasm_bindgen]
impl AnyhowError {
    #[allow(non_snake_case)]
    pub fn toString(&self) -> String {
        self.0.to_string()
    }
}

impl AnyhowError {
    pub fn new(inner: anyhow::Error) -> Self {
        Self(inner)
    }
}

impl<ErrType> From<ErrType> for AnyhowError
where
    ErrType: Into<anyhow::Error>,
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
