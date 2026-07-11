//! TL-GATE-WIRE/v1 — the six mandatory receipt kinds (SPEC §10–11).
//!
//! Envelope field order follows §10.1 exactly; payload orders follow the field
//! lists of §11.1–§11.6. `receipt_digest` from §10.1 is NOT part of the wire
//! body — it IS the commitment over these bytes (a hash cannot contain
//! itself); it lives only in the JSON mirror, computed.
//!
//! Extra primitives over the intent encoding (documented in
//! schemas/TL-GATE-WIRE-v1.md §1b): `i64` (8 bytes LE, two's complement),
//! `bool` (u8: 0|1, anything else rejected), object-typed policy fields travel
//! as `str` holding canonical JSON (sorted keys, no spaces) — committed as
//! opaque text in v1.

use serde::{Deserialize, Serialize};

use crate::wire::{DecodeError, WireError, MAGIC};

// ─────────────────────────── kinds and domains ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKind {
    PermissionReceipt,
    ScopeReceipt,
    ToolReceipt,
    ExecutionReceipt,
    ValidationReceipt,
    FinalReceipt,
}

impl ReceiptKind {
    pub fn kind_str(self) -> &'static str {
        match self {
            Self::PermissionReceipt => "tl-gate.permission-receipt/1",
            Self::ScopeReceipt => "tl-gate.scope-receipt/1",
            Self::ToolReceipt => "tl-gate.tool-receipt/1",
            Self::ExecutionReceipt => "tl-gate.execution-receipt/1",
            Self::ValidationReceipt => "tl-gate.validation-receipt/1",
            Self::FinalReceipt => "tl-gate.final-receipt/1",
        }
    }
    /// Domain separator (§10.2) — same bytes can never collide across kinds.
    pub fn domain(self) -> &'static str {
        match self {
            Self::PermissionReceipt => "TL-GATE/PERMISSION/v1",
            Self::ScopeReceipt => "TL-GATE/SCOPE/v1",
            Self::ToolReceipt => "TL-GATE/TOOL/v1",
            Self::ExecutionReceipt => "TL-GATE/EXECUTION/v1",
            Self::ValidationReceipt => "TL-GATE/VALIDATION/v1",
            Self::FinalReceipt => "TL-GATE/FINAL/v1",
        }
    }
    fn from_kind_str(s: &str) -> Option<Self> {
        Some(match s {
            "tl-gate.permission-receipt/1" => Self::PermissionReceipt,
            "tl-gate.scope-receipt/1" => Self::ScopeReceipt,
            "tl-gate.tool-receipt/1" => Self::ToolReceipt,
            "tl-gate.execution-receipt/1" => Self::ExecutionReceipt,
            "tl-gate.validation-receipt/1" => Self::ValidationReceipt,
            "tl-gate.final-receipt/1" => Self::FinalReceipt,
        _ => return None,
        })
    }
}

// ─────────────────────────── envelope (§10.1) ───────────────────────────

/// Common canonical fields of every receipt. Digest-valued fields hold 64
/// lowercase hex in memory (the JSON mirror renders them as `b3:<hex>`);
/// `causal_parent_digest` and `previous_receipt_digest` may be empty for a
/// chain root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReceiptEnvelope {
    pub receipt_id: String,
    pub chain_id: String,
    pub action_id: String,
    pub attempt: u64,
    pub principal_id: String,
    pub agent_instance_id: String,
    pub orchestrator_id: String,
    pub subject_digest: String,
    pub policy_digest: String,
    pub causal_parent_digest: String,
    pub previous_receipt_digest: String,
    pub local_poh_tick: u64,
    /// Informational only — never the sole authorization criterion (P-11),
    /// but committed like every other envelope byte.
    pub wall_clock_hint: String,
    pub nonce: String,
    pub issuer_ref: String,
}

// ─────────────────────────── payloads (§11.1–11.6) ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentBinding {
    ExactIntent,
    ActionTemplate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionPayload {
    pub capability: String,
    pub intent_binding: IntentBinding,
    pub action_template_digest: String, // "" | 64 hex
    pub delegation_parent_digest: String, // "" | 64 hex
    pub revocation_epoch: u64,
    pub max_attempts: u64,
    pub required_validation_policy_digest: String, // 64 hex
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopePayload {
    pub capability: String,
    pub resource_namespace: String,
    pub target_selectors: Vec<String>,
    pub allowed_operations: Vec<String>,
    pub denied_operations: Vec<String>,
    /// Canonical-JSON text (sorted keys) — opaque committed policy in v1.
    pub network_policy: String,
    pub path_policy: String,
    pub data_classification: String,
    pub max_payload: u64,
    pub max_result: u64,
    pub max_attempts: u64,
    pub validity_window: String,
    pub revocation_epoch: u64,
    pub human_approval_requirement: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolPayload {
    pub tool_id: String,
    pub tool_version: String,
    pub binary_or_image_digest: String, // 64 hex
    pub connector_id: String,
    pub connector_version: String,
    pub input_schema_digest: String,  // 64 hex
    pub output_schema_digest: String, // 64 hex
    pub environment_profile_digest: String, // 64 hex
    pub secret_handle_policy: String, // canonical JSON
    pub allowed_endpoints: Vec<String>,
    pub isolation_profile: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPayload {
    pub intent_digest: String,
    pub permission_digest: String,
    pub scope_digest: String,
    pub tool_digest: String,
    pub exact_input_digest: String,
    pub execution_environment_digest: String,
    pub local_poh_start: u64,
    pub local_poh_end: u64,
    pub exit_status: i64,
    pub output_digest: String,
    pub side_effect_digest: String,
    pub connector_attestation_digest: String, // "" | 64 hex
    pub bounded_error_digest: String,         // "" | 64 hex
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorType {
    Deterministic,
    Schema,
    Tests,
    DiffPolicy,
    SecurityScanner,
    ModelJudge,
    Human,
    ExternalService,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationPayload {
    pub validator_id: String,
    pub validator_type: ValidatorType,
    pub validator_version_or_model_digest: String,
    pub validation_policy_digest: String, // 64 hex
    pub input_result_digest: String,      // 64 hex
    pub evidence_digests: Vec<String>,    // each 64 hex
    pub verdict: Verdict,
    pub limitations: String,
    pub human_signer_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinalPayload {
    pub chain_root_digest: String,
    pub intent_digest: String,
    pub permission_digest: String,
    pub scope_digest: String,
    pub tool_digest: String,
    pub execution_digest: String,
    pub validation_digest_set: Vec<String>, // each 64 hex, min 1 (schema-enforced)
    pub final_result_digest: String,
    pub supersedes_digest: String, // "" | 64 hex
    pub local_poh_final_tick: u64,
    pub network_finality_proof_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "receipt_kind", rename_all = "snake_case")]
pub enum Receipt {
    PermissionReceipt { envelope: ReceiptEnvelope, payload: PermissionPayload },
    ScopeReceipt { envelope: ReceiptEnvelope, payload: ScopePayload },
    ToolReceipt { envelope: ReceiptEnvelope, payload: ToolPayload },
    ExecutionReceipt { envelope: ReceiptEnvelope, payload: ExecutionPayload },
    ValidationReceipt { envelope: ReceiptEnvelope, payload: ValidationPayload },
    FinalReceipt { envelope: ReceiptEnvelope, payload: FinalPayload },
}

impl Receipt {
    pub fn kind(&self) -> ReceiptKind {
        match self {
            Self::PermissionReceipt { .. } => ReceiptKind::PermissionReceipt,
            Self::ScopeReceipt { .. } => ReceiptKind::ScopeReceipt,
            Self::ToolReceipt { .. } => ReceiptKind::ToolReceipt,
            Self::ExecutionReceipt { .. } => ReceiptKind::ExecutionReceipt,
            Self::ValidationReceipt { .. } => ReceiptKind::ValidationReceipt,
            Self::FinalReceipt { .. } => ReceiptKind::FinalReceipt,
        }
    }

    pub fn envelope(&self) -> &ReceiptEnvelope {
        match self {
            Self::PermissionReceipt { envelope, .. }
            | Self::ScopeReceipt { envelope, .. }
            | Self::ToolReceipt { envelope, .. }
            | Self::ExecutionReceipt { envelope, .. }
            | Self::ValidationReceipt { envelope, .. }
            | Self::FinalReceipt { envelope, .. } => envelope,
        }
    }

    /// The receipt commitment: BLAKE3-256 over the kind's domain separator
    /// followed by the full wire bytes. This is the §10.1 `receipt_digest` —
    /// computed, never stored inside the hashed bytes.
    pub fn receipt_digest(&self) -> Result<String, WireError> {
        Ok(crate::domain_digest(self.kind().domain(), &self.wire_bytes()?))
    }

    pub fn wire_bytes(&self) -> Result<Vec<u8>, WireError> {
        encode_receipt_v1(self)
    }
}

// ─────────────────────────── encoder ───────────────────────────

struct W(Vec<u8>);

impl W {
    fn s(&mut self, v: &str) {
        self.0.extend_from_slice(&(v.len() as u32).to_le_bytes());
        self.0.extend_from_slice(v.as_bytes());
    }
    fn u64v(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i64v(&mut self, v: i64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn boolean(&mut self, v: bool) {
        self.0.push(v as u8);
    }
    fn enum8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn digest(&mut self, field: &'static str, v: &str) -> Result<(), WireError> {
        self.0.extend_from_slice(&hex32(field, v)?);
        Ok(())
    }
    fn digest_opt(&mut self, field: &'static str, v: &str) -> Result<(), WireError> {
        if v.is_empty() {
            self.0.extend_from_slice(&0u32.to_le_bytes());
        } else {
            self.0.extend_from_slice(&32u32.to_le_bytes());
            self.0.extend_from_slice(&hex32(field, v)?);
        }
        Ok(())
    }
    fn str_list(&mut self, v: &[String]) {
        self.0.extend_from_slice(&(v.len() as u32).to_le_bytes());
        for s in v {
            self.s(s);
        }
    }
    fn digest_list(&mut self, field: &'static str, v: &[String]) -> Result<(), WireError> {
        self.0.extend_from_slice(&(v.len() as u32).to_le_bytes());
        for d in v {
            self.digest(field, d)?;
        }
        Ok(())
    }
}

fn hex32(field: &'static str, s: &str) -> Result<[u8; 32], WireError> {
    let bad = || WireError::BadDigestField { field, got: s.to_string() };
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

fn encode_envelope(w: &mut W, e: &ReceiptEnvelope) -> Result<(), WireError> {
    w.s(&e.receipt_id);
    w.s(&e.chain_id);
    w.s(&e.action_id);
    w.u64v(e.attempt);
    w.s(&e.principal_id);
    w.s(&e.agent_instance_id);
    w.s(&e.orchestrator_id);
    w.digest("subject_digest", &e.subject_digest)?;
    w.digest("policy_digest", &e.policy_digest)?;
    w.digest_opt("causal_parent_digest", &e.causal_parent_digest)?;
    w.digest_opt("previous_receipt_digest", &e.previous_receipt_digest)?;
    w.u64v(e.local_poh_tick);
    w.s(&e.wall_clock_hint);
    w.s(&e.nonce);
    w.s(&e.issuer_ref);
    Ok(())
}

pub fn encode_receipt_v1(r: &Receipt) -> Result<Vec<u8>, WireError> {
    let mut b = W(Vec::with_capacity(1024));
    encode_envelope(&mut b, r.envelope())?;
    match r {
        Receipt::PermissionReceipt { payload: p, .. } => {
            b.s(&p.capability);
            b.enum8(match p.intent_binding {
                IntentBinding::ExactIntent => 0,
                IntentBinding::ActionTemplate => 1,
            });
            b.digest_opt("action_template_digest", &p.action_template_digest)?;
            b.digest_opt("delegation_parent_digest", &p.delegation_parent_digest)?;
            b.u64v(p.revocation_epoch);
            b.u64v(p.max_attempts);
            b.digest("required_validation_policy_digest", &p.required_validation_policy_digest)?;
        }
        Receipt::ScopeReceipt { payload: p, .. } => {
            b.s(&p.capability);
            b.s(&p.resource_namespace);
            b.str_list(&p.target_selectors);
            b.str_list(&p.allowed_operations);
            b.str_list(&p.denied_operations);
            b.s(&p.network_policy);
            b.s(&p.path_policy);
            b.s(&p.data_classification);
            b.u64v(p.max_payload);
            b.u64v(p.max_result);
            b.u64v(p.max_attempts);
            b.s(&p.validity_window);
            b.u64v(p.revocation_epoch);
            b.boolean(p.human_approval_requirement);
        }
        Receipt::ToolReceipt { payload: p, .. } => {
            b.s(&p.tool_id);
            b.s(&p.tool_version);
            b.digest("binary_or_image_digest", &p.binary_or_image_digest)?;
            b.s(&p.connector_id);
            b.s(&p.connector_version);
            b.digest("input_schema_digest", &p.input_schema_digest)?;
            b.digest("output_schema_digest", &p.output_schema_digest)?;
            b.digest("environment_profile_digest", &p.environment_profile_digest)?;
            b.s(&p.secret_handle_policy);
            b.str_list(&p.allowed_endpoints);
            b.s(&p.isolation_profile);
        }
        Receipt::ExecutionReceipt { payload: p, .. } => {
            b.digest("intent_digest", &p.intent_digest)?;
            b.digest("permission_digest", &p.permission_digest)?;
            b.digest("scope_digest", &p.scope_digest)?;
            b.digest("tool_digest", &p.tool_digest)?;
            b.digest("exact_input_digest", &p.exact_input_digest)?;
            b.digest("execution_environment_digest", &p.execution_environment_digest)?;
            b.u64v(p.local_poh_start);
            b.u64v(p.local_poh_end);
            b.i64v(p.exit_status);
            b.digest("output_digest", &p.output_digest)?;
            b.digest("side_effect_digest", &p.side_effect_digest)?;
            b.digest_opt("connector_attestation_digest", &p.connector_attestation_digest)?;
            b.digest_opt("bounded_error_digest", &p.bounded_error_digest)?;
        }
        Receipt::ValidationReceipt { payload: p, .. } => {
            b.s(&p.validator_id);
            b.enum8(match p.validator_type {
                ValidatorType::Deterministic => 0,
                ValidatorType::Schema => 1,
                ValidatorType::Tests => 2,
                ValidatorType::DiffPolicy => 3,
                ValidatorType::SecurityScanner => 4,
                ValidatorType::ModelJudge => 5,
                ValidatorType::Human => 6,
                ValidatorType::ExternalService => 7,
            });
            b.s(&p.validator_version_or_model_digest);
            b.digest("validation_policy_digest", &p.validation_policy_digest)?;
            b.digest("input_result_digest", &p.input_result_digest)?;
            b.digest_list("evidence_digests", &p.evidence_digests)?;
            b.enum8(match p.verdict {
                Verdict::Pass => 0,
                Verdict::Fail => 1,
                Verdict::Inconclusive => 2,
            });
            b.s(&p.limitations);
            b.s(&p.human_signer_ref);
        }
        Receipt::FinalReceipt { payload: p, .. } => {
            b.digest("chain_root_digest", &p.chain_root_digest)?;
            b.digest("intent_digest", &p.intent_digest)?;
            b.digest("permission_digest", &p.permission_digest)?;
            b.digest("scope_digest", &p.scope_digest)?;
            b.digest("tool_digest", &p.tool_digest)?;
            b.digest("execution_digest", &p.execution_digest)?;
            b.digest_list("validation_digest_set", &p.validation_digest_set)?;
            b.digest("final_result_digest", &p.final_result_digest)?;
            b.digest_opt("supersedes_digest", &p.supersedes_digest)?;
            b.u64v(p.local_poh_final_tick);
            b.s(&p.network_finality_proof_ref);
        }
    }

    let body = b.0;
    let mut out = W(Vec::with_capacity(body.len() + 64));
    out.0.extend_from_slice(MAGIC);
    out.s(r.kind().kind_str());
    out.0.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.0.extend_from_slice(&body);
    Ok(out.0)
}

// ─────────────────────────── decoder (fail-closed) ───────────────────────────

struct R<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> R<'a> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], DecodeError> {
        let end = self.pos.checked_add(n).ok_or(DecodeError::Truncated(what))?;
        if end > self.buf.len() {
            return Err(DecodeError::Truncated(what));
        }
        let s = &self.buf[self.pos..end];
        self.pos = end;
        Ok(s)
    }
    fn u32v(&mut self, what: &'static str) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }
    fn u64v(&mut self, what: &'static str) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.take(8, what)?.try_into().unwrap()))
    }
    fn i64v(&mut self, what: &'static str) -> Result<i64, DecodeError> {
        Ok(i64::from_le_bytes(self.take(8, what)?.try_into().unwrap()))
    }
    fn boolean(&mut self, what: &'static str) -> Result<bool, DecodeError> {
        match self.take(1, what)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(DecodeError::UnknownEnum { field: what, value: v }),
        }
    }
    fn s(&mut self, what: &'static str) -> Result<String, DecodeError> {
        let n = self.u32v(what)? as usize;
        String::from_utf8(self.take(n, what)?.to_vec()).map_err(|_| DecodeError::BadUtf8(what))
    }
    fn digest(&mut self, what: &'static str) -> Result<String, DecodeError> {
        Ok(self.take(32, what)?.iter().map(|x| format!("{x:02x}")).collect())
    }
    fn digest_opt(&mut self, what: &'static str) -> Result<String, DecodeError> {
        match self.u32v(what)? as usize {
            0 => Ok(String::new()),
            32 => self.digest(what),
            n => Err(DecodeError::BadDigestLen { field: what, len: n }),
        }
    }
    fn str_list(&mut self, what: &'static str) -> Result<Vec<String>, DecodeError> {
        let n = self.u32v(what)? as usize;
        (0..n).map(|_| self.s(what)).collect()
    }
    fn digest_list(&mut self, what: &'static str) -> Result<Vec<String>, DecodeError> {
        let n = self.u32v(what)? as usize;
        (0..n).map(|_| self.digest(what)).collect()
    }
}

fn decode_envelope(r: &mut R) -> Result<ReceiptEnvelope, DecodeError> {
    Ok(ReceiptEnvelope {
        receipt_id: r.s("receipt_id")?,
        chain_id: r.s("chain_id")?,
        action_id: r.s("action_id")?,
        attempt: r.u64v("attempt")?,
        principal_id: r.s("principal_id")?,
        agent_instance_id: r.s("agent_instance_id")?,
        orchestrator_id: r.s("orchestrator_id")?,
        subject_digest: r.digest("subject_digest")?,
        policy_digest: r.digest("policy_digest")?,
        causal_parent_digest: r.digest_opt("causal_parent_digest")?,
        previous_receipt_digest: r.digest_opt("previous_receipt_digest")?,
        local_poh_tick: r.u64v("local_poh_tick")?,
        wall_clock_hint: r.s("wall_clock_hint")?,
        nonce: r.s("nonce")?,
        issuer_ref: r.s("issuer_ref")?,
    })
}

pub fn decode_receipt_v1(wire: &[u8]) -> Result<Receipt, DecodeError> {
    let mut r = R { buf: wire, pos: 0 };
    if r.take(4, "magic")? != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let kind_str = r.s("kind")?;
    let kind =
        ReceiptKind::from_kind_str(&kind_str).ok_or(DecodeError::UnknownKind(kind_str))?;
    let body_len = r.u32v("body length")? as usize;
    let expected_end = r.pos.checked_add(body_len).ok_or(DecodeError::Truncated("body"))?;
    if expected_end > wire.len() {
        return Err(DecodeError::Truncated("body"));
    }
    if expected_end < wire.len() {
        return Err(DecodeError::TrailingBytes(wire.len() - expected_end));
    }

    let envelope = decode_envelope(&mut r)?;
    let receipt = match kind {
        ReceiptKind::PermissionReceipt => Receipt::PermissionReceipt {
            envelope,
            payload: PermissionPayload {
                capability: r.s("capability")?,
                intent_binding: match r.take(1, "intent_binding")?[0] {
                    0 => IntentBinding::ExactIntent,
                    1 => IntentBinding::ActionTemplate,
                    v => return Err(DecodeError::UnknownEnum { field: "intent_binding", value: v }),
                },
                action_template_digest: r.digest_opt("action_template_digest")?,
                delegation_parent_digest: r.digest_opt("delegation_parent_digest")?,
                revocation_epoch: r.u64v("revocation_epoch")?,
                max_attempts: r.u64v("max_attempts")?,
                required_validation_policy_digest: r.digest("required_validation_policy_digest")?,
            },
        },
        ReceiptKind::ScopeReceipt => Receipt::ScopeReceipt {
            envelope,
            payload: ScopePayload {
                capability: r.s("capability")?,
                resource_namespace: r.s("resource_namespace")?,
                target_selectors: r.str_list("target_selectors")?,
                allowed_operations: r.str_list("allowed_operations")?,
                denied_operations: r.str_list("denied_operations")?,
                network_policy: r.s("network_policy")?,
                path_policy: r.s("path_policy")?,
                data_classification: r.s("data_classification")?,
                max_payload: r.u64v("max_payload")?,
                max_result: r.u64v("max_result")?,
                max_attempts: r.u64v("max_attempts")?,
                validity_window: r.s("validity_window")?,
                revocation_epoch: r.u64v("revocation_epoch")?,
                human_approval_requirement: r.boolean("human_approval_requirement")?,
            },
        },
        ReceiptKind::ToolReceipt => Receipt::ToolReceipt {
            envelope,
            payload: ToolPayload {
                tool_id: r.s("tool_id")?,
                tool_version: r.s("tool_version")?,
                binary_or_image_digest: r.digest("binary_or_image_digest")?,
                connector_id: r.s("connector_id")?,
                connector_version: r.s("connector_version")?,
                input_schema_digest: r.digest("input_schema_digest")?,
                output_schema_digest: r.digest("output_schema_digest")?,
                environment_profile_digest: r.digest("environment_profile_digest")?,
                secret_handle_policy: r.s("secret_handle_policy")?,
                allowed_endpoints: r.str_list("allowed_endpoints")?,
                isolation_profile: r.s("isolation_profile")?,
            },
        },
        ReceiptKind::ExecutionReceipt => Receipt::ExecutionReceipt {
            envelope,
            payload: ExecutionPayload {
                intent_digest: r.digest("intent_digest")?,
                permission_digest: r.digest("permission_digest")?,
                scope_digest: r.digest("scope_digest")?,
                tool_digest: r.digest("tool_digest")?,
                exact_input_digest: r.digest("exact_input_digest")?,
                execution_environment_digest: r.digest("execution_environment_digest")?,
                local_poh_start: r.u64v("local_poh_start")?,
                local_poh_end: r.u64v("local_poh_end")?,
                exit_status: r.i64v("exit_status")?,
                output_digest: r.digest("output_digest")?,
                side_effect_digest: r.digest("side_effect_digest")?,
                connector_attestation_digest: r.digest_opt("connector_attestation_digest")?,
                bounded_error_digest: r.digest_opt("bounded_error_digest")?,
            },
        },
        ReceiptKind::ValidationReceipt => Receipt::ValidationReceipt {
            envelope,
            payload: ValidationPayload {
                validator_id: r.s("validator_id")?,
                validator_type: match r.take(1, "validator_type")?[0] {
                    0 => ValidatorType::Deterministic,
                    1 => ValidatorType::Schema,
                    2 => ValidatorType::Tests,
                    3 => ValidatorType::DiffPolicy,
                    4 => ValidatorType::SecurityScanner,
                    5 => ValidatorType::ModelJudge,
                    6 => ValidatorType::Human,
                    7 => ValidatorType::ExternalService,
                    v => return Err(DecodeError::UnknownEnum { field: "validator_type", value: v }),
                },
                validator_version_or_model_digest: r.s("validator_version_or_model_digest")?,
                validation_policy_digest: r.digest("validation_policy_digest")?,
                input_result_digest: r.digest("input_result_digest")?,
                evidence_digests: r.digest_list("evidence_digests")?,
                verdict: match r.take(1, "verdict")?[0] {
                    0 => Verdict::Pass,
                    1 => Verdict::Fail,
                    2 => Verdict::Inconclusive,
                    v => return Err(DecodeError::UnknownEnum { field: "verdict", value: v }),
                },
                limitations: r.s("limitations")?,
                human_signer_ref: r.s("human_signer_ref")?,
            },
        },
        ReceiptKind::FinalReceipt => Receipt::FinalReceipt {
            envelope,
            payload: FinalPayload {
                chain_root_digest: r.digest("chain_root_digest")?,
                intent_digest: r.digest("intent_digest")?,
                permission_digest: r.digest("permission_digest")?,
                scope_digest: r.digest("scope_digest")?,
                tool_digest: r.digest("tool_digest")?,
                execution_digest: r.digest("execution_digest")?,
                validation_digest_set: r.digest_list("validation_digest_set")?,
                final_result_digest: r.digest("final_result_digest")?,
                supersedes_digest: r.digest_opt("supersedes_digest")?,
                local_poh_final_tick: r.u64v("local_poh_final_tick")?,
                network_finality_proof_ref: r.s("network_finality_proof_ref")?,
            },
        },
    };
    if r.pos != wire.len() {
        return Err(DecodeError::TrailingBytes(wire.len() - r.pos));
    }
    Ok(receipt)
}

// ─────────────────────────── tests ───────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> ReceiptEnvelope {
        ReceiptEnvelope {
            receipt_id: "r-1".into(),
            chain_id: "c-1".into(),
            action_id: "a-1".into(),
            attempt: 1,
            principal_id: "user:owner".into(),
            agent_instance_id: "agent:demo#1".into(),
            orchestrator_id: "orch:generic".into(),
            subject_digest: "1".repeat(64),
            policy_digest: "2".repeat(64),
            causal_parent_digest: String::new(),
            previous_receipt_digest: String::new(),
            local_poh_tick: 1042,
            wall_clock_hint: "2026-07-11T00:00:00Z".into(),
            nonce: "n-1".into(),
            issuer_ref: "issuer:cabinet".into(),
        }
    }

    fn all_kinds() -> Vec<Receipt> {
        vec![
            Receipt::PermissionReceipt {
                envelope: envelope(),
                payload: PermissionPayload {
                    capability: "filesystem.write".into(),
                    intent_binding: IntentBinding::ExactIntent,
                    action_template_digest: String::new(),
                    delegation_parent_digest: String::new(),
                    revocation_epoch: 2,
                    max_attempts: 3,
                    required_validation_policy_digest: "3".repeat(64),
                },
            },
            Receipt::ScopeReceipt {
                envelope: envelope(),
                payload: ScopePayload {
                    capability: "filesystem.write".into(),
                    resource_namespace: "workspace".into(),
                    target_selectors: vec!["/workspace/project-a/src/**".into()],
                    allowed_operations: vec!["write".into(), "create".into()],
                    denied_operations: vec!["delete".into()],
                    network_policy: r#"{"egress":"deny"}"#.into(),
                    path_policy: r#"{"follow_symlinks":false}"#.into(),
                    data_classification: "internal".into(),
                    max_payload: 1048576,
                    max_result: 1048576,
                    max_attempts: 3,
                    validity_window: r#"{"not_before":"","expires_at":""}"#.into(),
                    revocation_epoch: 2,
                    human_approval_requirement: false,
                },
            },
            Receipt::ToolReceipt {
                envelope: envelope(),
                payload: ToolPayload {
                    tool_id: "fs-connector".into(),
                    tool_version: "1.0.0".into(),
                    binary_or_image_digest: "4".repeat(64),
                    connector_id: "filesystem".into(),
                    connector_version: "1".into(),
                    input_schema_digest: "5".repeat(64),
                    output_schema_digest: "6".repeat(64),
                    environment_profile_digest: "7".repeat(64),
                    secret_handle_policy: r#"{"handles":[]}"#.into(),
                    allowed_endpoints: vec![],
                    isolation_profile: "cooperative".into(),
                },
            },
            Receipt::ExecutionReceipt {
                envelope: envelope(),
                payload: ExecutionPayload {
                    intent_digest: "8".repeat(64),
                    permission_digest: "9".repeat(64),
                    scope_digest: "a".repeat(64),
                    tool_digest: "b".repeat(64),
                    exact_input_digest: "c".repeat(64),
                    execution_environment_digest: "d".repeat(64),
                    local_poh_start: 100,
                    local_poh_end: 108,
                    exit_status: 0,
                    output_digest: "e".repeat(64),
                    side_effect_digest: "f".repeat(64),
                    connector_attestation_digest: String::new(),
                    bounded_error_digest: String::new(),
                },
            },
            Receipt::ValidationReceipt {
                envelope: envelope(),
                payload: ValidationPayload {
                    validator_id: "judge-1".into(),
                    validator_type: ValidatorType::ModelJudge,
                    validator_version_or_model_digest: "model:decorrelated".into(),
                    validation_policy_digest: "1".repeat(64),
                    input_result_digest: "e".repeat(64),
                    evidence_digests: vec!["2".repeat(64)],
                    verdict: Verdict::Pass,
                    limitations: "model judge, k=5 repeated sampling".into(),
                    human_signer_ref: String::new(),
                },
            },
            Receipt::FinalReceipt {
                envelope: envelope(),
                payload: FinalPayload {
                    chain_root_digest: "1".repeat(64),
                    intent_digest: "8".repeat(64),
                    permission_digest: "9".repeat(64),
                    scope_digest: "a".repeat(64),
                    tool_digest: "b".repeat(64),
                    execution_digest: "c".repeat(64),
                    validation_digest_set: vec!["d".repeat(64), "e".repeat(64)],
                    final_result_digest: "f".repeat(64),
                    supersedes_digest: String::new(),
                    local_poh_final_tick: 110,
                    network_finality_proof_ref: "tl:final/abc".into(),
                },
            },
        ]
    }

    #[test]
    fn roundtrip_all_six_kinds() {
        for r in all_kinds() {
            let w = r.wire_bytes().unwrap();
            let d = decode_receipt_v1(&w).unwrap();
            assert_eq!(d, r, "{:?}", r.kind());
            assert_eq!(d.wire_bytes().unwrap(), w);
        }
    }

    #[test]
    fn six_kinds_have_six_domains_and_distinct_digests() {
        let rs = all_kinds();
        let mut digests: Vec<String> = rs.iter().map(|r| r.receipt_digest().unwrap()).collect();
        digests.sort();
        digests.dedup();
        assert_eq!(digests.len(), 6, "no two kinds may share a commitment");
    }

    #[test]
    fn kind_substitution_is_impossible() {
        // §10.3: one kind can never substitute for another. Same envelope,
        // same subject — but a scope receipt's bytes hashed under the
        // permission domain never equal a permission commitment.
        let rs = all_kinds();
        let scope_wire = rs[1].wire_bytes().unwrap();
        let perm_digest = rs[0].receipt_digest().unwrap();
        assert_ne!(
            crate::domain_digest(ReceiptKind::PermissionReceipt.domain(), &scope_wire),
            perm_digest
        );
        // and the decoder refuses to read one kind as another: the kind is in
        // the wire itself, so substitution requires forging bytes, which
        // changes the digest.
        let decoded = decode_receipt_v1(&scope_wire).unwrap();
        assert_eq!(decoded.kind(), ReceiptKind::ScopeReceipt);
    }

    #[test]
    fn tamper_and_truncation_fail_closed() {
        for r in all_kinds() {
            let w = r.wire_bytes().unwrap();
            let orig = r.receipt_digest().unwrap();
            let mut t = w.clone();
            let mid = t.len() / 2;
            t[mid] ^= 0xff;
            match decode_receipt_v1(&t) {
                Err(_) => {}
                Ok(d) => assert_ne!(d.receipt_digest().unwrap(), orig),
            }
            assert!(decode_receipt_v1(&w[..w.len() - 1]).is_err());
            let mut trail = w.clone();
            trail.push(0);
            assert!(matches!(decode_receipt_v1(&trail), Err(DecodeError::TrailingBytes(_))));
        }
    }

    #[test]
    fn bad_bool_and_bad_enum_rejected() {
        // scope: human_approval_requirement is the LAST body byte
        let scope = &all_kinds()[1];
        let mut w = scope.wire_bytes().unwrap();
        let last = w.len() - 1;
        w[last] = 7;
        assert!(matches!(
            decode_receipt_v1(&w),
            Err(DecodeError::UnknownEnum { field: "human_approval_requirement", value: 7 })
        ));
    }
}
