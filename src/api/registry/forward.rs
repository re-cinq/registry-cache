// SPDX-License-Identifier: Apache-2.0
use actix_web::{
    dev::PeerAddr, http::Method, web, HttpRequest, HttpResponse
};
use actix_web::http::header::HeaderValue;
use futures_util::{StreamExt as _};
use reqwest::header;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use url::Url;
use crate::api::state::AppState;
use crate::error::error_kind::ErrorKind;
use crate::error::registry::RegistryError;
use crate::metrics;


/// Forward the request to upstream
pub async fn forward(req: HttpRequest, mut payload: web::Payload,
                     method: Method, peer_addr: Option<PeerAddr>,
                     state: web::Data<AppState>) -> Result<HttpResponse, RegistryError> {

    let host_header = req.headers().get(header::HOST).cloned().unwrap_or_else(|| HeaderValue::from_static(""));
    let host = host_header.to_str().unwrap_or("");
    let upstream = state.upstreams.get(host);

    if upstream.is_none() {
        tracing::error!("Upstream not found for host {}", host);
        return Err(RegistryError::new(ErrorKind::NotFound));
    }

    // Increase the requests counter
    metrics::INCOMING_REQUESTS.inc();

    let upstream = upstream.unwrap();
    let forward_url = format!("{}://{}", upstream.schema, upstream.registry);

    // Rewrite the URL
    let mut new_url = Url::parse(&forward_url).unwrap();

    // Convert the original request URI to string
    let path = req.uri().path();

    // Set the URL path
    new_url.set_path(path);

    // Set the URL query string parameters
    new_url.set_query(req.uri().query());

    // Create a new channel
    let (tx, rx) = mpsc::unbounded_channel();

    // Start a new task where we forward a possible payload
    actix_web::rt::spawn(async move {
        while let Some(chunk) = payload.next().await {
            tx.send(chunk).unwrap();
        }
    });

    // Create the upstream request
    let mut forwarded_req = state.client
        .request(method, new_url)
        .body(reqwest::Body::wrap_stream(UnboundedReceiverStream::new(rx)));

    // Append the client request headers to the upstream request
    for (header_name, header_value) in req.headers().iter().filter(|(h, _)| *h != "host") {
        forwarded_req = forwarded_req.header(header_name, header_value);
    }

    // TODO: This forwarded implementation is incomplete as it only handles the unofficial
    // X-Forwarded-For header but not the official Forwarded one.
    let forwarded_req = match peer_addr {
        Some(PeerAddr(addr)) => forwarded_req.header("X-Forwarded-For", addr.ip().to_string()),
        None => forwarded_req,
    };

    let upstream_request = forwarded_req.build().map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

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
        //tracing::info!("Response header: {}: {:?}", header_name, header_value);
    }

    metrics::UPSTREAM_RESPONSES.inc();
    metrics::RESPONSE_CODE_COLLECTOR.with_label_values(&[res.status().as_str(), req.method().as_ref(), ""]).inc();

    Ok(client_resp.streaming(res.bytes_stream()))
}