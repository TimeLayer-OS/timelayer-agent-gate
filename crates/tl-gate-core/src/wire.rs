//! TL-GATE-WIRE/v1 — the canonical binary form (schemas/TL-GATE-WIRE-v1.md).
//!
//! Only these bytes are ever hashed. The JSON form is a human-readable mirror
//! and is never authoritative. Frozen 2026-07-11: changes mean /v2.

use crate::{ActionIntent, SideEffectClass};

pub const MAGIC: &[u8; 4] = b"TLG1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// A digest field must be empty (where allowed) or exactly 64 lowercase hex.
    BadDigestField { field: &'static str, got: String },
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadDigestField { field, got } => write!(
                f,
                "wire: field '{field}' must be 64 lowercase hex chars, got '{got}'"
            ),
        }
    }
}

impl std::error::Error for WireError {}

fn put_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn put_bytes(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}

fn hex32(field: &'static str, s: &str) -> Result<[u8; 32], WireError> {
    let bad = || WireError::BadDigestField {
        field,
        got: s.to_string(),
    };
    if s.len() != 64 || !s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(bad());
    }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).ok_or_else(bad)? as u8;
        let lo = (chunk[1] as char).to_digit(16).ok_or_else(bad)? as u8;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn side_effect_byte(c: SideEffectClass) -> u8 {
    match c {
        SideEffectClass::R0 => 0,
        SideEffectClass::R1 => 1,
        SideEffectClass::W1 => 2,
        SideEffectClass::W2 => 3,
        SideEffectClass::W3 => 4,
    }
}

/// Encode an `ActionIntent` as TL-GATE-WIRE/v1 bytes (magic + kind + body).
/// Field order is frozen — see schemas/TL-GATE-WIRE-v1.md §3.
pub fn encode_intent_v1(i: &ActionIntent) -> Result<Vec<u8>, WireError> {
    let mut body = Vec::with_capacity(512);
    put_str(&mut body, &i.schema);
    put_str(&mut body, &i.principal);
    put_str(&mut body, &i.orchestrator);
    put_str(&mut body, &i.agent_instance);
    put_str(&mut body, &i.session_ref);
    put_str(&mut body, &i.capability);
    put_str(&mut body, &i.target);
    body.extend_from_slice(&hex32("arguments_digest", &i.arguments_digest)?);
    put_str(&mut body, &i.tool_id);
    put_str(&mut body, &i.tool_version);
    body.extend_from_slice(&hex32("tool_digest", &i.tool_digest)?);
    body.push(side_effect_byte(i.side_effect_class));
    put_str(&mut body, &i.action_id);
    put_str(&mut body, &i.chain_id);
    body.extend_from_slice(&i.attempt.to_le_bytes());
    if i.parent_digest.is_empty() {
        put_bytes(&mut body, &[]);
    } else {
        put_bytes(&mut body, &hex32("parent_digest", &i.parent_digest)?);
    }

    let mut out = Vec::with_capacity(body.len() + 64);
    out.extend_from_slice(MAGIC);
    put_str(&mut out, "tl-gate.action-intent/1");
    put_bytes(&mut out, &body);
    Ok(out)
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
            session_ref: "s1".into(),
            capability: "filesystem.write".into(),
            target: "/workspace/demo.txt".into(),
            arguments_digest: "1".repeat(64),
            tool_id: "fs".into(),
            tool_version: "1.0.0".into(),
            tool_digest: "2".repeat(64),
            side_effect_class: SideEffectClass::W1,
            action_id: "a1".into(),
            chain_id: "c1".into(),
            attempt: 1,
            parent_digest: String::new(),
        }
    }

    #[test]
    fn wire_starts_with_magic_and_kind() {
        let w = encode_intent_v1(&intent()).unwrap();
        assert_eq!(&w[..4], MAGIC);
        assert_eq!(&w[8..8 + 23], b"tl-gate.action-intent/1");
    }

    #[test]
    fn wire_is_deterministic_and_field_sensitive() {
        let a = encode_intent_v1(&intent()).unwrap();
        assert_eq!(a, encode_intent_v1(&intent()).unwrap());
        let mut i2 = intent();
        i2.attempt = 2;
        assert_ne!(a, encode_intent_v1(&i2).unwrap());
    }

    #[test]
    fn uppercase_hex_is_rejected() {
        let mut i = intent();
        i.tool_digest = "A".repeat(64);
        assert!(encode_intent_v1(&i).is_err(), "hex must be lowercase (fail-closed)");
    }

    #[test]
    fn short_digest_is_rejected() {
        let mut i = intent();
        i.arguments_digest = "abc".into();
        assert!(encode_intent_v1(&i).is_err());
    }
}
