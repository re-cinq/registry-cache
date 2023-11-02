// SPDX-License-Identifier: Apache-2.0
use actix_web::{
    http::Method, web, HttpRequest, HttpResponse
};
use actix_web::http::header::HeaderValue;
use futures_util::{pin_mut, StreamExt as _, TryStreamExt};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use crate::api::registry::blobs::RepositoryRequest;
use crate::api::registry::{build_upstream_req, serve_from_cache, validate_repository};
use crate::api::state::AppState;
use crate::error::error_kind::ErrorKind;
use crate::error::registry::RegistryError;
use crate::metrics;
use crate::models::commands::RegistryCommand;
use crate::registry::digest::Digest;
use crate::registry::repository::Repository;


/// Handle the manifests requests
pub async fn get_manifests(manifest_request: web::Path<RepositoryRequest>,
                           req: HttpRequest,
                           method: Method,
                           state: web::Data<AppState>) -> Result<HttpResponse, RegistryError> {

    // Increase the requests counter
    metrics::INCOMING_REQUESTS.inc();

    // Build the upstream URL
    let upstream_request = build_upstream_req(&req, method, &state)?;

    // Build the upstream request
    let upstream_request = upstream_request.build().map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

    // Log the upstream request
    log::info!("Upstream: {} {}", upstream_request.method(), upstream_request.url());

    // Execute the request against the upstream
    let upstream_response = state.client.execute(upstream_request).await;

    // In case we get a timeout, from upstream, then serve the manifest from the cache, if present
    if let Err(ref e) = upstream_response {

        // if we got a timeout error, then serve it from cache, if present
        if e.is_timeout() {
            return handle_upstream_error(req, manifest_request, &state).await;
        }
    }

    // If we got here, we can safely unwrap
    let upstream_response = upstream_response.unwrap();

    // If we got an upstream error, try to serve the manifest from the cache, if present
    if upstream_response.status().is_server_error() {
        return handle_upstream_error(req, manifest_request, &state).await;
    }

    // Otherwise pipe the request upstream and store the manifest in cache

    // ---------------------------------------------------------------------------------------------
    // Get the repository from the request
    let manifest_repository = validate_repository(manifest_request).await?;

    // ---------------------------------------------------------------------------------------------
    // Get the manifest digest from the upstream response
    let manifest_digest = upstream_response.headers().get("docker-content-digest").cloned()
        .unwrap_or_else(|| HeaderValue::from_str("").unwrap()).to_str().unwrap_or("").to_string();

    // Parse it
    let manifest_digest = if manifest_digest.is_empty() {
        None
    } else {
        Digest::parse(&manifest_digest).ok()
    };

    // ---------------------------------------------------------------------------------------------
    // Get the content-type from the upstream response
    let content_type = upstream_response.headers().get("content-type").cloned()
        .unwrap_or_else(|| HeaderValue::from_static("")).to_str().unwrap_or("").to_string();

    // ---------------------------------------------------------------------------------------------

    // Build the response for the client
    let mut client_resp = HttpResponse::build(upstream_response.status());

    // Remove `Connection` as per
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    for (header_name, header_value) in upstream_response.headers().iter().filter(|(h, _)| *h != "connection") {
        client_resp.insert_header((header_name.clone(), header_value.clone()));
        // tracing::info!("Response header: {}: {:?}", header_name, header_value);
    }

    // Status code
    let status = upstream_response.status().to_string();

    // Create the client response channel
    let (mut response_tx, response_rx) = tokio::io::duplex(8192); //mpsc::unbounded_channel();
    let stream = tokio_util::codec::FramedRead::new(response_rx, tokio_util::codec::BytesCodec::new()).map_ok(|b| b.freeze());

    // Create the persistence channels
    let (persist_tx,persist_rx) = mpsc::unbounded_channel();

    // Ask the bus to store the data
    let persist_command = RegistryCommand::PersistManifest(manifest_repository, manifest_digest, 0, content_type, persist_rx);
    state.command_bus.publish(persist_command).await;

    // Consume the stream and send it to 2 channels:
    // - the response channel to send to the client
    // - the persist channel to persist the blob
    let _handle = tokio::spawn(async move {
        let stream = upstream_response.bytes_stream();
        pin_mut!(stream);

        while let Some(chunk) = stream.next().await {
            if let Ok(ref chunk) = chunk {
                if let Err(e) = persist_tx.send(chunk.clone()) {
                    tracing::error!("Failed to send manifest blob chunk for persistence: {}", e.to_string());
                }
                if let Err(e) = response_tx.write_all(chunk).await {
                    tracing::error!("Failed to send manifest blob chunk for client response: {}", e.to_string());
                }
            }
        }
    });

    metrics::UPSTREAM_RESPONSES.inc();
    metrics::RESPONSE_CODE_COLLECTOR.with_label_values(&[status.as_str(), req.method().as_ref(), ""]).inc();

    Ok(client_resp.streaming(stream))
}


/// Handles the client request in case the upstream timed out or returned an error
async fn handle_upstream_error(req: HttpRequest, manifest_request: web::Path<RepositoryRequest>, state: &web::Data<AppState>) -> Result<HttpResponse, RegistryError> {

    // parse the name from the request
    let repository = validate_repository(manifest_request).await?;

    // Load the manifest record
    let manifest_record = state.manifests.get(&repository).await?;

    match manifest_record {
        Some(manifest) => {

            // It means we don't have a blob cache for this specific tag
            // We can't do anything at this stage so return an error
            if let None = manifest.reference {
                return Err(RegistryError::new(ErrorKind::RegistryManifestUnknown));
            }

            // Build the manifest repository
            let manifest_repository = Repository::new_with_reference(&manifest.name, &manifest.reference.unwrap().to_string())?;

            // Serve the content from cache
            serve_from_cache(req, manifest_repository,Some(manifest.mime), state).await
        },
        None => {
            Err(RegistryError::new(ErrorKind::RegistryManifestUnknown))
        }
    }

}