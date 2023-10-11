// SPDX-License-Identifier: Apache-2.0
use actix_web::{get, HttpResponse, HttpResponseBuilder};
use actix_web::http::StatusCode;
use prometheus::{Encoder, TextEncoder};
use crate::error::registry::RegistryError;

#[get("/metrics")]
pub(crate) async fn metrics_handler() -> Result<HttpResponse, RegistryError>  {

    let encoder = TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        log::error!("could not encode prometheus metrics: {}", e);
    };
    let res_custom = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            log::error!("prometheus metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();


    Ok(HttpResponseBuilder::new(StatusCode::OK).body(res_custom))
}