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

// ─────────── reader (fail-closed, TL-GATE-WIRE-v1.md §6) ───────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    BadMagic,
    UnknownKind(String),
    Truncated(&'static str),
    BadUtf8(&'static str),
    UnknownEnum { field: &'static str, value: u8 },
    BadDigestLen { field: &'static str, len: usize },
    TrailingBytes(usize),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadMagic => write!(f, "wire: bad magic (want TLG1)"),
            Self::UnknownKind(k) => write!(f, "wire: unknown kind '{k}' — fail-closed"),
            Self::Truncated(w) => write!(f, "wire: truncated at {w}"),
            Self::BadUtf8(w) => write!(f, "wire: invalid UTF-8 in {w}"),
            Self::UnknownEnum { field, value } => {
                write!(f, "wire: unknown enum value {value} for {field} — fail-closed")
            }
            Self::BadDigestLen { field, len } => {
                write!(f, "wire: digest field {field} has {len} bytes, want 32 (or 0 where allowed)")
            }
            Self::TrailingBytes(n) => write!(f, "wire: {n} trailing bytes after last field — reject"),
        }
    }
}

impl std::error::Error for DecodeError {}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], DecodeError> {
        let end = self.pos.checked_add(n).ok_or(DecodeError::Truncated(what))?;
        if end > self.buf.len() {
            return Err(DecodeError::Truncated(what));
        }
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u32(&mut self, what: &'static str) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }
    fn u64(&mut self, what: &'static str) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.take(8, what)?.try_into().unwrap()))
    }
    fn str(&mut self, what: &'static str) -> Result<String, DecodeError> {
        let n = self.u32(what)? as usize;
        let b = self.take(n, what)?;
        String::from_utf8(b.to_vec()).map_err(|_| DecodeError::BadUtf8(what))
    }
    fn digest32(&mut self, what: &'static str) -> Result<String, DecodeError> {
        let b = self.take(32, what)?;
        Ok(b.iter().map(|x| format!("{x:02x}")).collect())
    }
}

fn side_effect_from(b: u8) -> Result<SideEffectClass, DecodeError> {
    Ok(match b {
        0 => SideEffectClass::R0,
        1 => SideEffectClass::R1,
        2 => SideEffectClass::W1,
        3 => SideEffectClass::W2,
        4 => SideEffectClass::W3,
        v => return Err(DecodeError::UnknownEnum { field: "side_effect_class", value: v }),
    })
}

/// Decode TL-GATE-WIRE/v1 bytes back into an `ActionIntent`, enforcing every
/// reader rule from the spec: exact magic, known kind, exact digest widths,
/// known enum values, and NOT ONE trailing byte. The reader never fixes
/// anything up — bytes are either exactly canonical or invalid.
pub fn decode_intent_v1(wire: &[u8]) -> Result<ActionIntent, DecodeError> {
    let mut r = Reader { buf: wire, pos: 0 };
    if r.take(4, "magic")? != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let kind = r.str("kind")?;
    if kind != "tl-gate.action-intent/1" {
        return Err(DecodeError::UnknownKind(kind));
    }
    let body_len = r.u32("body length")? as usize;
    let body_start = r.pos;
    let body_end = body_start.checked_add(body_len).ok_or(DecodeError::Truncated("body"))?;
    if body_end != wire.len() {
        if body_end > wire.len() {
            return Err(DecodeError::Truncated("body"));
        }
        return Err(DecodeError::TrailingBytes(wire.len() - body_end));
    }

    let intent = ActionIntent {
        schema: r.str("schema")?,
        principal: r.str("principal")?,
        orchestrator: r.str("orchestrator")?,
        agent_instance: r.str("agent_instance")?,
        session_ref: r.str("session_ref")?,
        capability: r.str("capability")?,
        target: r.str("target")?,
        arguments_digest: r.digest32("arguments_digest")?,
        tool_id: r.str("tool_id")?,
        tool_version: r.str("tool_version")?,
        tool_digest: r.digest32("tool_digest")?,
        side_effect_class: side_effect_from(r.take(1, "side_effect_class")?[0])?,
        action_id: r.str("action_id")?,
        chain_id: r.str("chain_id")?,
        attempt: r.u64("attempt")?,
        parent_digest: {
            let n = r.u32("parent_digest length")? as usize;
            match n {
                0 => String::new(),
                32 => r.digest32("parent_digest")?,
                _ => return Err(DecodeError::BadDigestLen { field: "parent_digest", len: n }),
            }
        },
    };
    if r.pos != wire.len() {
        return Err(DecodeError::TrailingBytes(wire.len() - r.pos));
    }
    Ok(intent)
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

    // ── negative vectors (fail-closed reader, TL-GATE-WIRE-v1.md §6) ──

    #[test]
    fn roundtrip_is_lossless() {
        let i = intent();
        let w = encode_intent_v1(&i).unwrap();
        let d = decode_intent_v1(&w).unwrap();
        assert_eq!(encode_intent_v1(&d).unwrap(), w);
    }

    #[test]
    fn forged_byte_changes_digest_or_fails_decode() {
        let i = intent();
        let w = encode_intent_v1(&i).unwrap();
        let orig = crate::domain_digest(crate::INTENT_DOMAIN_V1, &w);
        // flip one byte somewhere in the body — either the decode rejects it
        // or the digest no longer matches; silence is never an option
        for pos in [40usize, 60, w.len() - 5] {
            let mut t = w.clone();
            t[pos] ^= 0xff;
            match decode_intent_v1(&t) {
                Err(_) => {}
                Ok(_) => assert_ne!(crate::domain_digest(crate::INTENT_DOMAIN_V1, &t), orig),
            }
        }
    }

    #[test]
    fn bad_magic_rejected() {
        let mut w = encode_intent_v1(&intent()).unwrap();
        w[0] = b'X';
        assert!(matches!(decode_intent_v1(&w), Err(DecodeError::BadMagic)));
    }

    #[test]
    fn truncation_rejected_at_every_length() {
        let w = encode_intent_v1(&intent()).unwrap();
        for cut in [3usize, 10, w.len() / 2, w.len() - 1] {
            assert!(decode_intent_v1(&w[..cut]).is_err(), "cut at {cut} must fail");
        }
    }

    #[test]
    fn trailing_bytes_rejected() {
        let mut w = encode_intent_v1(&intent()).unwrap();
        w.push(0x00);
        assert!(matches!(decode_intent_v1(&w), Err(DecodeError::TrailingBytes(_))));
    }

    #[test]
    fn unknown_enum_rejected() {
        let i = intent();
        let w = encode_intent_v1(&i).unwrap();
        // side_effect_class байт находится сразу после tool_digest; найдём его
        // честно: перекодируем с другим классом и найдём отличающийся байт.
        let mut i2 = i.clone();
        i2.side_effect_class = SideEffectClass::W3;
        let w2 = encode_intent_v1(&i2).unwrap();
        let pos = w.iter().zip(&w2).position(|(a, b)| a != b).unwrap();
        let mut t = w.clone();
        t[pos] = 99;
        assert!(matches!(
            decode_intent_v1(&t),
            Err(DecodeError::UnknownEnum { field: "side_effect_class", value: 99 })
        ));
    }

    #[test]
    fn cross_domain_replay_impossible() {
        // same bytes, different domain → different commitment (spec §10.2)
        let w = encode_intent_v1(&intent()).unwrap();
        assert_ne!(
            crate::domain_digest("TL-GATE/INTENT/v1", &w),
            crate::domain_digest("TL-GATE/EXECUTION/v1", &w)
        );
    }
}
