//! PLC state management and DID document conversion.
//!
//! `PlcState` represents the internal state of a did:plc document as stored
//! in the PLC directory. It can be converted to and from the existing
//! `model::Document` type for W3C DID document compatibility.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::errors::PLCDIDError;
use crate::model;

/// Maximum number of verification methods allowed.
pub const MAX_VERIFICATION_METHODS: usize = 10;

/// Internal PLC state format.
///
/// Represents the internal state of a did:plc document as stored
/// in the PLC directory, with rotation keys, verification methods,
/// also-known-as URIs, and service endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlcState {
    /// Rotation keys (1-5 did:key strings).
    #[serde(rename = "rotationKeys")]
    pub rotation_keys: Vec<String>,

    /// Verification methods (max 10 entries).
    #[serde(rename = "verificationMethods")]
    pub verification_methods: HashMap<String, String>,

    /// Also-known-as URIs.
    #[serde(rename = "alsoKnownAs")]
    pub also_known_as: Vec<String>,

    /// Service endpoints.
    pub services: HashMap<String, ServiceEndpoint>,
}

impl PlcState {
    /// Create a new empty PLC state.
    pub fn new() -> Self {
        Self {
            rotation_keys: Vec::new(),
            verification_methods: HashMap::new(),
            also_known_as: Vec::new(),
            services: HashMap::new(),
        }
    }

    /// Validate this PLC state according to the specification.
    ///
    /// Checks that rotation keys count is 1-5 with no duplicates,
    /// verification methods do not exceed 10, and all keys are in
    /// did:key format.
    pub fn validate(&self) -> Result<(), PLCDIDError> {
        if self.rotation_keys.is_empty() {
            return Err(PLCDIDError::InvalidRotationKeys {
                details: "At least one rotation key is required".to_string(),
            });
        }

        if self.rotation_keys.len() > 5 {
            return Err(PLCDIDError::TooManyEntries {
                field: "rotation_keys".to_string(),
                max: 5,
                actual: self.rotation_keys.len(),
            });
        }

        // Check for duplicate rotation keys
        let mut seen = std::collections::HashSet::new();
        for key in &self.rotation_keys {
            if !seen.insert(key) {
                return Err(PLCDIDError::DuplicateEntry {
                    field: "rotation_keys".to_string(),
                    value: key.clone(),
                });
            }
        }

        // Validate all rotation keys are valid did:key format
        for key in &self.rotation_keys {
            if !key.starts_with("did:key:") {
                return Err(PLCDIDError::InvalidRotationKeys {
                    details: format!("Rotation key must be in did:key format: {}", key),
                });
            }
        }

        // Validate verification methods (max 10)
        if self.verification_methods.len() > MAX_VERIFICATION_METHODS {
            return Err(PLCDIDError::TooManyEntries {
                field: "verification_methods".to_string(),
                max: MAX_VERIFICATION_METHODS,
                actual: self.verification_methods.len(),
            });
        }

        // Validate all verification methods are valid did:key format
        for (name, key) in &self.verification_methods {
            if !key.starts_with("did:key:") {
                return Err(PLCDIDError::InvalidVerificationMethods {
                    details: format!(
                        "Verification method '{}' must be in did:key format: {}",
                        name, key
                    ),
                });
            }
        }

        Ok(())
    }

    /// Convert this PLC state to a `model::Document`.
    ///
    /// Maps rotation keys, verification methods, also-known-as URIs, and
    /// services to the existing W3C DID document format.
    pub fn to_document(&self, did: &str) -> model::Document {
        let mut verification_methods = Vec::new();

        for (id, controller) in &self.verification_methods {
            verification_methods.push(model::VerificationMethod::Multikey {
                id: format!("{}#{}", did, id),
                controller: did.to_string(),
                public_key_multibase: controller.clone(),
                extra: HashMap::new(),
            });
        }

        let services: Vec<model::Service> = self
            .services
            .iter()
            .map(|(id, endpoint)| model::Service {
                id: format!("{}#{}", did, id),
                r#type: endpoint.service_type.clone(),
                service_endpoint: endpoint.endpoint.clone(),
                extra: HashMap::new(),
            })
            .collect();

        model::Document {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/multikey/v1".to_string(),
            ],
            id: did.to_string(),
            also_known_as: self.also_known_as.clone(),
            verification_method: verification_methods,
            service: services,
            extra: HashMap::new(),
        }
    }

    /// Convert a `model::Document` back to PLC state.
    ///
    /// Note: rotation keys are not stored in the W3C DID document format,
    /// so the resulting state will have an empty `rotation_keys` vector.
    pub fn from_document(doc: &model::Document) -> Result<PlcState, PLCDIDError> {
        let mut verification_methods = HashMap::new();

        for vm in &doc.verification_method {
            match vm {
                model::VerificationMethod::Multikey {
                    id,
                    public_key_multibase,
                    ..
                } => {
                    let fragment = id
                        .rsplit('#')
                        .next()
                        .ok_or_else(|| PLCDIDError::InvalidVerificationMethods {
                            details: format!("Invalid verification method ID: {}", id),
                        })?
                        .to_string();

                    verification_methods.insert(fragment, public_key_multibase.clone());
                }
                model::VerificationMethod::Other { .. } => {
                    // Skip unsupported verification method types
                }
            }
        }

        let mut services = HashMap::new();
        for svc in &doc.service {
            let fragment = svc
                .id
                .rsplit('#')
                .next()
                .ok_or_else(|| PLCDIDError::InvalidService {
                    details: format!("Invalid service ID: {}", svc.id),
                })?
                .to_string();

            services.insert(
                fragment,
                ServiceEndpoint {
                    service_type: svc.r#type.clone(),
                    endpoint: svc.service_endpoint.clone(),
                },
            );
        }

        Ok(PlcState {
            rotation_keys: Vec::new(),
            verification_methods,
            also_known_as: doc.also_known_as.clone(),
            services,
        })
    }
}

impl Default for PlcState {
    fn default() -> Self {
        Self::new()
    }
}

/// Service endpoint definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service type (e.g., "AtprotoPersonalDataServer").
    #[serde(rename = "type")]
    pub service_type: String,

    /// Service endpoint URL.
    pub endpoint: String,
}

impl ServiceEndpoint {
    /// Create a new service endpoint.
    pub fn new(service_type: String, endpoint: String) -> Self {
        Self {
            service_type,
            endpoint,
        }
    }

    /// Validate this service endpoint.
    pub fn validate(&self) -> Result<(), PLCDIDError> {
        if self.service_type.is_empty() {
            return Err(PLCDIDError::InvalidService {
                details: "Service type cannot be empty".to_string(),
            });
        }

        if self.endpoint.is_empty() {
            return Err(PLCDIDError::InvalidService {
                details: "Service endpoint cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plc_state_validation() {
        let mut state = PlcState::new();

        // Empty state should fail (no rotation keys)
        assert!(state.validate().is_err());

        // Add a rotation key
        state
            .rotation_keys
            .push("did:key:zQ3shZc2QzApp2oymGvQbzP8eKheVshBHbU4ZYjeXqwSKEn6N".to_string());
        assert!(state.validate().is_ok());

        // Add duplicate rotation key
        state
            .rotation_keys
            .push("did:key:zQ3shZc2QzApp2oymGvQbzP8eKheVshBHbU4ZYjeXqwSKEn6N".to_string());
        assert!(state.validate().is_err());
    }

    #[test]
    fn test_service_endpoint() {
        let endpoint = ServiceEndpoint::new(
            "AtprotoPersonalDataServer".to_string(),
            "https://pds.example.com".to_string(),
        );
        assert!(endpoint.validate().is_ok());

        let empty_type = ServiceEndpoint::new(String::new(), "https://example.com".to_string());
        assert!(empty_type.validate().is_err());

        let empty_endpoint =
            ServiceEndpoint::new("AtprotoPersonalDataServer".to_string(), String::new());
        assert!(empty_endpoint.validate().is_err());
    }

    #[test]
    fn test_to_document() {
        let mut state = PlcState::new();
        state
            .rotation_keys
            .push("did:key:zQ3shZc2QzApp2oymGvQbzP8eKheVshBHbU4ZYjeXqwSKEn6N".to_string());
        state.verification_methods.insert(
            "atproto".to_string(),
            "did:key:zDnaerDaTF5BXEavCrfRZEk316dpbLsfPDZ3WJ5hRTPFU2169".to_string(),
        );
        state
            .also_known_as
            .push("at://alice.example.com".to_string());
        state.services.insert(
            "atproto_pds".to_string(),
            ServiceEndpoint::new(
                "AtprotoPersonalDataServer".to_string(),
                "https://pds.example.com".to_string(),
            ),
        );

        let doc = state.to_document("did:plc:ewvi7nxzyoun6zhxrhs64oiz");
        assert_eq!(doc.id, "did:plc:ewvi7nxzyoun6zhxrhs64oiz");
        assert_eq!(doc.verification_method.len(), 1);
        assert_eq!(doc.service.len(), 1);
        assert_eq!(doc.also_known_as.len(), 1);
    }

    #[test]
    fn test_from_document_roundtrip() {
        let mut state = PlcState::new();
        state.verification_methods.insert(
            "atproto".to_string(),
            "did:key:zDnaerDaTF5BXEavCrfRZEk316dpbLsfPDZ3WJ5hRTPFU2169".to_string(),
        );
        state.services.insert(
            "atproto_pds".to_string(),
            ServiceEndpoint::new(
                "AtprotoPersonalDataServer".to_string(),
                "https://pds.example.com".to_string(),
            ),
        );
        state
            .also_known_as
            .push("at://alice.example.com".to_string());

        let doc = state.to_document("did:plc:ewvi7nxzyoun6zhxrhs64oiz");
        let recovered = PlcState::from_document(&doc).unwrap();

        // Rotation keys are not preserved in W3C DID docs
        assert!(recovered.rotation_keys.is_empty());
        assert_eq!(recovered.verification_methods, state.verification_methods);
        assert_eq!(recovered.services, state.services);
        assert_eq!(recovered.also_known_as, state.also_known_as);
    }
}
