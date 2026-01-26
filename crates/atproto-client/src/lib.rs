//! AT Protocol HTTP client with authentication support.
//!
//! HTTP client for AT Protocol services supporting DPoP, bearer tokens, and sessions
//! with native XRPC protocol operations and repository management.
//! - **`url`**: URL construction and validation utilities for AT Protocol endpoints
//! - **`com::atproto::repo`**: Repository operations for record management
//! - **`com::atproto::server`**: Server operations for authentication and session management
//! - **`errors`**: Structured error types for HTTP and authentication failures
//!
//! ## Command-Line Tools
//!
//! When built with the `clap` feature, provides XRPC client tools:
//!
//! - **`atproto-client-dpop`**: Make authenticated XRPC calls using DPoP (Demonstration of Proof-of-Possession) tokens
//! - **`atproto-client-auth`**: Create and refresh authentication sessions with AT Protocol services
//! - **`atproto-client-app-password`**: Make authenticated XRPC calls using application-specific Bearer tokens

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod client;
pub mod errors;
pub mod record_resolver;

pub use record_resolver::{HttpRecordResolver, RecordResolver};

mod com_atproto_identity;
mod com_atproto_repo;
mod com_atproto_server;

/// AT Protocol namespace modules.
pub mod com {
    /// AT Protocol core modules.
    pub mod atproto {
        /// Repository operations for AT Protocol records.
        pub mod repo {
            pub use crate::com_atproto_repo::*;
        }
        /// Server operations for AT Protocol authentication and session management.
        pub mod server {
            pub use crate::com_atproto_server::*;
        }

        /// Identity operations for AT Protocol handle and DID resolution.
        pub mod identity {
            pub use crate::com_atproto_identity::*;
        }
    }
}
