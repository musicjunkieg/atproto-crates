//! RASL HTTP handlers for serving content-addressed resources via axum.
//!
//! Provides four handler types matching the RASL specification:
//! - [`redirect_handler`]: Maps CIDs to redirect URLs (HTTP 307)
//! - [`func_handler`]: Custom function-based CID lookup
//! - [`directory_handler`]: Pre-hashes a directory and serves files by CID
//! - [`cid_directory_handler`]: Serves files from a directory where filenames are CIDs
//!
//! All handlers serve content at `/.well-known/rasl/{cid}` and enforce
//! GET/HEAD methods only with `Content-Type: application/octet-stream`.
//!
//! Requires the `axum` feature.

use crate::cid::{CidCore, compute_raw_cid, compute_raw_cid_blake3};
use crate::errors::RaslError;
use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

/// RASL well-known route path pattern for axum.
const RASL_ROUTE: &str = "/.well-known/rasl/{cid}";

/// Content-Type header value for RASL responses.
const OCTET_STREAM: &str = "application/octet-stream";

/// Create an axum router that redirects CID requests to specified URLs.
///
/// Maps CID strings to redirect target URLs. Returns HTTP 307 Temporary
/// Redirect for known CIDs and 404 for unknown ones.
///
/// # Example
///
/// ```rust,no_run
/// use std::collections::HashMap;
/// use atproto_dasl::rasl::handler::redirect_handler;
///
/// let mut redirects = HashMap::new();
/// redirects.insert(
///     "bafkreifn5yxi7nkftsn46b6x26grda57ict7md2xuvfbsgkiahe2e7vnq4".to_string(),
///     "https://cdn.example.com/data.bin".to_string(),
/// );
/// let router = redirect_handler(redirects);
/// ```
pub fn redirect_handler(redirects: HashMap<String, String>) -> Router {
    let state = Arc::new(redirects);
    Router::new()
        .route(RASL_ROUTE, get(handle_redirect).head(handle_redirect_head))
        .with_state(state)
}

async fn handle_redirect(
    State(redirects): State<Arc<HashMap<String, String>>>,
    Path(cid): Path<String>,
) -> Response {
    match redirects.get(&cid) {
        Some(target) => {
            let mut response = Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT)
                .body(Body::empty())
                .unwrap();
            if let Ok(value) = HeaderValue::from_str(target) {
                response.headers_mut().insert(header::LOCATION, value);
            }
            response
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn handle_redirect_head(
    State(redirects): State<Arc<HashMap<String, String>>>,
    Path(cid): Path<String>,
) -> Response {
    match redirects.get(&cid) {
        Some(target) => {
            let mut response = Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT)
                .body(Body::empty())
                .unwrap();
            if let Ok(value) = HeaderValue::from_str(target) {
                response.headers_mut().insert(header::LOCATION, value);
            }
            response
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// State for the function handler.
struct FuncState<F> {
    func: F,
}

/// Create an axum router that resolves CIDs using a custom function.
///
/// The function receives a parsed [`CidCore`] and returns `Option<Vec<u8>>`.
/// Returns the data with `Content-Type: application/octet-stream` when
/// the function returns `Some`, or 404 when it returns `None`.
///
/// # Example
///
/// ```rust,no_run
/// use atproto_dasl::rasl::handler::func_handler;
///
/// let router = func_handler(|cid| async move {
///     // Look up data by CID from some storage
///     Some(b"content data".to_vec())
/// });
/// ```
pub fn func_handler<F, Fut>(f: F) -> Router
where
    F: Fn(CidCore) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Option<Vec<u8>>> + Send + 'static,
{
    let state = Arc::new(FuncState { func: f });
    Router::new()
        .route(
            RASL_ROUTE,
            get(handle_func::<F, Fut>).head(handle_func_head::<F, Fut>),
        )
        .with_state(state)
}

async fn handle_func<F, Fut>(
    State(state): State<Arc<FuncState<F>>>,
    Path(cid_str): Path<String>,
) -> Response
where
    F: Fn(CidCore) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Option<Vec<u8>>> + Send + 'static,
{
    let cid = match parse_cid_from_path(&cid_str) {
        Some(cid) => cid,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    match (state.func.clone())(cid).await {
        Some(data) => {
            (StatusCode::OK, [(header::CONTENT_TYPE, OCTET_STREAM)], data).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn handle_func_head<F, Fut>(
    State(state): State<Arc<FuncState<F>>>,
    Path(cid_str): Path<String>,
) -> Response
where
    F: Fn(CidCore) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Option<Vec<u8>>> + Send + 'static,
{
    let cid = match parse_cid_from_path(&cid_str) {
        Some(cid) => cid,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    match (state.func.clone())(cid).await {
        Some(data) => {
            let len = data.len().to_string();
            let mut response = Response::builder()
                .status(StatusCode::OK)
                .body(Body::empty())
                .unwrap();
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static(OCTET_STREAM));
            if let Ok(value) = HeaderValue::from_str(&len) {
                response.headers_mut().insert(header::CONTENT_LENGTH, value);
            }
            response
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Directory handler state containing pre-computed CID-to-path mappings.
#[derive(Clone)]
struct DirectoryState {
    files: Arc<HashMap<String, PathBuf>>,
}

/// Create an axum router that serves files from a directory by their CID.
///
/// On initialization, all files in the directory are hashed with SHA-256
/// (and optionally BLAKE3). A mapping from CID string to file path is built.
/// Requests for known CIDs serve the corresponding file; unknown CIDs get 404.
///
/// # Errors
///
/// Returns `RaslError::DirectoryError` if the directory cannot be read.
///
/// # Example
///
/// ```rust,no_run
/// use std::path::PathBuf;
/// use atproto_dasl::rasl::handler::directory_handler;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let router = directory_handler(PathBuf::from("/srv/rasl-content"), false).await?;
/// # Ok(())
/// # }
/// ```
pub async fn directory_handler(dir: PathBuf, hash_blake3: bool) -> Result<Router, RaslError> {
    let files = hash_directory(&dir, hash_blake3).await?;
    let state = DirectoryState {
        files: Arc::new(files),
    };

    Ok(Router::new()
        .route(
            RASL_ROUTE,
            get(handle_directory).head(handle_directory_head),
        )
        .with_state(state))
}

async fn handle_directory(
    State(state): State<DirectoryState>,
    Path(cid_str): Path<String>,
) -> Response {
    match state.files.get(&cid_str) {
        Some(path) => match tokio::fs::read(path).await {
            Ok(data) => {
                (StatusCode::OK, [(header::CONTENT_TYPE, OCTET_STREAM)], data).into_response()
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn handle_directory_head(
    State(state): State<DirectoryState>,
    Path(cid_str): Path<String>,
) -> Response {
    match state.files.get(&cid_str) {
        Some(path) => match tokio::fs::metadata(path).await {
            Ok(meta) => {
                let len = meta.len().to_string();
                let mut response = Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap();
                response
                    .headers_mut()
                    .insert(header::CONTENT_TYPE, HeaderValue::from_static(OCTET_STREAM));
                if let Ok(value) = HeaderValue::from_str(&len) {
                    response.headers_mut().insert(header::CONTENT_LENGTH, value);
                }
                response
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Create an axum router that serves files from a directory where filenames
/// are CID strings.
///
/// Unlike [`directory_handler`], this handler does not pre-hash files.
/// Instead, it expects the directory to already contain files whose names
/// are their CID strings (e.g., `bafkreifn5yxi...`). This is a simpler
/// alternative for pre-organized content stores.
///
/// # Errors
///
/// Returns `RaslError::DirectoryError` if the directory path is invalid.
///
/// # Example
///
/// ```rust,no_run
/// use std::path::PathBuf;
/// use atproto_dasl::rasl::handler::cid_directory_handler;
///
/// let router = cid_directory_handler(PathBuf::from("/srv/cid-store"));
/// ```
pub fn cid_directory_handler(dir: PathBuf) -> Router {
    let state = Arc::new(dir);
    Router::new()
        .route(
            RASL_ROUTE,
            get(handle_cid_directory).head(handle_cid_directory_head),
        )
        .with_state(state)
}

async fn handle_cid_directory(
    State(dir): State<Arc<PathBuf>>,
    Path(cid_str): Path<String>,
) -> Response {
    // Validate that the CID string is a valid CID to prevent path traversal.
    if parse_cid_from_path(&cid_str).is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let file_path = dir.join(&cid_str);
    match tokio::fs::read(&file_path).await {
        Ok(data) => (StatusCode::OK, [(header::CONTENT_TYPE, OCTET_STREAM)], data).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn handle_cid_directory_head(
    State(dir): State<Arc<PathBuf>>,
    Path(cid_str): Path<String>,
) -> Response {
    if parse_cid_from_path(&cid_str).is_none() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let file_path = dir.join(&cid_str);
    match tokio::fs::metadata(&file_path).await {
        Ok(meta) => {
            let len = meta.len().to_string();
            let mut response = Response::builder()
                .status(StatusCode::OK)
                .body(Body::empty())
                .unwrap();
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static(OCTET_STREAM));
            if let Ok(value) = HeaderValue::from_str(&len) {
                response.headers_mut().insert(header::CONTENT_LENGTH, value);
            }
            response
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Hash all files in a directory, building CID-to-path mappings.
async fn hash_directory(
    dir: &PathBuf,
    hash_blake3: bool,
) -> Result<HashMap<String, PathBuf>, RaslError> {
    let mut files = HashMap::new();

    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| RaslError::DirectoryError {
            reason: format!("failed to read directory {}: {}", dir.display(), e),
        })?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| RaslError::DirectoryError {
            reason: format!("failed to read directory entry: {}", e),
        })?
    {
        let path = entry.path();

        // Skip directories and non-regular files
        match tokio::fs::metadata(&path).await {
            Ok(m) if m.is_file() => {}
            _ => continue,
        };

        let data = tokio::fs::read(&path)
            .await
            .map_err(|e| RaslError::DirectoryError {
                reason: format!("failed to read file {}: {}", path.display(), e),
            })?;

        // SHA-256 CID
        let sha_cid = compute_raw_cid(&data);
        files.insert(sha_cid.to_string(), path.clone());

        // Optionally BLAKE3 CID
        if hash_blake3 {
            let blake_cid = compute_raw_cid_blake3(&data);
            files.insert(blake_cid.to_string(), path);
        }
    }

    Ok(files)
}

/// Parse a CID string from a URL path segment.
fn parse_cid_from_path(cid_str: &str) -> Option<CidCore> {
    use std::str::FromStr;
    crate::cid::Cid::from_str(cid_str)
        .ok()
        .map(|c| c.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rasl_route_pattern() {
        assert!(RASL_ROUTE.starts_with("/.well-known/rasl/"));
    }

    #[test]
    fn test_parse_cid_from_path_valid() {
        let data = b"test data";
        let cid = compute_raw_cid(data);
        let cid_str = cid.to_string();
        let parsed = parse_cid_from_path(&cid_str);
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap(), cid);
    }

    #[test]
    fn test_parse_cid_from_path_invalid() {
        assert!(parse_cid_from_path("not-a-valid-cid").is_none());
    }

    #[tokio::test]
    async fn test_hash_directory_nonexistent() {
        let result = hash_directory(&PathBuf::from("/nonexistent/path"), false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_hash_directory_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        tokio::fs::write(&file_path, b"hello world").await.unwrap();

        let files = hash_directory(&dir.path().to_path_buf(), false)
            .await
            .unwrap();
        assert_eq!(files.len(), 1);

        // Verify the CID matches
        let expected_cid = compute_raw_cid(b"hello world");
        assert!(files.contains_key(&expected_cid.to_string()));
    }

    #[tokio::test]
    async fn test_hash_directory_with_blake3() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        tokio::fs::write(&file_path, b"hello world").await.unwrap();

        let files = hash_directory(&dir.path().to_path_buf(), true)
            .await
            .unwrap();
        // SHA-256 + BLAKE3 = 2 entries for one file
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_redirect_handler_creates_router() {
        let redirects = HashMap::new();
        let _router = redirect_handler(redirects);
    }

    #[test]
    fn test_func_handler_creates_router() {
        let _router = func_handler(|_cid: CidCore| async move { Some(vec![1, 2, 3]) });
    }

    #[test]
    fn test_cid_directory_handler_creates_router() {
        let _router = cid_directory_handler(PathBuf::from("/tmp/cid-store"));
    }

    #[tokio::test]
    async fn test_cid_directory_handler_serves_file() {
        let dir = tempfile::tempdir().unwrap();
        let data = b"hello cid world";
        let cid = compute_raw_cid(data);
        let cid_str = cid.to_string();

        // Write file named by its CID
        let file_path = dir.path().join(&cid_str);
        tokio::fs::write(&file_path, data).await.unwrap();

        // Verify the file exists with the right name
        assert!(tokio::fs::metadata(&file_path).await.is_ok());
    }
}
