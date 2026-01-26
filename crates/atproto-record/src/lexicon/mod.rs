//! AT Protocol lexicon type definitions and implementations.
//!
//! This module provides Rust implementations of AT Protocol lexicon types,
//! organized by namespace following the AT Protocol naming conventions.
//!
//! # Structure
//!
//! The module is organized into two main namespaces:
//!
//! - `com.atproto.*` - Core AT Protocol types (e.g., StrongRef)
//! - `community.lexicon.*` - Community-defined types for social features
//!
//! # TypedLexicon Pattern
//!
//! Most types in this module follow the TypedLexicon pattern, which automatically
//! handles the `$type` field required by AT Protocol for type discrimination.
//! This pattern ensures correct serialization and deserialization of lexicon types.
//!
//! # Example
//!
//! ```ignore
//! use atproto_record::lexicon::com::atproto::repo::{StrongRef, TypedStrongRef};
//!
//! // Create a strong reference
//! let strong_ref = StrongRef {
//!     uri: "at://did:plc:example/collection/record".to_string(),
//!     cid: "bafyreicid123".to_string(),
//! };
//!
//! // Wrap it with TypedLexicon for automatic $type handling
//! let typed_ref = TypedStrongRef::new(strong_ref);
//! ```

mod app_bsky_richtext_facet;
mod com_atproto_repo;
mod community_lexicon_attestation;
mod community_lexicon_badge;
mod community_lexicon_calendar_event;
mod community_lexicon_calendar_rsvp;
mod community_lexicon_location;
mod primatives;

// Re-export primitive types for convenience
pub use primatives::*;

/// Bluesky application namespace.
///
/// Contains lexicon types specific to the Bluesky application,
/// including rich text formatting and social features.
pub mod app {
    /// Bluesky namespace.
    pub mod bsky {
        /// Rich text formatting types.
        pub mod richtext {
            /// Facet types for semantic text annotations.
            ///
            /// Provides types for mentions, links, hashtags, and other
            /// structured metadata that can be attached to text content.
            pub mod facet {
                pub use crate::lexicon::app_bsky_richtext_facet::*;
            }
        }
    }
}

/// AT Protocol core types namespace
pub mod com {
    /// AT Protocol namespace
    pub mod atproto {
        /// Repository-related types
        pub mod repo {
            pub use crate::lexicon::com_atproto_repo::*;
        }
    }
}

/// Community lexicon types namespace
pub mod community {
    /// Community lexicon definitions
    pub mod lexicon {
        /// Calendar-related types for events and RSVPs
        pub mod calendar {
            /// Event-related types
            pub mod event {
                pub use crate::lexicon::community_lexicon_calendar_event::*;
            }
            /// RSVP-related types
            pub mod rsvp {
                pub use crate::lexicon::community_lexicon_calendar_rsvp::*;
            }
        }
        /// Location-related types
        pub mod location {
            pub use crate::lexicon::community_lexicon_location::*;
        }

        /// Attestation and signature types
        pub mod attestation {
            pub use crate::lexicon::community_lexicon_attestation::*;
        }

        /// Badge and award types
        pub mod badge {
            pub use crate::lexicon::community_lexicon_badge::*;
        }
    }
}
