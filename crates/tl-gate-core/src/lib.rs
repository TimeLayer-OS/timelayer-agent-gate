//! TL-Gate core: canonical `ActionIntent`, BLAKE3 commitments, gate decisions,
//! and chain states.
//!
//! Status: **Phase 0 (protocol freeze in progress).** What is implemented here
//! is real and tested: intent canonicalization and the domain-separated BLAKE3
//! commitment. Everything the spec describes beyond that (receipt resolution,
//! broker, validation, finalization) is NOT here yet — and per the fail-closed
//! rule, absence of an implemented check means STOP, never silent ALLOW.
//!
//! Canonical serialization (provisional until TL-GATE-WIRE/v1 freezes): JSON
//! with alphabetically sorted keys, no insignificant whitespace, UTF-8. The
//! digest is BLAKE3-256 over `domain_separator || canonical_bytes`. SHA-256 is
//! deliberately absent from this crate (spec P-10: BLAKE3 only).

use serde::{Deserialize, Serialize};

/// Domain separator for the intent commitment (spec §9.2).
pub const INTENT_DOMAIN_V1: &str = "TL-GATE/INTENT/v1";

/// Side-effect class of an action (spec §13). Determines execution profile
/// and how much validation/finality discipline the chain demands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideEffectClass {
    /// R0 — pure computation, no external effect.
    R0,
    /// R1 — read-only external access.
    R1,
    /// W1 — reversible write.
    W1,
    /// W2 — transactional external effect.
    W2,
    /// W3 — irreversible or high-impact effect.
    W3,
}

/// Canonical, immutable description of one proposed action (spec §5, §8.2).
///
/// Any change to any field after canonicalization is a NEW intent with a new
/// `action_id` and a new chain — never an edit (spec §8.2, §9.7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionIntent {
    /// Protocol schema tag, e.g. "tl-gate.action-intent/1".
    pub schema: String,
    /// Who the action is performed on behalf of.
    pub principal: String,
    /// The external planner that proposed the action.
    pub orchestrator: String,
    /// The concrete executing agent in a concrete session.
    pub agent_instance: String,
    /// Session binding reference.
    pub session_ref: String,
    /// Capability kind: "filesystem.write", "http.get", "process.exec", ...
    pub capability: String,
    /// Canonicalized target (absolute path, normalized URL, ...).
    pub target: String,
    /// BLAKE3-256 hex of the canonical argument bytes (the arguments
    /// themselves may stay in user-owned storage — TB-06).
    pub arguments_digest: String,
    /// Tool identity the action is bound to (id@version, executable digest).
    pub tool_id: String,
    pub tool_version: String,
    pub tool_digest: String,
    /// Side-effect class (spec §13).
    pub side_effect_class: SideEffectClass,
    /// Chain coordinates.
    pub action_id: String,
    pub chain_id: String,
    pub attempt: u64,
    /// Causal parent (empty string for a chain root).
    pub parent_digest: String,
}

impl ActionIntent {
    /// Canonical bytes: JSON with alphabetically sorted keys (serde_json's
    /// default map ordering), no extra whitespace. Provisional wire form
    /// until TL-GATE-WIRE/v1 (length-prefixed binary) freezes.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // Route through Value to get sorted-key ordering regardless of the
        // struct's field declaration order.
        let v: serde_json::Value =
            serde_json::to_value(self).expect("ActionIntent is always serializable");
        serde_json::to_vec(&v).expect("canonical serialization")
    }

    /// The intent commitment: BLAKE3-256 over the domain separator followed
    /// by the canonical bytes (spec §9.2). Lowercase hex, 64 chars.
    pub fn intent_digest(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(INTENT_DOMAIN_V1.as_bytes());
        hasher.update(&self.canonical_bytes());
        hasher.finalize().to_hex().to_string()
    }
}

/// The only two outcomes a gate decision can have (spec §8.7): there is no
/// "best-effort allow".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Allow,
    Stop(StopCode),
}

/// Stop reason codes (spec §21, condensed for Phase 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopCode {
    NoPermissionReceipt,
    NoScopeReceipt,
    NoToolReceipt,
    ReceiptNotValid,
    SubjectMismatch,
    ScopeViolation,
    ToolSubstitution,
    Revoked,
    Replay,
    Expired,
    DelegationAmplification,
    ValidationFailed,
    FinalityConflict,
    VerifierUnavailable,
    SchemaMismatch,
    NotImplemented,
}

impl std::fmt::Display for StopCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::NoPermissionReceipt => "NO_PERMISSION_RECEIPT",
            Self::NoScopeReceipt => "NO_SCOPE_RECEIPT",
            Self::NoToolReceipt => "NO_TOOL_RECEIPT",
            Self::ReceiptNotValid => "RECEIPT_NOT_VALID",
            Self::SubjectMismatch => "SUBJECT_MISMATCH",
            Self::ScopeViolation => "SCOPE_VIOLATION",
            Self::ToolSubstitution => "TOOL_SUBSTITUTION",
            Self::Revoked => "REVOKED",
            Self::Replay => "REPLAY",
            Self::Expired => "EXPIRED",
            Self::DelegationAmplification => "DELEGATION_AMPLIFICATION",
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::FinalityConflict => "FINALITY_CONFLICT",
            Self::VerifierUnavailable => "VERIFIER_UNAVAILABLE",
            Self::SchemaMismatch => "SCHEMA_MISMATCH",
            Self::NotImplemented => "NOT_IMPLEMENTED",
        };
        f.write_str(s)
    }
}

/// Chain lifecycle states (spec §20), including the two honest in-between
/// states most systems pretend don't exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainState {
    Proposed,
    Allowed,
    Executing,
    /// Effect happened, final receipt not yet issued — NOT the same as final.
    ExecutedUnfinalized,
    /// The effect may or may not have happened (crash mid-effect). Requires
    /// explicit reconciliation, never silent retry.
    EffectUnknown,
    Validated,
    Final,
    Stopped,
}

/// BLAKE3-256 hex of arbitrary bytes with an explicit domain separator.
/// Every TL-Gate commitment goes through here — never a bare hash (spec §10.2).
pub fn domain_digest(domain: &str, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent() -> ActionIntent {
        ActionIntent {
            schema: "tl-gate.action-intent/1".into(),
            principal: "user:owner".into(),
            orchestrator: "orchestrator:generic".into(),
            agent_instance: "agent:demo#1".into(),
            session_ref: "session-1".into(),
            capability: "filesystem.write".into(),
            target: "/workspace/project-a/src/main.rs".into(),
            arguments_digest: domain_digest("TL-GATE/ARGS/v1", b"patch-bytes"),
            tool_id: "fs-connector".into(),
            tool_version: "1.0.0".into(),
            tool_digest: "0".repeat(64),
            side_effect_class: SideEffectClass::W1,
            action_id: "act-1".into(),
            chain_id: "chain-1".into(),
            attempt: 1,
            parent_digest: String::new(),
        }
    }

    #[test]
    fn digest_is_deterministic() {
        assert_eq!(intent().intent_digest(), intent().intent_digest());
        assert_eq!(intent().intent_digest().len(), 64);
    }

    #[test]
    fn any_field_change_changes_digest() {
        let base = intent().intent_digest();
        let mut i = intent();
        i.target = "/workspace/project-a/src/lib.rs".into();
        assert_ne!(base, i.intent_digest());
        let mut i = intent();
        i.attempt = 2;
        assert_ne!(base, i.intent_digest());
        let mut i = intent();
        i.side_effect_class = SideEffectClass::W3;
        assert_ne!(base, i.intent_digest());
    }

    #[test]
    fn domain_separation_matters() {
        let bytes = intent().canonical_bytes();
        assert_ne!(
            domain_digest("TL-GATE/INTENT/v1", &bytes),
            domain_digest("TL-GATE/EXECUTION/v1", &bytes),
            "same bytes under different domains must never collide into one commitment"
        );
    }

    #[test]
    fn canonical_bytes_are_key_sorted() {
        let s = String::from_utf8(intent().canonical_bytes()).unwrap();
        let a = s.find("\"action_id\"").unwrap();
        let t = s.find("\"tool_id\"").unwrap();
        assert!(a < t, "keys must be alphabetically ordered");
    }
}
