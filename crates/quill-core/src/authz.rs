//! Counselor-led AI authorization: per-chamber "gate toggles".
//!
//! An AI agent may only operate inside a chamber the supervising counselor has
//! explicitly enabled, and only if that specific agent is on the chamber's
//! allow-list. This is the security tie-in between the AI layer and the vault
//! model — the router (in `quill-ai`) must consult a [`ChamberAiAuthorization`]
//! before dispatching any agent into a chamber.

use crate::error::{CoreError, Result};
use crate::ids::{AgentId, VaultId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The AI authorization state for a single chamber.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChamberAiAuthorization {
    /// The chamber this authorization governs.
    pub chamber: VaultId,
    /// Master switch: if `false`, no agent may run in the chamber regardless of
    /// the allow-list.
    pub enabled: bool,
    /// The set of agents the counselor has explicitly permitted.
    authorized: BTreeSet<AgentId>,
}

impl ChamberAiAuthorization {
    /// A brand-new chamber starts with AI disabled and an empty allow-list
    /// (privacy by default).
    #[must_use]
    pub fn disabled(chamber: VaultId) -> Self {
        Self {
            chamber,
            enabled: false,
            authorized: BTreeSet::new(),
        }
    }

    /// Toggle the chamber-wide master switch.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Add an agent to the allow-list.
    pub fn authorize(&mut self, agent: AgentId) {
        self.authorized.insert(agent);
    }

    /// Remove an agent from the allow-list.
    pub fn revoke(&mut self, agent: AgentId) {
        self.authorized.remove(&agent);
    }

    /// Whether `agent` may currently run in this chamber (master switch on *and*
    /// agent allow-listed).
    #[must_use]
    pub fn is_authorized(&self, agent: AgentId) -> bool {
        self.enabled && self.authorized.contains(&agent)
    }

    /// Assert authorization, producing a typed error suitable for the audit log.
    ///
    /// # Errors
    /// Returns [`CoreError::AgentNotAuthorized`] when the agent is not permitted.
    pub fn ensure_authorized(&self, agent: AgentId) -> Result<()> {
        if self.is_authorized(agent) {
            Ok(())
        } else {
            Err(CoreError::AgentNotAuthorized {
                agent: agent.to_string(),
                chamber: self.chamber.to_string(),
            })
        }
    }

    /// Snapshot of the currently authorized agents.
    #[must_use]
    pub fn authorized_agents(&self) -> Vec<AgentId> {
        self.authorized.iter().copied().collect()
    }
}
