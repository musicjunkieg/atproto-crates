//! Operation chain validation with fork resolution.
//!
//! Validates sequences of PLC operations ensuring correct chaining via CID
//! references, valid signatures, and proper state transitions. Supports
//! fork resolution using rotation key priority within a 72-hour recovery window.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

use crate::errors::PLCDIDError;
use crate::key;

use super::operations::Operation;
use super::state::{MAX_VERIFICATION_METHODS, PlcState, ServiceEndpoint};

/// Recovery window duration (72 hours).
const RECOVERY_WINDOW_HOURS: i64 = 72;

/// An operation with its server-assigned timestamp.
#[derive(Debug, Clone)]
pub struct OperationWithTimestamp {
    /// The operation itself.
    pub operation: Operation,
    /// Server-assigned timestamp when the operation was received.
    pub timestamp: DateTime<Utc>,
}

/// A fork point where multiple operations reference the same prev CID.
#[derive(Debug)]
struct ForkPoint {
    /// The prev CID that multiple operations reference.
    #[allow(dead_code)]
    prev_cid: String,
    /// Competing operations at this fork point, with their timestamps and signing key indices.
    operations: Vec<(Operation, DateTime<Utc>, usize)>,
}

impl ForkPoint {
    /// Create a new fork point.
    fn new(prev_cid: String) -> Self {
        Self {
            prev_cid,
            operations: Vec::new(),
        }
    }

    /// Add an operation to this fork point.
    fn add_operation(&mut self, operation: Operation, timestamp: DateTime<Utc>, key_index: usize) {
        self.operations.push((operation, timestamp, key_index));
    }

    /// Resolve this fork point and return the canonical operation.
    ///
    /// Resolution: sort by timestamp (first-received wins by default), then check
    /// if any later-received operation with higher priority (lower key index) can
    /// invalidate within the 72-hour recovery window.
    fn resolve(&self) -> Result<Operation, PLCDIDError> {
        if self.operations.is_empty() {
            return Err(PLCDIDError::ForkResolutionError {
                details: "Fork point has no operations".to_string(),
            });
        }

        if self.operations.len() == 1 {
            return Ok(self.operations[0].0.clone());
        }

        let mut sorted = self.operations.clone();
        sorted.sort_by_key(|(_, timestamp, _)| *timestamp);

        let (mut canonical_op, mut canonical_ts, mut canonical_key_idx) = sorted[0].clone();

        for (competing_op, competing_ts, competing_key_idx) in &sorted[1..] {
            if *competing_key_idx < canonical_key_idx {
                let time_diff = *competing_ts - canonical_ts;
                if time_diff <= Duration::hours(RECOVERY_WINDOW_HOURS)
                    && time_diff >= Duration::zero()
                {
                    canonical_op = competing_op.clone();
                    canonical_ts = *competing_ts;
                    canonical_key_idx = *competing_key_idx;
                }
            }
        }

        Ok(canonical_op)
    }
}

/// Builder for constructing the canonical chain from operations with timestamps.
struct CanonicalChainBuilder {
    operations: Vec<OperationWithTimestamp>,
}

impl CanonicalChainBuilder {
    /// Create a new builder.
    fn new(operations: Vec<OperationWithTimestamp>) -> Self {
        Self { operations }
    }

    /// Build the canonical chain by detecting and resolving forks.
    fn build(&self) -> Result<Vec<Operation>, PLCDIDError> {
        if self.operations.is_empty() {
            return Err(PLCDIDError::EmptyChain);
        }

        let mut operation_map: HashMap<String, (Operation, DateTime<Utc>)> = HashMap::new();
        for op_with_ts in &self.operations {
            let cid = op_with_ts.operation.cid()?;
            operation_map.insert(cid, (op_with_ts.operation.clone(), op_with_ts.timestamp));
        }

        let genesis = self
            .operations
            .iter()
            .find(|op| op.operation.is_genesis())
            .ok_or(PLCDIDError::FirstOperationNotGenesis)?;

        // Group operations by their prev CID
        let mut prev_to_operations: HashMap<String, Vec<(Operation, DateTime<Utc>)>> =
            HashMap::new();

        for op_with_ts in &self.operations {
            if let Some(prev_cid) = op_with_ts.operation.prev() {
                prev_to_operations
                    .entry(prev_cid.to_string())
                    .or_default()
                    .push((op_with_ts.operation.clone(), op_with_ts.timestamp));
            }
        }

        // Build fork points for any prev CID with multiple operations
        let mut fork_points: HashMap<String, ForkPoint> = HashMap::new();
        for (prev_cid, operations) in &prev_to_operations {
            if operations.len() > 1 {
                let mut fork_point = ForkPoint::new(prev_cid.clone());

                for (operation, timestamp) in operations {
                    let rotation_keys = if let Some((prev_op, _)) = operation_map.get(prev_cid) {
                        self.get_state_at_operation(prev_op)?
                    } else {
                        PlcState::new()
                    };

                    let key_index = self.find_signing_key_index(operation, &rotation_keys)?;
                    fork_point.add_operation(operation.clone(), *timestamp, key_index);
                }

                fork_points.insert(prev_cid.clone(), fork_point);
            }
        }

        // Build the canonical chain from genesis to tip
        let mut canonical_chain = vec![genesis.operation.clone()];
        let mut current_cid = genesis.operation.cid()?;

        loop {
            if let Some(fork_point) = fork_points.get(&current_cid) {
                let canonical_op = fork_point.resolve()?;
                let next_cid = canonical_op.cid()?;
                canonical_chain.push(canonical_op);
                current_cid = next_cid;
            } else if let Some(operations) = prev_to_operations.get(&current_cid) {
                if operations.len() == 1 {
                    let (operation, _) = &operations[0];
                    let next_cid = operation.cid()?;
                    canonical_chain.push(operation.clone());
                    current_cid = next_cid;
                } else {
                    return Err(PLCDIDError::ForkResolutionError {
                        details: "Unexpected multiple operations without fork point".to_string(),
                    });
                }
            } else {
                break;
            }
        }

        Ok(canonical_chain)
    }

    /// Find the index of the rotation key that signed this operation.
    fn find_signing_key_index(
        &self,
        operation: &Operation,
        state: &PlcState,
    ) -> Result<usize, PLCDIDError> {
        if state.rotation_keys.is_empty() {
            return Err(PLCDIDError::InvalidRotationKeys {
                details: "No rotation keys available for verification".to_string(),
            });
        }

        for (index, key_did) in state.rotation_keys.iter().enumerate() {
            let public_key_data =
                key::identify_key(key_did).map_err(|e| PLCDIDError::InvalidRotationKeys {
                    details: format!("Failed to parse rotation key: {}", e),
                })?;
            let public_key_data =
                key::to_public(&public_key_data).map_err(|e| PLCDIDError::InvalidRotationKeys {
                    details: format!("Failed to derive public key: {}", e),
                })?;

            if operation.verify(&[public_key_data]).is_ok() {
                return Ok(index);
            }
        }

        Err(PLCDIDError::SignatureVerificationFailed)
    }

    /// Get the state at a specific operation by replaying the chain.
    fn get_state_at_operation(
        &self,
        target_operation: &Operation,
    ) -> Result<PlcState, PLCDIDError> {
        let mut state = PlcState::new();
        let target_cid = target_operation.cid()?;

        for op_with_ts in &self.operations {
            let op = &op_with_ts.operation;

            match op {
                Operation::PlcOperation {
                    rotation_keys,
                    verification_methods,
                    also_known_as,
                    services,
                    ..
                } => {
                    state.rotation_keys = rotation_keys.clone();
                    state.verification_methods = verification_methods.clone();
                    state.also_known_as = also_known_as.clone();
                    state.services = services.clone();
                }
                Operation::PlcTombstone { .. } => {
                    state = PlcState::new();
                }
                Operation::LegacyCreate { .. } => {
                    return Err(PLCDIDError::InvalidOperationType {
                        details: "Legacy create operations not fully supported".to_string(),
                    });
                }
            }

            if op.cid()? == target_cid {
                break;
            }
        }

        Ok(state)
    }
}

/// Operation chain validator.
///
/// Validates operation chains for did:plc, ensuring correct CID linking,
/// valid signatures, and proper state transitions. Supports fork resolution
/// using rotation key priority within the 72-hour recovery window.
pub struct OperationChainValidator;

impl OperationChainValidator {
    /// Validate a complete operation chain and return the final state.
    ///
    /// Checks that the chain starts with a genesis operation, each operation
    /// correctly references the previous CID, and all signatures are valid
    /// against the current rotation keys.
    pub fn validate_chain(operations: &[Operation]) -> Result<PlcState, PLCDIDError> {
        if operations.is_empty() {
            return Err(PLCDIDError::EmptyChain);
        }

        if !operations[0].is_genesis() {
            return Err(PLCDIDError::FirstOperationNotGenesis);
        }

        let mut current_state = PlcState::new();
        let mut prev_cid: Option<String> = None;

        for (i, operation) in operations.iter().enumerate() {
            if i == 0 {
                if operation.prev().is_some() {
                    return Err(PLCDIDError::InvalidPrev {
                        details: "Genesis operation must have prev = null".to_string(),
                    });
                }
            } else {
                let expected_prev =
                    prev_cid
                        .as_ref()
                        .ok_or_else(|| PLCDIDError::ChainValidationFailed {
                            details: "Missing previous CID".to_string(),
                        })?;

                let actual_prev = operation.prev().ok_or_else(|| PLCDIDError::InvalidPrev {
                    details: "Non-genesis operation must have prev field".to_string(),
                })?;

                if actual_prev != expected_prev {
                    return Err(PLCDIDError::InvalidPrev {
                        details: format!("Expected prev = {}, got {}", expected_prev, actual_prev),
                    });
                }
            }

            // Verify signature using current rotation keys
            if !current_state.rotation_keys.is_empty() {
                let verifying_keys: Result<Vec<_>, _> = current_state
                    .rotation_keys
                    .iter()
                    .map(|k| {
                        let key_data =
                            key::identify_key(k).map_err(|e| PLCDIDError::InvalidRotationKeys {
                                details: format!("Failed to parse rotation key: {}", e),
                            })?;
                        key::to_public(&key_data).map_err(|e| PLCDIDError::InvalidRotationKeys {
                            details: format!("Failed to derive public key: {}", e),
                        })
                    })
                    .collect();

                operation.verify(&verifying_keys?)?;
            } else if i > 0 {
                return Err(PLCDIDError::InvalidRotationKeys {
                    details: "No rotation keys available for verification".to_string(),
                });
            }

            // Apply operation to state
            match operation {
                Operation::PlcOperation {
                    rotation_keys,
                    verification_methods,
                    also_known_as,
                    services,
                    ..
                } => {
                    current_state.rotation_keys = rotation_keys.clone();
                    current_state.verification_methods = verification_methods.clone();
                    current_state.also_known_as = also_known_as.clone();
                    current_state.services = services.clone();

                    current_state.validate()?;
                }
                Operation::PlcTombstone { .. } => {
                    current_state = PlcState::new();
                }
                Operation::LegacyCreate { .. } => {
                    return Err(PLCDIDError::InvalidOperationType {
                        details: "Legacy create operations not fully supported".to_string(),
                    });
                }
            }

            prev_cid = Some(operation.cid()?);
        }

        Ok(current_state)
    }

    /// Validate a chain with fork resolution.
    ///
    /// Handles the recovery mechanism where operations signed by higher-priority
    /// rotation keys can invalidate later operations if submitted within 72 hours.
    pub fn validate_chain_with_forks(
        operations: &[Operation],
        timestamps: &[DateTime<Utc>],
    ) -> Result<PlcState, PLCDIDError> {
        if operations.len() != timestamps.len() {
            return Err(PLCDIDError::ChainValidationFailed {
                details: "Operations and timestamps length mismatch".to_string(),
            });
        }

        if operations.is_empty() {
            return Err(PLCDIDError::EmptyChain);
        }

        let operations_with_timestamps: Vec<OperationWithTimestamp> = operations
            .iter()
            .zip(timestamps.iter())
            .map(|(op, ts)| OperationWithTimestamp {
                operation: op.clone(),
                timestamp: *ts,
            })
            .collect();

        let builder = CanonicalChainBuilder::new(operations_with_timestamps);
        let canonical_chain = builder.build()?;

        Self::validate_chain(&canonical_chain)
    }

    /// Check if an operation is within the recovery window relative to another.
    ///
    /// Returns true if the time difference is less than 72 hours and non-negative.
    pub fn is_within_recovery_window(
        fork_timestamp: DateTime<Utc>,
        current_timestamp: DateTime<Utc>,
    ) -> bool {
        let diff = current_timestamp - fork_timestamp;
        diff < Duration::hours(RECOVERY_WINDOW_HOURS) && diff >= Duration::zero()
    }
}

/// Validate rotation keys.
///
/// Checks 1-5 keys, no duplicates, valid did:key format, and parseable key types.
pub fn validate_rotation_keys(keys: &[String]) -> Result<(), PLCDIDError> {
    if keys.is_empty() {
        return Err(PLCDIDError::InvalidRotationKeys {
            details: "At least one rotation key is required".to_string(),
        });
    }

    if keys.len() > 5 {
        return Err(PLCDIDError::TooManyEntries {
            field: "rotation_keys".to_string(),
            max: 5,
            actual: keys.len(),
        });
    }

    let mut seen = std::collections::HashSet::new();
    for key_str in keys {
        if !seen.insert(key_str) {
            return Err(PLCDIDError::DuplicateEntry {
                field: "rotation_keys".to_string(),
                value: key_str.clone(),
            });
        }

        if !key_str.starts_with("did:key:") {
            return Err(PLCDIDError::InvalidRotationKeys {
                details: format!("Rotation key must be in did:key format: {}", key_str),
            });
        }

        key::identify_key(key_str).map_err(|e| PLCDIDError::InvalidRotationKeys {
            details: format!("Failed to parse rotation key: {}", e),
        })?;
    }

    Ok(())
}

/// Validate verification methods.
///
/// Checks max 10 methods, valid did:key format.
pub fn validate_verification_methods(methods: &HashMap<String, String>) -> Result<(), PLCDIDError> {
    if methods.len() > MAX_VERIFICATION_METHODS {
        return Err(PLCDIDError::TooManyEntries {
            field: "verification_methods".to_string(),
            max: MAX_VERIFICATION_METHODS,
            actual: methods.len(),
        });
    }

    for (name, key_str) in methods {
        if !key_str.starts_with("did:key:") {
            return Err(PLCDIDError::InvalidVerificationMethods {
                details: format!(
                    "Verification method '{}' must be in did:key format: {}",
                    name, key_str
                ),
            });
        }

        key::identify_key(key_str).map_err(|e| PLCDIDError::InvalidVerificationMethods {
            details: format!("Failed to parse verification method key: {}", e),
        })?;
    }

    Ok(())
}

/// Validate also-known-as URIs.
pub fn validate_also_known_as(uris: &[String]) -> Result<(), PLCDIDError> {
    for uri in uris {
        if uri.is_empty() {
            return Err(PLCDIDError::InvalidAlsoKnownAs {
                details: "URI cannot be empty".to_string(),
            });
        }

        if !uri.contains(':') {
            return Err(PLCDIDError::InvalidAlsoKnownAs {
                details: format!("URI must contain a scheme: {}", uri),
            });
        }
    }

    Ok(())
}

/// Validate service endpoints.
pub fn validate_services(services: &HashMap<String, ServiceEndpoint>) -> Result<(), PLCDIDError> {
    for (name, service) in services {
        if name.is_empty() {
            return Err(PLCDIDError::InvalidService {
                details: "Service name cannot be empty".to_string(),
            });
        }

        service.validate()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::state::ServiceEndpoint;
    use super::*;
    use crate::key::{KeyType, generate_key, to_public};

    #[test]
    fn test_validate_chain_genesis() {
        let private_key_data = generate_key(KeyType::P256Private).unwrap();
        let did_key = format!("{}", private_key_data);

        let unsigned =
            Operation::new_genesis(vec![did_key], HashMap::new(), vec![], HashMap::new());

        let signed = unsigned.sign(&private_key_data).unwrap();

        let state = OperationChainValidator::validate_chain(&[signed]).unwrap();
        assert_eq!(state.rotation_keys.len(), 1);
    }

    #[test]
    fn test_validate_chain_empty() {
        assert!(OperationChainValidator::validate_chain(&[]).is_err());
    }

    #[test]
    fn test_validate_chain_genesis_and_update() {
        let private_key_data = generate_key(KeyType::P256Private).unwrap();
        let public_key_data = to_public(&private_key_data).unwrap();
        let did_key = format!("{}", public_key_data);

        let genesis = Operation::new_genesis(
            vec![did_key.clone()],
            HashMap::new(),
            vec![],
            HashMap::new(),
        )
        .sign(&private_key_data)
        .unwrap();

        let genesis_cid = genesis.cid().unwrap();

        let update = Operation::new_update(
            vec![did_key],
            HashMap::new(),
            vec!["at://alice.example.com".to_string()],
            HashMap::new(),
            genesis_cid,
        )
        .sign(&private_key_data)
        .unwrap();

        let state = OperationChainValidator::validate_chain(&[genesis, update]).unwrap();
        assert_eq!(state.also_known_as, vec!["at://alice.example.com"]);
    }

    #[test]
    fn test_recovery_window() {
        let base = Utc::now();
        let within = base + Duration::hours(24);
        let outside = base + Duration::hours(100);

        assert!(OperationChainValidator::is_within_recovery_window(
            base, within
        ));
        assert!(!OperationChainValidator::is_within_recovery_window(
            base, outside
        ));
    }

    #[test]
    fn test_validate_rotation_keys() {
        let key1 = generate_key(KeyType::P256Private).unwrap();
        let key2 = generate_key(KeyType::K256Private).unwrap();

        let public1 = to_public(&key1).unwrap();
        let public2 = to_public(&key2).unwrap();

        let keys = vec![format!("{}", public1), format!("{}", public2)];
        assert!(validate_rotation_keys(&keys).is_ok());

        // Empty keys
        assert!(validate_rotation_keys(&[]).is_err());

        // Too many keys
        let many_keys: Vec<String> = (0..6)
            .map(|_| {
                let k = generate_key(KeyType::P256Private).unwrap();
                let p = to_public(&k).unwrap();
                format!("{}", p)
            })
            .collect();
        assert!(validate_rotation_keys(&many_keys).is_err());

        // Duplicate keys
        let dup_key = format!("{}", public1);
        let dup_keys = vec![dup_key.clone(), dup_key];
        assert!(validate_rotation_keys(&dup_keys).is_err());
    }

    #[test]
    fn test_validate_also_known_as() {
        let uris = vec![
            "at://alice.example.com".to_string(),
            "https://example.com".to_string(),
        ];
        assert!(validate_also_known_as(&uris).is_ok());

        assert!(validate_also_known_as(&[String::new()]).is_err());
        assert!(validate_also_known_as(&["not-a-uri".to_string()]).is_err());
    }

    #[test]
    fn test_fork_resolution_priority_within_window() {
        let primary_key = generate_key(KeyType::P256Private).unwrap();
        let backup_key = generate_key(KeyType::K256Private).unwrap();

        let primary_public = to_public(&primary_key).unwrap();
        let backup_public = to_public(&backup_key).unwrap();

        let rotation_keys = vec![format!("{}", primary_public), format!("{}", backup_public)];

        let genesis = Operation::new_genesis(
            rotation_keys.clone(),
            HashMap::new(),
            vec![],
            HashMap::new(),
        )
        .sign(&primary_key)
        .unwrap();

        let genesis_cid = genesis.cid().unwrap();
        let genesis_time = Utc::now();

        let mut services_a = HashMap::new();
        services_a.insert(
            "pds".to_string(),
            ServiceEndpoint {
                service_type: "AtprotoPersonalDataServer".to_string(),
                endpoint: "https://pds-a.example.com".to_string(),
            },
        );

        let op_a = Operation::new_update(
            rotation_keys.clone(),
            HashMap::new(),
            vec![],
            services_a,
            genesis_cid.clone(),
        )
        .sign(&backup_key)
        .unwrap();

        let op_a_time = genesis_time + Duration::hours(1);

        let mut services_b = HashMap::new();
        services_b.insert(
            "pds".to_string(),
            ServiceEndpoint {
                service_type: "AtprotoPersonalDataServer".to_string(),
                endpoint: "https://pds-b.example.com".to_string(),
            },
        );

        let op_b = Operation::new_update(
            rotation_keys,
            HashMap::new(),
            vec![],
            services_b,
            genesis_cid,
        )
        .sign(&primary_key)
        .unwrap();

        let op_b_time = op_a_time + Duration::hours(24);

        let operations = vec![genesis, op_a, op_b];
        let timestamps = vec![genesis_time, op_a_time, op_b_time];

        let result = OperationChainValidator::validate_chain_with_forks(&operations, &timestamps);
        assert!(result.is_ok());

        let state = result.unwrap();
        assert_eq!(
            state.services.get("pds").unwrap().endpoint,
            "https://pds-b.example.com"
        );
    }

    #[test]
    fn test_fork_resolution_priority_outside_window() {
        let primary_key = generate_key(KeyType::P256Private).unwrap();
        let backup_key = generate_key(KeyType::K256Private).unwrap();

        let primary_public = to_public(&primary_key).unwrap();
        let backup_public = to_public(&backup_key).unwrap();

        let rotation_keys = vec![format!("{}", primary_public), format!("{}", backup_public)];

        let genesis = Operation::new_genesis(
            rotation_keys.clone(),
            HashMap::new(),
            vec![],
            HashMap::new(),
        )
        .sign(&primary_key)
        .unwrap();

        let genesis_cid = genesis.cid().unwrap();
        let genesis_time = Utc::now();

        let mut services_a = HashMap::new();
        services_a.insert(
            "pds".to_string(),
            ServiceEndpoint {
                service_type: "AtprotoPersonalDataServer".to_string(),
                endpoint: "https://pds-a.example.com".to_string(),
            },
        );

        let op_a = Operation::new_update(
            rotation_keys.clone(),
            HashMap::new(),
            vec![],
            services_a,
            genesis_cid.clone(),
        )
        .sign(&backup_key)
        .unwrap();

        let op_a_time = genesis_time + Duration::hours(1);

        let mut services_b = HashMap::new();
        services_b.insert(
            "pds".to_string(),
            ServiceEndpoint {
                service_type: "AtprotoPersonalDataServer".to_string(),
                endpoint: "https://pds-b.example.com".to_string(),
            },
        );

        let op_b = Operation::new_update(
            rotation_keys,
            HashMap::new(),
            vec![],
            services_b,
            genesis_cid,
        )
        .sign(&primary_key)
        .unwrap();

        let op_b_time = op_a_time + Duration::hours(100); // Outside 72-hour window

        let operations = vec![genesis, op_a, op_b];
        let timestamps = vec![genesis_time, op_a_time, op_b_time];

        let result = OperationChainValidator::validate_chain_with_forks(&operations, &timestamps);
        assert!(result.is_ok());

        let state = result.unwrap();
        // Op A should win because op B is outside the recovery window
        assert_eq!(
            state.services.get("pds").unwrap().endpoint,
            "https://pds-a.example.com"
        );
    }

    #[test]
    fn test_fork_resolution_no_fork() {
        let private_key_data = generate_key(KeyType::P256Private).unwrap();
        let public_key_data = to_public(&private_key_data).unwrap();
        let rotation_keys = vec![format!("{}", public_key_data)];

        let genesis = Operation::new_genesis(
            rotation_keys.clone(),
            HashMap::new(),
            vec![],
            HashMap::new(),
        )
        .sign(&private_key_data)
        .unwrap();

        let genesis_cid = genesis.cid().unwrap();
        let genesis_time = Utc::now();

        let mut services = HashMap::new();
        services.insert(
            "pds".to_string(),
            ServiceEndpoint {
                service_type: "AtprotoPersonalDataServer".to_string(),
                endpoint: "https://pds.example.com".to_string(),
            },
        );

        let op1 =
            Operation::new_update(rotation_keys, HashMap::new(), vec![], services, genesis_cid)
                .sign(&private_key_data)
                .unwrap();

        let op1_time = genesis_time + Duration::hours(1);

        let operations = vec![genesis, op1];
        let timestamps = vec![genesis_time, op1_time];

        let result = OperationChainValidator::validate_chain_with_forks(&operations, &timestamps);
        assert!(result.is_ok());

        let state = result.unwrap();
        assert_eq!(
            state.services.get("pds").unwrap().endpoint,
            "https://pds.example.com"
        );
    }

    #[test]
    fn test_fork_resolution_mismatched_lengths() {
        let private_key_data = generate_key(KeyType::P256Private).unwrap();
        let public_key_data = to_public(&private_key_data).unwrap();
        let rotation_keys = vec![format!("{}", public_key_data)];

        let genesis = Operation::new_genesis(rotation_keys, HashMap::new(), vec![], HashMap::new())
            .sign(&private_key_data)
            .unwrap();

        let operations = vec![genesis];
        let timestamps = vec![Utc::now(), Utc::now()]; // Different length

        let result = OperationChainValidator::validate_chain_with_forks(&operations, &timestamps);
        assert!(result.is_err());
    }
}
