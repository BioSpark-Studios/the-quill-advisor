//! Strongly-typed identifiers.
//!
//! Newtypes over [`uuid::Uuid`] so that a `ChamberId` can never be passed where
//! a `VaultId` is expected. All are `Copy`, serde-serializable, and display as
//! plain UUID strings.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! typed_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generate a fresh random identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// The underlying UUID.
            #[must_use]
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }
    };
}

typed_id!(
    /// Identifies any node in the vault hierarchy (Master, Counselor, Classroom, Chamber).
    VaultId
);
typed_id!(
    /// Identifies a specialized AI agent (e.g. Quantum Quill).
    AgentId
);
typed_id!(
    /// Identifies a human actor (counselor, director, student, parent).
    ActorId
);
