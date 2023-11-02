// SPDX-License-Identifier: Apache-2.0
use actix_web::{
   http::Method, web, HttpRequest, HttpResponse
};
use futures_util::{StreamExt as _};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use crate::api::registry::build_upstream_req;
use crate::api::state::AppState;
use crate::error::error_kind::ErrorKind;
use crate::error::registry::RegistryError;
use crate::metrics;


/// Forward the request to upstream
pub async fn forward(req: HttpRequest, mut payload: web::Payload,
                     method: Method,
                     state: web::Data<AppState>) -> Result<HttpResponse, RegistryError> {

    // Increase the requests counter
    metrics::INCOMING_REQUESTS.inc();

    // Build the upstream URL
    let upstream_request = build_upstream_req(&req, method, &state)?;

    // Create a new channel
    let (tx, rx) = mpsc::unbounded_channel();

    // Start a new task where we forward a possible payload
    actix_web::rt::spawn(async move {
        while let Some(chunk) = payload.next().await {
            tx.send(chunk).unwrap();
        }
    });

    // Add the body
    let upstream_request = upstream_request.body(reqwest::Body::wrap_stream(UnboundedReceiverStream::new(rx)));

    // Build the upstream request
    let upstream_request = upstream_request.build().map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

    // Logging
    log::info!("Upstream: {} {}", upstream_request.method(), upstream_request.url());

    // Execute the request against the upstream
    let res = state.client.execute(upstream_request).await
        .map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

    // Build the response for the client
    let mut client_resp = HttpResponse::build(res.status());
    // Remove `Connection` as per
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    for (header_name, header_value) in res.headers().iter().filter(|(h, _)| *h != "connection") {
        client_resp.insert_header((header_name.clone(), header_value.clone()));
        tracing::info!("Response header: {}: {:?}", header_name, header_value);
    }

    metrics::UPSTREAM_RESPONSES.inc();
    metrics::RESPONSE_CODE_COLLECTOR.with_label_values(&[res.status().as_str(), req.method().as_ref(), ""]).inc();

    Ok(client_resp.streaming(res.bytes_stream()))


}