use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    Error,
};
use actix_web::middleware::Next;
use std::time::Instant;

/// Logging middleware that logs request and response details
pub async fn request_logger(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.path().to_string();
    let query = req.query_string().to_string();
    let remote_addr = req
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "-".to_string());

    // Log request
    if !query.is_empty() {
        tracing::info!(
            "--> {} {} ?{} (from: {})",
            method,
            path,
            query,
            remote_addr
        );
    } else {
        tracing::info!("--> {} {} (from: {})", method, path, remote_addr);
    }

    // Call the next service
    let res = next.call(req).await?;

    // Log response
    let elapsed = start.elapsed().as_millis();
    let status = res.status().as_u16();

    if status >= 400 {
        tracing::warn!(
            "<-- {} {} {} ({}ms)",
            method,
            path,
            status,
            elapsed
        );
    } else {
        tracing::info!(
            "<-- {} {} {} ({}ms)",
            method,
            path,
            status,
            elapsed
        );
    }

    Ok(res)
}
