//! MASL (Metadata for Arbitrary Structures & Links).
//!
//! A type of CBOR metadata document designed to work well with content-addressed
//! and decentralized systems, enabling fully self-contained, self-certified
//! content distribution.
//!
//! MASL documents come in two modes:
//! - **Single mode**: Describes a single resource with HTTP-like headers
//! - **Bundle mode**: Describes multiple path-keyed resources (e.g., a web app)
//!
//! See <https://dasl.ing/> for the specification.

use crate::cid::Cid;
use crate::errors::MaslError;
use crate::value::Ipld;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// AT Protocol type identifier for MASL documents.
pub const MASL_TYPE: &str = "ing.dasl.masl";

/// A MASL metadata document.
///
/// In single mode, the document describes one resource with optional HTTP
/// headers and metadata. In bundle mode, it contains a map of path-keyed
/// resources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// The resource described by this document (single mode).
    #[serde(flatten)]
    pub resource: Resource,

    /// Bundle of path-keyed resources (bundle mode).
    /// When present, the document is in bundle mode.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resources: BTreeMap<String, Resource>,

    /// CID of the previous version of this document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev: Option<Cid>,

    /// CAR version compatibility (must be 0 or 1 if present).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,

    /// CAR root CIDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roots: Vec<Cid>,

    /// AT Protocol type identifier.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "$type")]
    pub doc_type: Option<String>,
}

impl Document {
    /// Returns true if this document is in bundle mode.
    pub fn is_bundle(&self) -> bool {
        !self.resources.is_empty()
    }

    /// Validate the document structure.
    ///
    /// # Errors
    ///
    /// Returns `MaslError` if the document is structurally invalid.
    pub fn validate(&self) -> Result<(), MaslError> {
        // Validate CAR version if present
        if let Some(version) = self.version
            && version > 1
        {
            return Err(MaslError::InvalidCarFormat {
                reason: format!("CAR version must be 0 or 1, got {}", version),
            });
        }

        // Validate AT Protocol type
        if let Some(ref doc_type) = self.doc_type
            && doc_type != MASL_TYPE
        {
            return Err(MaslError::InvalidType {
                reason: format!("expected '{}' or empty, got '{}'", MASL_TYPE, doc_type),
            });
        }

        if self.is_bundle() {
            self.validate_bundle()?;
        } else {
            self.validate_single()?;
        }

        Ok(())
    }

    /// Validate bundle mode constraints.
    fn validate_bundle(&self) -> Result<(), MaslError> {
        for (path, resource) in &self.resources {
            // All paths must start with '/'
            if !path.starts_with('/') {
                return Err(MaslError::InvalidPath {
                    reason: format!("bundle path must start with '/': {}", path),
                });
            }

            // All resources must have a defined source CID
            if resource.source.is_none() {
                return Err(MaslError::MissingField {
                    field: format!("resources[{}].source", path),
                });
            }

            // Validate sourcemap references existing path
            if let Some(ref sourcemap) = resource.sourcemap
                && !sourcemap.is_empty()
                && !self.resources.contains_key(sourcemap)
            {
                return Err(MaslError::InvalidReference {
                    path: sourcemap.clone(),
                    field: format!("resources[{}].sourcemap", path),
                });
            }

            // Validate speculation_rules references existing path
            if let Some(ref rules) = resource.speculation_rules
                && !rules.is_empty()
                && !self.resources.contains_key(rules)
            {
                return Err(MaslError::InvalidReference {
                    path: rules.clone(),
                    field: format!("resources[{}].speculation_rules", path),
                });
            }

            // Validate icon src references existing paths
            for icon in &resource.icons {
                if !icon.src.is_empty() && !self.resources.contains_key(&icon.src) {
                    return Err(MaslError::InvalidReference {
                        path: icon.src.clone(),
                        field: format!("resources[{}].icons[].src", path),
                    });
                }
            }

            // Validate screenshot src references existing paths
            for screenshot in &resource.screenshots {
                if !screenshot.src.is_empty() && !self.resources.contains_key(&screenshot.src) {
                    return Err(MaslError::InvalidReference {
                        path: screenshot.src.clone(),
                        field: format!("resources[{}].screenshots[].src", path),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate single mode constraints.
    fn validate_single(&self) -> Result<(), MaslError> {
        // In single mode, icon src must be empty
        for icon in &self.resource.icons {
            if !icon.src.is_empty() {
                return Err(MaslError::InvalidResource {
                    reason: format!("single mode icon src must be empty, got '{}'", icon.src),
                });
            }
        }

        // In single mode, screenshot src must be empty
        for screenshot in &self.resource.screenshots {
            if !screenshot.src.is_empty() {
                return Err(MaslError::InvalidResource {
                    reason: format!(
                        "single mode screenshot src must be empty, got '{}'",
                        screenshot.src
                    ),
                });
            }
        }

        Ok(())
    }
}

/// A single resource with HTTP-like headers and metadata.
///
/// Resource fields are organized into three categories:
/// - **Content identification**: `source`, `name`
/// - **HTTP response headers**: `content_type`, `content_encoding`, etc.
/// - **Web app manifest fields**: `background_color`, `categories`, `icons`, etc.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Resource {
    // --- Content identification ---
    /// CID of the resource content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Cid>,

    /// Resource name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    // --- HTTP response headers ---
    /// Content type (MIME type).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "content-type"
    )]
    pub content_type: Option<String>,

    /// Content disposition header value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "content-disposition"
    )]
    pub content_disposition: Option<String>,

    /// Content encoding (e.g., "gzip", "br").
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "content-encoding"
    )]
    pub content_encoding: Option<String>,

    /// Content language header value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "content-language"
    )]
    pub content_language: Option<String>,

    /// Content Security Policy header value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "content-security-policy"
    )]
    pub csp: Option<String>,

    /// HTTP Link header value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,

    /// Permissions policy header value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "permissions-policy"
    )]
    pub permissions_policy: Option<String>,

    /// Referrer policy header value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "referrer-policy"
    )]
    pub referrer_policy: Option<String>,

    /// Service worker allowed scope.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "service-worker-allowed"
    )]
    pub service_worker_allowed: Option<String>,

    /// Content location header value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "content-location"
    )]
    pub content_location: Option<String>,

    /// Source map path reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sourcemap: Option<String>,

    /// Speculation rules path reference.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "speculation-rules"
    )]
    pub speculation_rules: Option<String>,

    /// Supports loading mode header value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "supports-loading-mode"
    )]
    pub supports_loading_mode: Option<String>,

    /// X-Content-Type-Options header value.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "x-content-type-options"
    )]
    pub x_content_type_options: Option<String>,

    // --- Web app manifest fields ---
    /// Background color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,

    /// App categories.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,

    /// App description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// App icons.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icons: Vec<Icon>,

    /// Manifest ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// App screenshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<Screenshot>,

    /// Short app name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,

    /// Theme color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,

    /// Catch-all for unknown/custom fields.
    ///
    /// Captures any fields not explicitly defined in the struct, enabling
    /// forward compatibility with newer MASL document versions.
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, Ipld>,
}

/// A web app icon reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Icon {
    /// Path reference to the icon resource (bundle mode) or empty (single mode).
    #[serde(default)]
    pub src: String,

    /// Icon sizes (e.g., "192x192", "512x512").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,

    /// Icon purpose (e.g., "any", "maskable").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// A web app screenshot reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Screenshot {
    /// Path reference to the screenshot resource (bundle mode) or empty (single mode).
    #[serde(default)]
    pub src: String,

    /// Screenshot sizes (e.g., "1080x1920").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,

    /// Screenshot label for accessibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Form factor hint (e.g., "wide", "narrow").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_factor: Option<String>,

    /// Platform hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_mode_document() {
        let doc = Document {
            resource: Resource {
                content_type: Some("text/html".into()),
                ..Default::default()
            },
            resources: BTreeMap::new(),
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(!doc.is_bundle());
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_bundle_mode_document() {
        let mut resources = BTreeMap::new();
        resources.insert(
            "/index.html".to_string(),
            Resource {
                content_type: Some("text/html".into()),
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"html"))),
                ..Default::default()
            },
        );
        resources.insert(
            "/style.css".to_string(),
            Resource {
                content_type: Some("text/css".into()),
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"css"))),
                ..Default::default()
            },
        );

        let doc = Document {
            resource: Resource::default(),
            resources,
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.is_bundle());
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_bundle_invalid_path() {
        let mut resources = BTreeMap::new();
        resources.insert(
            "no-leading-slash".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"data"))),
                ..Default::default()
            },
        );

        let doc = Document {
            resource: Resource::default(),
            resources,
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_invalid_car_version() {
        let doc = Document {
            resource: Resource::default(),
            resources: BTreeMap::new(),
            prev: None,
            version: Some(2),
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_valid_car_version() {
        let doc = Document {
            resource: Resource::default(),
            resources: BTreeMap::new(),
            prev: None,
            version: Some(1),
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_car_version_zero() {
        let doc = Document {
            resource: Resource::default(),
            resources: BTreeMap::new(),
            prev: None,
            version: Some(0),
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_valid_doc_type() {
        let doc = Document {
            resource: Resource::default(),
            resources: BTreeMap::new(),
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: Some(MASL_TYPE.to_string()),
        };
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_invalid_doc_type() {
        let doc = Document {
            resource: Resource::default(),
            resources: BTreeMap::new(),
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: Some("com.example.wrong".to_string()),
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_bundle_missing_source() {
        let mut resources = BTreeMap::new();
        resources.insert(
            "/missing-source".to_string(),
            Resource {
                content_type: Some("text/plain".into()),
                source: None,
                ..Default::default()
            },
        );

        let doc = Document {
            resource: Resource::default(),
            resources,
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_bundle_valid_sourcemap_reference() {
        let mut resources = BTreeMap::new();
        resources.insert(
            "/app.js".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"js"))),
                sourcemap: Some("/app.js.map".to_string()),
                ..Default::default()
            },
        );
        resources.insert(
            "/app.js.map".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"sourcemap"))),
                ..Default::default()
            },
        );

        let doc = Document {
            resource: Resource::default(),
            resources,
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_bundle_invalid_sourcemap_reference() {
        let mut resources = BTreeMap::new();
        resources.insert(
            "/app.js".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"js"))),
                sourcemap: Some("/nonexistent.map".to_string()),
                ..Default::default()
            },
        );

        let doc = Document {
            resource: Resource::default(),
            resources,
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_bundle_valid_icon_reference() {
        let mut resources = BTreeMap::new();
        resources.insert(
            "/index.html".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"html"))),
                icons: vec![Icon {
                    src: "/icon.png".to_string(),
                    sizes: Some("192x192".to_string()),
                    purpose: None,
                }],
                ..Default::default()
            },
        );
        resources.insert(
            "/icon.png".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"icon"))),
                content_type: Some("image/png".to_string()),
                ..Default::default()
            },
        );

        let doc = Document {
            resource: Resource::default(),
            resources,
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_bundle_invalid_icon_reference() {
        let mut resources = BTreeMap::new();
        resources.insert(
            "/index.html".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"html"))),
                icons: vec![Icon {
                    src: "/nonexistent.png".to_string(),
                    sizes: None,
                    purpose: None,
                }],
                ..Default::default()
            },
        );

        let doc = Document {
            resource: Resource::default(),
            resources,
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_single_mode_icon_src_must_be_empty() {
        let doc = Document {
            resource: Resource {
                icons: vec![Icon {
                    src: "/icon.png".to_string(),
                    sizes: None,
                    purpose: None,
                }],
                ..Default::default()
            },
            resources: BTreeMap::new(),
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_single_mode_screenshot_src_must_be_empty() {
        let doc = Document {
            resource: Resource {
                screenshots: vec![Screenshot {
                    src: "/screenshot.png".to_string(),
                    sizes: None,
                    label: None,
                    form_factor: None,
                    platform: None,
                }],
                ..Default::default()
            },
            resources: BTreeMap::new(),
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_single_mode_empty_icon_src_is_valid() {
        let doc = Document {
            resource: Resource {
                icons: vec![Icon {
                    src: String::new(),
                    sizes: Some("192x192".to_string()),
                    purpose: None,
                }],
                ..Default::default()
            },
            resources: BTreeMap::new(),
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_resource_all_http_headers() {
        let resource = Resource {
            content_type: Some("text/html".into()),
            content_disposition: Some("inline".into()),
            content_encoding: Some("gzip".into()),
            content_language: Some("en-US".into()),
            csp: Some("default-src 'self'".into()),
            link: Some("</style.css>; rel=preload; as=style".into()),
            permissions_policy: Some("camera=(), microphone=()".into()),
            referrer_policy: Some("no-referrer".into()),
            service_worker_allowed: Some("/".into()),
            supports_loading_mode: Some("fenced-frame".into()),
            x_content_type_options: Some("nosniff".into()),
            ..Default::default()
        };
        assert!(resource.content_type.is_some());
        assert!(resource.content_disposition.is_some());
        assert!(resource.x_content_type_options.is_some());
    }

    #[test]
    fn test_resource_manifest_fields() {
        let resource = Resource {
            name: Some("My App".into()),
            short_name: Some("App".into()),
            description: Some("A great app".into()),
            background_color: Some("#ffffff".into()),
            theme_color: Some("#000000".into()),
            categories: vec!["productivity".into(), "utilities".into()],
            id: Some("/".into()),
            icons: vec![Icon {
                src: String::new(),
                sizes: Some("512x512".to_string()),
                purpose: Some("any maskable".to_string()),
            }],
            screenshots: vec![Screenshot {
                src: String::new(),
                sizes: Some("1080x1920".to_string()),
                label: Some("Home screen".to_string()),
                form_factor: Some("narrow".to_string()),
                platform: Some("android".to_string()),
            }],
            ..Default::default()
        };
        assert_eq!(resource.categories.len(), 2);
        assert!(resource.description.is_some());
    }

    #[test]
    fn test_bundle_speculation_rules_reference() {
        let mut resources = BTreeMap::new();
        resources.insert(
            "/page.html".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"page"))),
                speculation_rules: Some("/rules.json".to_string()),
                ..Default::default()
            },
        );
        resources.insert(
            "/rules.json".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"rules"))),
                ..Default::default()
            },
        );

        let doc = Document {
            resource: Resource::default(),
            resources,
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_resource_content_location() {
        let resource = Resource {
            content_location: Some("/alternate".into()),
            ..Default::default()
        };
        assert_eq!(resource.content_location.as_deref(), Some("/alternate"));
    }

    #[test]
    fn test_resource_attributes_catch_all() {
        use crate::value::Ipld;

        let mut resource = Resource::default();
        resource
            .attributes
            .insert("custom-field".to_string(), Ipld::String("value".into()));
        resource
            .attributes
            .insert("x-custom-int".to_string(), Ipld::Integer(42));

        assert_eq!(resource.attributes.len(), 2);
        assert_eq!(
            resource.attributes.get("custom-field"),
            Some(&Ipld::String("value".into()))
        );
        assert_eq!(
            resource.attributes.get("x-custom-int"),
            Some(&Ipld::Integer(42))
        );
    }

    #[test]
    fn test_bundle_invalid_speculation_rules_reference() {
        let mut resources = BTreeMap::new();
        resources.insert(
            "/page.html".to_string(),
            Resource {
                source: Some(crate::cid::Cid::new(crate::cid::compute_cid(b"page"))),
                speculation_rules: Some("/nonexistent.json".to_string()),
                ..Default::default()
            },
        );

        let doc = Document {
            resource: Resource::default(),
            resources,
            prev: None,
            version: None,
            roots: Vec::new(),
            doc_type: None,
        };
        assert!(doc.validate().is_err());
    }
}
