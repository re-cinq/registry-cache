// SPDX-License-Identifier: Apache-2.0
pub mod blobs;
pub mod forward;
pub mod manifests;

use actix_web::{HttpRequest, HttpResponse, web};
use actix_web::http::{header, Method};
use actix_web::http::header::{HeaderName, HeaderValue};
use reqwest::RequestBuilder;
use url::Url;
use crate::api::registry::blobs::RepositoryRequest;
use crate::api::state::AppState;
use crate::error::error_kind::ErrorKind;
use crate::error::registry::RegistryError;
use crate::metrics;
use crate::models::types::MimeType;
use crate::registry::repository::Repository;

/// Serve the content from the cache via the repository info
async fn serve_from_cache(req: HttpRequest, repository: Repository, mime: Option<MimeType>, state: &web::Data<AppState>) -> Result<HttpResponse, RegistryError> {

    // Image name
    let image_name = repository.name.clone();
    let repository_digest = repository.digest.clone();

    // Load the file
    let file = actix_files::NamedFile::open_async(state.storage.blob_path(repository)).await
        .map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

    // Add the content type if we have it
    let file = if let Some(mime) = mime {
        file.set_content_type(mime.parse().unwrap())
    } else {
        file
    };

    // Convert to response
    let mut response = file.into_response(&req);

    // Add the digest and etag if present
    if let Some(ref digest) = repository_digest {

        let digest_string = HeaderValue::from_str(&digest.to_string())
            .map_err(|e| RegistryError::new(ErrorKind::InternalError).with_error(e.to_string()))?;

        // Add the docker-content-digest
        response.headers_mut().insert(HeaderName::from_static("docker-content-digest"), digest_string.clone());

        // Add the etag
        response.headers_mut().insert(HeaderName::from_static("etag"), digest_string);
    }

    // Collect the metrics for the cached data
    metrics::CACHED_RESPONSES.inc();
    metrics::RESPONSE_CODE_COLLECTOR.with_label_values(&[response.status().as_str(), req.method().as_str(), &image_name]).inc();

    // Logging
    log::info!("*** Cached: {} {}", req.method(), req.uri());

    // Return the response
    Ok(response)
}

/// Builds the upstream request URL starting from the client one
fn build_upstream_req(req: &HttpRequest,  method: Method, state: &web::Data<AppState>) -> Result<RequestBuilder, RegistryError> {

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

    // Create the upstream request
    let mut upstream_request = state.client
        .request(method, new_url);

    // Append the client request headers to the upstream request
    for (header_name, header_value) in req.headers().iter().filter(|(h, _)| *h != "host") {
        upstream_request = upstream_request.header(header_name, header_value);
    }

    // TODO: This forwarded implementation is incomplete as it only handles the unofficial
    // X-Forwarded-For header but not the official Forwarded one.
    let upstream_request = match req.peer_addr() {
        Some(addr) => upstream_request.header("X-Forwarded-For", addr.ip().to_string()),
        None => upstream_request,
    };


    // Return the new URL
    Ok(upstream_request)

}

async fn validate_repository(repository_request: web::Path<RepositoryRequest>) -> Result<Repository, RegistryError> {
    // parse the name from the request
    let repository = repository_request.into_inner();

    // validate the repository
    let repository = repository.is_valid().await?;

    Ok(repository)
}