//! Intent Normalizer (SPEC §8.2): adapter-specific proposal → canonical,
//! immutable `ActionIntent`.
//!
//! Phase 1 slice: filesystem.* and http(s).* target canonicalization, BLAKE3
//! arguments digest under TL-GATE/ARGS/v1, content-derived `action_id`. After
//! normalization the intent is immutable — any change is a NEW action (§9.7).
//!
//! Symlink and mount-escape resolution happens at the broker boundary against
//! the live filesystem (§9.4); the normalizer rejects everything that is
//! already wrong lexically: relative paths, `.`/`..` components, empty hosts.

use serde::{Deserialize, Serialize};

use crate::{domain_digest, ActionIntent, SideEffectClass};

pub const ARGS_DOMAIN_V1: &str = "TL-GATE/ARGS/v1";
pub const ACTION_ID_DOMAIN_V1: &str = "TL-GATE/ACTION-ID/v1";

/// What an adapter hands over. A proposal is NOT a permission (§9.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub principal: String,
    pub orchestrator: String,
    pub agent_instance: String,
    pub session_ref: String,
    pub capability: String,
    pub target: String,
    /// Arbitrary JSON arguments; only their canonical digest enters the intent.
    pub arguments: serde_json::Value,
    pub tool_id: String,
    pub tool_version: String,
    pub tool_digest: String,
    pub side_effect_class: SideEffectClass,
    /// Empty = chain root (chain_id becomes the derived action_id).
    #[serde(default)]
    pub chain_id: String,
    #[serde(default)]
    pub parent_digest: String,
    #[serde(default = "one")]
    pub attempt: u64,
}

fn one() -> u64 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizeError {
    EmptyField(&'static str),
    RelativePath(String),
    DotComponent(String),
    BadUrl(String),
    UnsupportedCapability(String),
}

impl std::fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(w) => write!(f, "normalize: field '{w}' must not be empty"),
            Self::RelativePath(t) => write!(f, "normalize: filesystem target must be absolute: '{t}'"),
            Self::DotComponent(t) => {
                write!(f, "normalize: '.' or '..' path components are rejected before scope checks: '{t}'")
            }
            Self::BadUrl(t) => write!(f, "normalize: http target must be http(s)://host/…: '{t}'"),
            Self::UnsupportedCapability(c) => {
                write!(f, "normalize: capability family '{c}' has no canonicalizer yet — fail-closed")
            }
        }
    }
}

impl std::error::Error for NormalizeError {}

/// Lexical canonicalization of a filesystem path: absolute, no `.`/`..`,
/// collapsed duplicate slashes, no trailing slash (except root).
fn canonical_fs_path(target: &str) -> Result<String, NormalizeError> {
    if !target.starts_with('/') {
        return Err(NormalizeError::RelativePath(target.to_string()));
    }
    let mut parts: Vec<&str> = Vec::new();
    for comp in target.split('/') {
        match comp {
            "" => continue,
            "." | ".." => return Err(NormalizeError::DotComponent(target.to_string())),
            c => parts.push(c),
        }
    }
    Ok(format!("/{}", parts.join("/")))
}

/// Minimal canonicalization for http(s) targets: scheme+host lowercased,
/// default path "/". No userinfo, no fragment.
fn canonical_http_target(target: &str) -> Result<String, NormalizeError> {
    let bad = || NormalizeError::BadUrl(target.to_string());
    let (scheme, rest) = target.split_once("://").ok_or_else(bad)?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(bad());
    }
    let (host_part, path) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (rest, "/".to_string()),
    };
    if host_part.is_empty() || host_part.contains('@') || path.contains('#') {
        return Err(bad());
    }
    Ok(format!("{scheme}://{}{path}", host_part.to_ascii_lowercase()))
}

/// Build the canonical intent. Deterministic: the same proposal always yields
/// the same intent and the same digests.
pub fn normalize(p: &Proposal) -> Result<ActionIntent, NormalizeError> {
    for (name, v) in [
        ("principal", &p.principal),
        ("orchestrator", &p.orchestrator),
        ("agent_instance", &p.agent_instance),
        ("session_ref", &p.session_ref),
        ("capability", &p.capability),
        ("tool_id", &p.tool_id),
        ("tool_version", &p.tool_version),
        ("tool_digest", &p.tool_digest),
    ] {
        if v.is_empty() {
            return Err(NormalizeError::EmptyField(name));
        }
    }

    let target = match p.capability.split('.').next().unwrap_or("") {
        "filesystem" => canonical_fs_path(&p.target)?,
        "http" => canonical_http_target(&p.target)?,
        "process" => p.target.clone(), // command id; allowlisted by scope
        other => return Err(NormalizeError::UnsupportedCapability(other.to_string())),
    };

    // canonical JSON (sorted keys) → domain-separated digest; the raw
    // arguments stay in user-owned storage (TB-06)
    let canon_args = serde_json::to_vec(&p.arguments).expect("json args");
    let arguments_digest = domain_digest(ARGS_DOMAIN_V1, &canon_args);

    let id_material = format!(
        "{}\x00{}\x00{}\x00{}\x00{}",
        p.session_ref, p.capability, target, arguments_digest, p.attempt
    );
    let action_id = domain_digest(ACTION_ID_DOMAIN_V1, id_material.as_bytes())[..16].to_string();
    let chain_id = if p.chain_id.is_empty() { action_id.clone() } else { p.chain_id.clone() };

    Ok(ActionIntent {
        schema: "tl-gate.action-intent/1".into(),
        principal: p.principal.clone(),
        orchestrator: p.orchestrator.clone(),
        agent_instance: p.agent_instance.clone(),
        session_ref: p.session_ref.clone(),
        capability: p.capability.clone(),
        target,
        arguments_digest,
        tool_id: p.tool_id.clone(),
        tool_version: p.tool_version.clone(),
        tool_digest: p.tool_digest.clone(),
        side_effect_class: p.side_effect_class,
        action_id,
        chain_id,
        attempt: p.attempt,
        parent_digest: p.parent_digest.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal() -> Proposal {
        Proposal {
            principal: "user:owner".into(),
            orchestrator: "orch:generic".into(),
            agent_instance: "agent:demo#1".into(),
            session_ref: "s1".into(),
            capability: "filesystem.write".into(),
            target: "/workspace//project-a/src/main.rs".into(),
            arguments: serde_json::json!({"patch_ref": "user-owned://p/1"}),
            tool_id: "fs-connector".into(),
            tool_version: "1.0.0".into(),
            tool_digest: "2".repeat(64),
            side_effect_class: SideEffectClass::W1,
            chain_id: String::new(),
            parent_digest: String::new(),
            attempt: 1,
        }
    }

    #[test]
    fn normalization_is_deterministic_and_canonical() {
        let a = normalize(&proposal()).unwrap();
        let b = normalize(&proposal()).unwrap();
        assert_eq!(a.intent_digest().unwrap(), b.intent_digest().unwrap());
        assert_eq!(a.target, "/workspace/project-a/src/main.rs"); // slashes collapsed
        assert_eq!(a.chain_id, a.action_id); // chain root
    }

    #[test]
    fn traversal_rejected_before_any_scope_check() {
        let mut p = proposal();
        p.target = "/workspace/project-a/../../etc/passwd".into();
        assert!(matches!(normalize(&p), Err(NormalizeError::DotComponent(_))));
        p.target = "workspace/relative".into();
        assert!(matches!(normalize(&p), Err(NormalizeError::RelativePath(_))));
    }

    #[test]
    fn http_targets_canonicalized() {
        let mut p = proposal();
        p.capability = "http.post".into();
        p.target = "HTTPS://Api.Example.COM/v1/pay".into();
        assert_eq!(normalize(&p).unwrap().target, "https://api.example.com/v1/pay");
        p.target = "ftp://x".into();
        assert!(normalize(&p).is_err());
    }

    #[test]
    fn unknown_capability_family_fails_closed() {
        let mut p = proposal();
        p.capability = "browser.click".into();
        assert!(matches!(normalize(&p), Err(NormalizeError::UnsupportedCapability(_))));
    }
}
