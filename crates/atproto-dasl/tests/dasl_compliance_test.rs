//! DASL compliance tests using community test vectors.
//!
//! Runs the test fixtures from <https://github.com/hyphacoop/dasl-testing>
//! against our DRISL (DAG-CBOR) encoder/decoder.

use atproto_dasl::{DecodeConfig, Ipld, from_slice_non_strict, from_slice_with_config, to_vec};
use serde::Deserialize;
use std::path::Path;

/// A single test case from a DASL fixture file.
#[derive(Deserialize)]
#[allow(dead_code)]
struct TestCase {
    /// Test type: "roundtrip", "invalid_in", or "invalid_out".
    #[serde(rename = "type")]
    test_type: String,
    /// Hex-encoded CBOR bytes.
    data: String,
    /// Human-readable test name.
    name: String,
    /// Spec tags (e.g. "dag-cbor", "dasl-cid", "basic").
    #[serde(default)]
    tags: Vec<String>,
    /// Detailed description.
    #[serde(default)]
    desc: String,
    /// Optional unique test ID.
    #[serde(default)]
    id: Option<String>,
}

/// Tests to skip in standard mode (strict=true, validate_dasl_cid=false).
///
/// Format: (test_type, name_substring_or_id, reason)
/// Use "*" for test_type to match any type.
///
/// Skip categories:
///   - Tested elsewhere (6): DASL CID constraints validated in `dasl_cid_validation`
///   - Out of scope (9): Specs beyond DAG-CBOR (RFC 8949 floats, BigInt tags,
///     non-string keys, dCBOR NFC)
///
/// Total: 15 skipped (6 tested elsewhere + 9 out of scope)
///
/// Note: The `invalid_out` tests (datetime, undefined, unassigned) are NOT
/// skipped because `run_invalid_out` uses non-strict decode, which either
/// fails (acceptable) or produces different canonical output (also acceptable).
const SKIP_LIST: &[(&str, &str, &str)] = &[
    // --- Tested elsewhere: DASL CID constraints (6 tests) ---
    // These require `validate_dasl_cid=true` to enforce DASL-specific CID rules
    // (CIDv1 only, restricted codecs/hashes, 32-byte digest).
    // Validated separately in the `dasl_cid_validation` test.
    ("*", "CIDv0", "tested in dasl_cid_validation"),
    ("*", "empty CID", "tested in dasl_cid_validation"),
    ("*", "short hash digest", "tested in dasl_cid_validation"),
    ("*", "long hash digest", "tested in dasl_cid_validation"),
    (
        "*",
        "CIDv1 that isn't raw or cbor",
        "tested in dasl_cid_validation",
    ),
    ("*", "disallowed hash type", "tested in dasl_cid_validation"),
    // --- Out of scope: RFC 8949 / CBOR-Core float rules (4 tests) ---
    // DAG-CBOR requires 64-bit floats only and bans NaN/Infinity entirely.
    // These test RFC 8949 float reduction and CBOR-Core NaN handling.
    (
        "roundtrip",
        "float reduction",
        "RFC 8949 float reduction not applicable to DAG-CBOR (64-bit only)",
    ),
    (
        "invalid_in",
        "float reduction",
        "RFC 8949 float reduction not applicable to DAG-CBOR",
    ),
    (
        "roundtrip",
        "short NaN",
        "DAG-CBOR bans NaN entirely (RFC 8949/CBOR-Core test)",
    ),
    (
        "roundtrip",
        "zero float16",
        "DAG-CBOR requires 64-bit floats (RFC 8949/CBOR-Core test)",
    ),
    // --- Out of scope: BigInt CBOR tags 2/3 (2 tests) ---
    // DAG-CBOR only allows tag 42 (CIDs). BigInt tags are not part of the spec.
    (
        "roundtrip",
        "smallest unsigned bigint",
        "CBOR tag 2 (BigInt) not in DAG-CBOR (tag 42 only)",
    ),
    (
        "roundtrip",
        "highest negative bigint",
        "CBOR tag 3 (BigInt) not in DAG-CBOR (tag 42 only)",
    ),
    // --- Out of scope: Non-string map keys (1 test) ---
    // DAG-CBOR requires all map keys to be strings.
    (
        "roundtrip",
        "map with equal int and float keys",
        "DAG-CBOR requires string-only map keys",
    ),
    // --- Out of scope: Unicode NFC normalization (1 test) ---
    // NFC normalization is a dCBOR requirement, not DAG-CBOR.
    (
        "invalid_in",
        "text not in Unicode Normalization Form C",
        "dCBOR NFC requirement, not enforced by DAG-CBOR",
    ),
];

/// Tests to skip in DASL CID validation mode.
const DASL_CID_SKIP_LIST: &[(&str, &str, &str)] = &[
    // Same permanent skips as standard mode that appear in cid.json: none.
    // All CID tests should pass with DASL CID validation enabled.
];

/// Check if a test case should be skipped using the given skip list.
fn should_skip_with<'a>(test: &TestCase, skip_list: &[(&str, &str, &'a str)]) -> Option<&'a str> {
    let name = &test.name;
    let id = test.id.as_deref().unwrap_or("");

    for &(test_type, pattern, reason) in skip_list {
        let type_matches = test_type == "*" || test_type == test.test_type;
        let name_matches = name.contains(pattern) || id == pattern;
        if type_matches && name_matches {
            return Some(reason);
        }
    }

    None
}

/// Run all tests from a single fixture file with the given config and skip list.
fn run_fixture_with(path: &str, config: DecodeConfig, skip_list: &[(&str, &str, &str)]) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let full_path = Path::new(manifest_dir).join(path);
    let content = std::fs::read_to_string(&full_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", full_path.display(), e));

    let tests: Vec<TestCase> = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", full_path.display(), e));

    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0;
    let mut skipped = 0;

    for (i, test) in tests.iter().enumerate() {
        if let Some(reason) = should_skip_with(test, skip_list) {
            skipped += 1;
            eprintln!(
                "  SKIP [{}] #{}: {} ({})",
                test.test_type, i, test.name, reason
            );
            continue;
        }

        let data = match hex::decode(&test.data) {
            Ok(d) => d,
            Err(e) => {
                failures.push(format!(
                    "#{} '{}': failed to decode hex: {}",
                    i, test.name, e
                ));
                continue;
            }
        };

        let result = match test.test_type.as_str() {
            "roundtrip" => run_roundtrip(&data, &test.name, &config),
            "invalid_in" => run_invalid_in(&data, &test.name, &config),
            "invalid_out" => run_invalid_out(&data, &test.name),
            other => Err(format!("unknown test type: {}", other)),
        };

        match result {
            Ok(()) => {
                passed += 1;
            }
            Err(msg) => {
                failures.push(format!(
                    "#{} [{}] '{}': {}",
                    i, test.test_type, test.name, msg
                ));
            }
        }
    }

    let file_name = Path::new(path).file_name().unwrap().to_str().unwrap();
    eprintln!(
        "{}: {} passed, {} skipped, {} failed (of {} total)",
        file_name,
        passed,
        skipped,
        failures.len(),
        tests.len()
    );

    if !failures.is_empty() {
        let msg = failures.join("\n  ");
        panic!(
            "{} compliance failures in {}:\n  {}",
            failures.len(),
            file_name,
            msg
        );
    }
}

/// Run all tests from a single fixture file with default strict config.
fn run_fixture(path: &str) {
    run_fixture_with(path, DecodeConfig::default(), SKIP_LIST);
}

/// Roundtrip test: decode bytes, re-encode, verify output matches input.
fn run_roundtrip(data: &[u8], name: &str, config: &DecodeConfig) -> Result<(), String> {
    let decoded: Ipld = from_slice_with_config(data, config.clone())
        .map_err(|e| format!("decode failed (expected success): {}", e))?;

    let encoded = to_vec(&decoded).map_err(|e| format!("re-encode failed: {}", e))?;

    if encoded != data {
        Err(format!(
            "roundtrip mismatch for '{}': input={} output={}",
            name,
            hex::encode(data),
            hex::encode(&encoded)
        ))
    } else {
        Ok(())
    }
}

/// Invalid input test: decoding must fail.
fn run_invalid_in(data: &[u8], _name: &str, config: &DecodeConfig) -> Result<(), String> {
    match from_slice_with_config::<Ipld>(data, config.clone()) {
        Err(_) => Ok(()),
        Ok(val) => Err(format!(
            "decode succeeded but should have failed, got: {:?}",
            val
        )),
    }
}

/// Invalid output test: decode with non-strict, then re-encode must differ.
fn run_invalid_out(data: &[u8], _name: &str) -> Result<(), String> {
    // First decode with non-strict mode (general CBOR)
    let decoded: Ipld = match from_slice_non_strict(data) {
        Ok(val) => val,
        // If even non-strict decoding fails, that's acceptable — the value
        // can't be represented in our type system at all.
        Err(_) => return Ok(()),
    };

    // Now try to re-encode with strict DAG-CBOR
    match to_vec(&decoded) {
        Err(_) => Ok(()),
        Ok(encoded) => {
            // If encoding succeeded, the output must differ from input
            // (non-canonical input gets canonicalized)
            if encoded == data {
                Err(format!(
                    "encode succeeded with identical output, should have failed or changed: {}",
                    hex::encode(data)
                ))
            } else {
                // Encoding succeeded but produced different (canonical) output.
                // This is acceptable for invalid_out — the non-canonical form
                // was rejected by producing a different canonical form.
                Ok(())
            }
        }
    }
}

// One test function per fixture file for clear reporting.

#[test]
fn dasl_simple() {
    run_fixture("tests/dasl-testing/fixtures/cbor/simple.json");
}

#[test]
fn dasl_integer_range() {
    run_fixture("tests/dasl-testing/fixtures/cbor/integer_range.json");
}

#[test]
fn dasl_cid() {
    run_fixture("tests/dasl-testing/fixtures/cbor/cid.json");
}

#[test]
fn dasl_floats() {
    run_fixture("tests/dasl-testing/fixtures/cbor/floats.json");
}

#[test]
fn dasl_tags() {
    run_fixture("tests/dasl-testing/fixtures/cbor/tags.json");
}

#[test]
fn dasl_indefinite() {
    run_fixture("tests/dasl-testing/fixtures/cbor/indefinite.json");
}

#[test]
fn dasl_map_keys() {
    run_fixture("tests/dasl-testing/fixtures/cbor/map_keys.json");
}

#[test]
fn dasl_short_form() {
    run_fixture("tests/dasl-testing/fixtures/cbor/short_form.json");
}

#[test]
fn dasl_numeric_reduction() {
    run_fixture("tests/dasl-testing/fixtures/cbor/numeric_reduction.json");
}

#[test]
fn dasl_recursion() {
    // The deeply nested fixture requires:
    // 1. A high max_depth limit (the fixture has thousands of nested arrays)
    // 2. A larger stack than the default test thread provides
    let config = DecodeConfig::default().with_max_depth(16384);
    let builder = std::thread::Builder::new().stack_size(16 * 1024 * 1024);
    let handler = builder
        .spawn(move || {
            run_fixture_with(
                "tests/dasl-testing/fixtures/cbor/recursion.json",
                config,
                SKIP_LIST,
            );
        })
        .unwrap();
    handler.join().unwrap();
}

#[test]
fn dasl_concat() {
    run_fixture("tests/dasl-testing/fixtures/cbor/concat.json");
}

#[test]
fn dasl_utf8() {
    run_fixture("tests/dasl-testing/fixtures/cbor/utf8.json");
}

/// Run CID fixture with DASL CID validation enabled.
///
/// This tests DASL-specific CID constraints (CIDv1 only, restricted codecs,
/// restricted hash algorithms, 32-byte digest) that go beyond basic DAG-CBOR
/// CID validation.
#[test]
fn dasl_cid_validation() {
    run_fixture_with(
        "tests/dasl-testing/fixtures/cbor/cid.json",
        DecodeConfig::dasl(),
        DASL_CID_SKIP_LIST,
    );
}
