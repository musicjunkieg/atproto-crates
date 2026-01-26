//! Example XRPC service with DID:web identity and JWT authentication.

use anyhow::Result;
use async_trait::async_trait;
use atproto_identity::resolve::SharedIdentityResolver;
use atproto_identity::{
    config::{CertificateBundles, DnsNameservers, default_env, optional_env, require_env, version},
    key::{KeyData, KeyResolver, identify_key, to_public},
    resolve::{HickoryDnsResolver, IdentityResolver, InnerIdentityResolver},
};
use atproto_xrpcs::authorization::Authorization;
use axum::{
    Json, Router,
    extract::{FromRef, Query, State},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use clap::Parser;
use http::{HeaderMap, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct SimpleKeyResolver {
    keys: HashMap<String, KeyData>,
}

impl Default for SimpleKeyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleKeyResolver {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }
}

#[async_trait]
impl KeyResolver for SimpleKeyResolver {
    async fn resolve(&self, key: &str) -> anyhow::Result<KeyData> {
        if let Some(key_data) = self.keys.get(key) {
            Ok(key_data.clone())
        } else {
            identify_key(key).map_err(Into::into)
        }
    }
}

#[derive(Clone)]
pub struct ServiceDocument(pub serde_json::Value);

#[derive(Clone)]
pub struct ServiceDID(pub String);

pub struct InnerWebContext {
    pub http_client: reqwest::Client,
    pub key_resolver: Arc<dyn KeyResolver>,
    pub service_document: ServiceDocument,
    pub service_did: ServiceDID,
    pub identity_resolver: Arc<dyn IdentityResolver>,
}

#[derive(Clone, FromRef)]
pub struct WebContext(pub Arc<InnerWebContext>);

impl Deref for WebContext {
    type Target = InnerWebContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRef<WebContext> for reqwest::Client {
    fn from_ref(context: &WebContext) -> Self {
        context.0.http_client.clone()
    }
}

impl FromRef<WebContext> for ServiceDocument {
    fn from_ref(context: &WebContext) -> Self {
        context.0.service_document.clone()
    }
}

impl FromRef<WebContext> for ServiceDID {
    fn from_ref(context: &WebContext) -> Self {
        context.0.service_did.clone()
    }
}

impl FromRef<WebContext> for Arc<dyn KeyResolver> {
    fn from_ref(context: &WebContext) -> Self {
        context.0.key_resolver.clone()
    }
}

impl FromRef<WebContext> for Arc<dyn IdentityResolver> {
    fn from_ref(context: &WebContext) -> Self {
        context.0.identity_resolver.clone()
    }
}

/// AT Protocol XRPC Hello World Service
#[derive(Parser)]
#[command(
    name = "atproto-xrpcs-helloworld",
    version,
    about = "AT Protocol XRPC Hello World demonstration service",
    long_about = "
A demonstration XRPC service implementation showcasing the AT Protocol ecosystem.
This service provides a simple \"Hello, World!\" endpoint that supports both
authenticated and unauthenticated requests.

FEATURES:
  - AT Protocol identity resolution and DID document management
  - XRPC service endpoint with optional authentication
  - DID:web identity publishing via .well-known endpoints
  - JWT-based request authentication using AT Protocol standards

ENVIRONMENT VARIABLES:
  SERVICE_KEY          Private key for service identity (required)
  EXTERNAL_BASE        External hostname for service endpoints (required)
  PORT                HTTP server port (default: 8080)
  PLC_HOSTNAME        PLC directory hostname (default: plc.directory)
  USER_AGENT          HTTP User-Agent header (auto-generated)
  DNS_NAMESERVERS     Custom DNS nameservers (optional)
  CERTIFICATE_BUNDLES Additional CA certificates (optional)

ENDPOINTS:
  GET /                           HTML index page
  GET /.well-known/did.json       DID document (DID:web)
  GET /.well-known/atproto-did    AT Protocol DID identifier
  GET /xrpc/.../Hello            Hello World XRPC endpoint
"
)]
struct Args {}

#[tokio::main]
async fn main() -> Result<()> {
    let _args = Args::parse();

    let plc_hostname = default_env("PLC_HOSTNAME", "plc.directory");
    let certificate_bundles: CertificateBundles = optional_env("CERTIFICATE_BUNDLES").try_into()?;
    let default_user_agent = format!(
        "atproto-identity-rs ({}; +https://tangled.sh/@smokesignal.events/atproto-identity-rs)",
        version()?
    );
    let user_agent = default_env("USER_AGENT", &default_user_agent);
    let dns_nameservers: DnsNameservers = optional_env("DNS_NAMESERVERS").try_into()?;

    let mut client_builder = reqwest::Client::builder();
    for ca_certificate in certificate_bundles.as_ref() {
        let cert = std::fs::read(ca_certificate)?;
        let cert = reqwest::Certificate::from_pem(&cert)?;
        client_builder = client_builder.add_root_certificate(cert);
    }

    client_builder = client_builder.user_agent(user_agent);
    let http_client = client_builder.build()?;

    let dns_resolver = HickoryDnsResolver::create_resolver(dns_nameservers.as_ref());

    let external_base = require_env("EXTERNAL_BASE")?;
    let port = default_env("PORT", "8080");
    let service_did = format!("did:web:{}", external_base);

    let private_service_key = require_env("SERVICE_KEY")?;
    let private_service_key_data = identify_key(&private_service_key)?;
    let public_service_key_data = to_public(&private_service_key_data)?;
    let public_service_key = public_service_key_data.to_string();

    let signing_key_storage = HashMap::from_iter(vec![(
        public_service_key.clone(),
        private_service_key_data.clone(),
    )]);

    let service_document = ServiceDocument(json!({
            "@context": vec!["https://www.w3.org/ns/did/v1","https://w3id.org/security/multikey/v1"],
            "id": service_did,
            "verificationMethod":[{
                "id": format!("{service_did}#atproto"),
                "type":"Multikey",
                "controller": service_did,
                "publicKeyMultibase": public_service_key
            }],
            "service":[{
                "id":"#helloworld",
                "type":"HelloWorldService",
                "serviceEndpoint":format!("https://{external_base}")
            }]
        }
    ));

    let service_did = ServiceDID(service_did);

    let identity_resolver = Arc::new(SharedIdentityResolver(Arc::new(InnerIdentityResolver {
        dns_resolver: Arc::new(dns_resolver),
        http_client: http_client.clone(),
        plc_hostname,
    })));

    let web_context = WebContext(Arc::new(InnerWebContext {
        http_client: http_client.clone(),
        key_resolver: Arc::new(SimpleKeyResolver {
            keys: signing_key_storage,
        }),
        service_document,
        service_did,
        identity_resolver,
    }));

    let router = Router::new()
        .route("/", get(handle_index))
        .route("/.well-known/did.json", get(handle_wellknown_did_web))
        .route(
            "/.well-known/atproto-did",
            get(handle_wellknown_atproto_did),
        )
        .route(
            "/xrpc/garden.lexicon.ngerakines.helloworld.Hello",
            get(handle_xrpc_hello_world),
        )
        .with_state(web_context);

    let bind_address = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;

    // Start the web server in the background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            eprintln!("Server error: {}", e);
        }
    });

    println!(
        "XRPC Hello World service started on http://0.0.0.0:{}",
        port
    );

    // Keep the server running
    server_handle.await.unwrap();

    Ok(())
}

async fn handle_index() -> Html<&'static str> {
    Html("<html><body><h1>Hello, World!</h1></body></html>")
}

// /.well-known/did.json
async fn handle_wellknown_did_web(
    service_document: State<ServiceDocument>,
) -> Json<serde_json::Value> {
    Json(service_document.0.0)
}

// /.well-known/atproto-did
async fn handle_wellknown_atproto_did(service_did: State<ServiceDID>) -> Response {
    (StatusCode::OK, service_did.0.0).into_response()
}

#[derive(Deserialize)]
struct HelloParameters {
    subject: Option<String>,
}

// /xrpc/garden.lexicon.ngerakines.helloworld.Hello
async fn handle_xrpc_hello_world(
    parameters: Query<HelloParameters>,
    headers: HeaderMap,
    authorization: Option<Authorization>,
) -> Json<serde_json::Value> {
    println!("headers {headers:?}");
    let subject = parameters.subject.as_deref().unwrap_or("World");
    let message = if authorization.is_none() {
        format!("Hello, {subject}!")
    } else {
        format!("Hello, authenticated {subject}!")
    };
    Json(json!({ "message": message}))
}
