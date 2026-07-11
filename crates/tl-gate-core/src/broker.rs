//! Controlled Tool Broker (SPEC §8.8) + Result Capture (§8.9).
//!
//! The ONLY place a real side effect happens. Nothing here runs until the
//! Pre-Execution Gate returned ALLOW for this exact intent; the broker then
//! re-checks the intent digest, executes through one allow-listed connector,
//! captures exact input/output digests, and produces an `execution_receipt`
//! (§11.4). The broker never inherits the ambient environment and never lets
//! an adapter perform the effect itself.
//!
//! Phase 1 slice 3: the filesystem connector (W1 reversible write). Process
//! and HTTP connectors follow the same trait; each is a separate, auditable
//! unit.

use std::path::{Path, PathBuf};

use crate::gate::{pre_gate, GateInput, ReceiptVerifier};
use crate::receipts::{Receipt, ReceiptEnvelope, ReceiptKind};
use crate::{domain_digest, ActionIntent, GateDecision, StopCode};

pub const INPUT_DOMAIN_V1: &str = "TL-GATE/INPUT/v1";
pub const OUTPUT_DOMAIN_V1: &str = "TL-GATE/OUTPUT/v1";
pub const EFFECT_DOMAIN_V1: &str = "TL-GATE/EFFECT/v1";
pub const ENV_DOMAIN_V1: &str = "TL-GATE/ENV/v1";

/// A captured, immutable execution evidence package (§8.9). Digests only —
/// the bytes themselves may stay in user-owned storage (TB-06).
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionEvidence {
    pub intent_digest: String,
    pub exact_input_digest: String,
    pub execution_environment_digest: String,
    pub exit_status: i64,
    pub output_digest: String,
    pub side_effect_digest: String,
    /// created / modified / deleted object identifiers, structured summary.
    pub side_effect_summary: SideEffectSummary,
    pub local_poh_start: u64,
    pub local_poh_end: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SideEffectSummary {
    pub created: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerError {
    /// The gate did not (re)confirm ALLOW for this intent.
    NotAllowed(StopCode),
    /// The connector class has no implementation here — fail-closed, never a
    /// silent no-op that pretends the effect happened.
    NoConnector(String),
    /// The effect started but its completion is uncertain (§20.2
    /// EFFECT_UNKNOWN) — requires explicit reconciliation, never silent retry.
    EffectUnknown(String),
    /// A pre-effect invariant of the connector was violated (e.g. the target
    /// escaped its scope once symlinks were resolved on the live fs).
    Refused(String),
    Io(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAllowed(c) => write!(f, "broker: gate re-check returned STOP({c}) — no effect"),
            Self::NoConnector(c) => write!(f, "broker: no connector for '{c}' — fail-closed"),
            Self::EffectUnknown(m) => write!(f, "broker: EFFECT_UNKNOWN — {m}"),
            Self::Refused(m) => write!(f, "broker: refused before effect — {m}"),
            Self::Io(m) => write!(f, "broker: io — {m}"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// The exact input a connector will act on, resolved by the caller from the
/// (user-owned) arguments. The broker digests these bytes as the authoritative
/// `exact_input_digest` — no connector may act on anything else.
pub struct ExecInput {
    pub bytes: Vec<u8>,
}

/// One connector class (§8.8). Implementations perform the real effect and
/// return evidence; they must be reversible-safe for their side-effect class
/// and must never read ambient env or secrets they were not handed.
pub trait Connector {
    fn class(&self) -> &'static str;
    /// Perform the effect for `intent` with `input`. `poh` is the LocalPoH
    /// tick at broker entry (§8.9 start position). Returns (evidence).
    fn execute(
        &self,
        intent: &ActionIntent,
        input: &ExecInput,
        poh_start: u64,
    ) -> Result<ExecutionEvidence, BrokerError>;
}

/// Filesystem connector: writes `input.bytes` to `intent.target`, capturing
/// whether the target was created or modified. Reversible (W1): it records the
/// prior state digest so a revert is possible; it refuses to follow a symlink
/// out of the target's own directory (live-fs scope escape, §9.4).
pub struct FilesystemConnector {
    /// Environment profile digest the tool receipt admitted — captured into
    /// evidence, never derived from the ambient process environment.
    pub environment_profile_digest: String,
}

impl Connector for FilesystemConnector {
    fn class(&self) -> &'static str {
        "filesystem"
    }

    fn execute(
        &self,
        intent: &ActionIntent,
        input: &ExecInput,
        poh_start: u64,
    ) -> Result<ExecutionEvidence, BrokerError> {
        let target = PathBuf::from(&intent.target);

        // Live-fs scope escape: if the target (or its parent) is a symlink
        // pointing elsewhere, the lexical scope check in the gate was not
        // enough. Refuse before any write (§9.4, §22.1).
        if let Some(parent) = target.parent() {
            if let Ok(canon_parent) = std::fs::canonicalize(parent) {
                let expected = lexical_parent(&target);
                if canon_parent != expected && parent.exists() {
                    return Err(BrokerError::Refused(format!(
                        "target parent resolves to {} (symlink escape), lexical parent is {}",
                        canon_parent.display(),
                        expected.display()
                    )));
                }
            }
        }
        if target.is_symlink() {
            return Err(BrokerError::Refused("target itself is a symlink — refusing W1 write".into()));
        }

        let existed = target.exists();
        let prior = if existed {
            std::fs::read(&target).map_err(|e| BrokerError::Io(e.to_string()))?
        } else {
            Vec::new()
        };

        // The effect. From here to the poh_end capture, a crash means
        // EFFECT_UNKNOWN — the caller must reconcile, never silently retry.
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| BrokerError::Io(e.to_string()))?;
        }
        std::fs::write(&target, &input.bytes).map_err(|e| BrokerError::Io(e.to_string()))?;

        let summary = if existed {
            SideEffectSummary { modified: vec![intent.target.clone()], ..Default::default() }
        } else {
            SideEffectSummary { created: vec![intent.target.clone()], ..Default::default() }
        };
        let effect_material = format!(
            "fs\x00{}\x00prior={}\x00now={}",
            intent.target,
            domain_digest(OUTPUT_DOMAIN_V1, &prior),
            domain_digest(OUTPUT_DOMAIN_V1, &input.bytes)
        );

        Ok(ExecutionEvidence {
            intent_digest: intent.intent_digest().map_err(|e| BrokerError::Io(e.to_string()))?,
            exact_input_digest: domain_digest(INPUT_DOMAIN_V1, &input.bytes),
            execution_environment_digest: self.environment_profile_digest.clone(),
            exit_status: 0,
            output_digest: domain_digest(OUTPUT_DOMAIN_V1, &input.bytes),
            side_effect_digest: domain_digest(EFFECT_DOMAIN_V1, effect_material.as_bytes()),
            side_effect_summary: summary,
            local_poh_start: poh_start,
            local_poh_end: poh_start + 1,
        })
    }
}

fn lexical_parent(target: &Path) -> PathBuf {
    target.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("/"))
}

/// The broker entry point (§8.8). Re-checks the gate for THIS intent, then
/// executes through the connector, then builds the `execution_receipt` bound
/// to the chain. Nothing runs if the re-check is not ALLOW.
#[allow(clippy::too_many_arguments)]
pub fn broker_execute(
    intent: &ActionIntent,
    gate_input: &GateInput,
    verifier: &dyn ReceiptVerifier,
    connector: &dyn Connector,
    input: &ExecInput,
    scope_digest: &str,
    permission_digest: &str,
    tool_digest_receipt: &str,
    poh_start: u64,
    envelope_template: ReceiptEnvelope,
) -> Result<Receipt, BrokerError> {
    // 1. Re-run the gate. The broker does not trust a caller's word that the
    //    action was allowed (§8.8: "re-check digest intent before execution").
    match pre_gate(intent, gate_input, verifier) {
        GateDecision::Allow => {}
        GateDecision::Stop(code) => return Err(BrokerError::NotAllowed(code)),
    }

    // 2. The connector class must match the capability family.
    let family = intent.capability.split('.').next().unwrap_or("");
    if connector.class() != family {
        return Err(BrokerError::NoConnector(family.to_string()));
    }

    // 3. The one real side effect.
    let ev = connector.execute(intent, input, poh_start)?;

    // 4. Result Capture → execution_receipt (§11.4), chained after the tool
    //    receipt via previous_receipt_digest (set by the caller in the
    //    envelope template: it points at the tool receipt digest).
    let payload = crate::receipts::ExecutionPayload {
        intent_digest: ev.intent_digest.clone(),
        permission_digest: permission_digest.to_string(),
        scope_digest: scope_digest.to_string(),
        tool_digest: tool_digest_receipt.to_string(),
        exact_input_digest: ev.exact_input_digest.clone(),
        execution_environment_digest: ev.execution_environment_digest.clone(),
        local_poh_start: ev.local_poh_start,
        local_poh_end: ev.local_poh_end,
        exit_status: ev.exit_status,
        output_digest: ev.output_digest.clone(),
        side_effect_digest: ev.side_effect_digest.clone(),
        connector_attestation_digest: String::new(),
        bounded_error_digest: String::new(),
    };
    let receipt = Receipt::ExecutionReceipt { envelope: envelope_template, payload };
    debug_assert_eq!(receipt.kind(), ReceiptKind::ExecutionReceipt);
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::BoundReceipt;
    use crate::normalizer::{normalize, Proposal};
    use crate::receipts::*;
    use crate::SideEffectClass;

    struct Mock;
    impl ReceiptVerifier for Mock {
        fn verify_bound(&self, cert: &[u8], _b: &[u8], expected: &str) -> bool {
            cert == format!("OK:{expected}").as_bytes()
        }
    }

    fn proposal(target: &str) -> Proposal {
        Proposal {
            principal: "user:owner".into(),
            orchestrator: "orch:generic".into(),
            agent_instance: "agent:demo#1".into(),
            session_ref: "s1".into(),
            capability: "filesystem.write".into(),
            target: target.into(),
            arguments: serde_json::json!({"content": "hello"}),
            tool_id: "fs-connector".into(),
            tool_version: "1.0.0".into(),
            tool_digest: "2".repeat(64),
            side_effect_class: SideEffectClass::W1,
            chain_id: String::new(),
            parent_digest: String::new(),
            attempt: 1,
        }
    }

    fn env(i: &ActionIntent, subject: &str, prev: &str) -> ReceiptEnvelope {
        ReceiptEnvelope {
            receipt_id: "r".into(),
            chain_id: i.chain_id.clone(),
            action_id: i.action_id.clone(),
            attempt: i.attempt,
            principal_id: i.principal.clone(),
            agent_instance_id: i.agent_instance.clone(),
            orchestrator_id: i.orchestrator.clone(),
            subject_digest: subject.into(),
            policy_digest: "3".repeat(64),
            causal_parent_digest: String::new(),
            previous_receipt_digest: prev.into(),
            local_poh_tick: 1,
            wall_clock_hint: String::new(),
            nonce: "n".into(),
            issuer_ref: "issuer:test".into(),
        }
    }

    fn triple(i: &ActionIntent, selector: &str) -> (GateInput, String, String, String) {
        let idig = i.intent_digest().unwrap();
        let permission = Receipt::PermissionReceipt {
            envelope: env(i, &idig, ""),
            payload: PermissionPayload {
                capability: i.capability.clone(),
                intent_binding: IntentBinding::ExactIntent,
                action_template_digest: String::new(),
                delegation_parent_digest: String::new(),
                revocation_epoch: 2,
                max_attempts: 3,
                required_validation_policy_digest: "4".repeat(64),
            },
        };
        let pd = permission.receipt_digest().unwrap();
        let scope = Receipt::ScopeReceipt {
            envelope: env(i, &idig, &pd),
            payload: ScopePayload {
                capability: i.capability.clone(),
                resource_namespace: "workspace".into(),
                target_selectors: vec![selector.into()],
                allowed_operations: vec!["write".into()],
                denied_operations: vec![],
                network_policy: "{}".into(),
                path_policy: "{}".into(),
                data_classification: "internal".into(),
                max_payload: 1,
                max_result: 1,
                max_attempts: 3,
                validity_window: "{}".into(),
                revocation_epoch: 2,
                human_approval_requirement: false,
            },
        };
        let sd = scope.receipt_digest().unwrap();
        let tool = Receipt::ToolReceipt {
            envelope: env(i, &idig, &sd),
            payload: ToolPayload {
                tool_id: i.tool_id.clone(),
                tool_version: i.tool_version.clone(),
                binary_or_image_digest: i.tool_digest.clone(),
                connector_id: "filesystem".into(),
                connector_version: "1".into(),
                input_schema_digest: "5".repeat(64),
                output_schema_digest: "6".repeat(64),
                environment_profile_digest: "7".repeat(64),
                secret_handle_policy: "{}".into(),
                allowed_endpoints: vec![],
                isolation_profile: "cooperative".into(),
            },
        };
        let td = tool.receipt_digest().unwrap();
        let bind = |r: &Receipt| BoundReceipt {
            wire: r.wire_bytes().unwrap(),
            cert: format!("OK:{}", r.receipt_digest().unwrap()).into_bytes(),
            bundle: vec![],
        };
        (
            GateInput { permission: bind(&permission), scope: bind(&scope), tool: bind(&tool) },
            pd,
            sd,
            td,
        )
    }

    #[test]
    fn writes_and_captures_execution_receipt() {
        let dir = std::env::temp_dir().join(format!("tlg-broker-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("workspace/out.txt");
        let sel = format!("{}/**", dir.join("workspace").display());
        let p = proposal(target.to_str().unwrap());
        let i = normalize(&p).unwrap();
        let (input, pd, sd, td) = triple(&i, &sel);

        let conn = FilesystemConnector { environment_profile_digest: "7".repeat(64) };
        let exec_env = env(&i, &"e".repeat(64), &td);
        let receipt = broker_execute(
            &i, &input, &Mock, &conn,
            &ExecInput { bytes: b"hello world".to_vec() },
            &sd, &pd, &td, 100, exec_env,
        )
        .unwrap();

        // the effect really happened
        assert_eq!(std::fs::read(&target).unwrap(), b"hello world");
        // the receipt captures it, chained after the tool receipt
        let Receipt::ExecutionReceipt { envelope, payload } = &receipt else { panic!() };
        assert_eq!(envelope.previous_receipt_digest, td);
        assert_eq!(payload.intent_digest, i.intent_digest().unwrap());
        assert_eq!(payload.exit_status, 0);
        assert!(payload.output_digest.starts_with(char::is_alphanumeric));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_effect_when_gate_stops() {
        let dir = std::env::temp_dir().join(format!("tlg-broker-stop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("workspace/out.txt");
        let sel = format!("{}/**", dir.join("workspace").display());
        let p = proposal(target.to_str().unwrap());
        let i = normalize(&p).unwrap();
        let (mut input, pd, sd, td) = triple(&i, &sel);
        // forge the permission cert so the gate re-check fails
        input.permission.cert = b"OK:wrong".to_vec();

        let conn = FilesystemConnector { environment_profile_digest: "7".repeat(64) };
        let exec_env = env(&i, &"e".repeat(64), &td);
        let out = broker_execute(
            &i, &input, &Mock, &conn,
            &ExecInput { bytes: b"should not be written".to_vec() },
            &sd, &pd, &td, 100, exec_env,
        );
        assert!(matches!(out, Err(BrokerError::NotAllowed(_))));
        // the file was never created — no ALLOW, no effect
        assert!(!target.exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wrong_connector_class_fails_closed() {
        let dir = std::env::temp_dir().join(format!("tlg-broker-wc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("workspace/out.txt");
        let sel = format!("{}/**", dir.join("workspace").display());
        let p = proposal(target.to_str().unwrap());
        let i = normalize(&p).unwrap();
        let (input, pd, sd, td) = triple(&i, &sel);

        struct Httpish;
        impl Connector for Httpish {
            fn class(&self) -> &'static str { "http" }
            fn execute(&self, _: &ActionIntent, _: &ExecInput, _: u64)
                -> Result<ExecutionEvidence, BrokerError> {
                panic!("must never be called for a filesystem capability");
            }
        }
        let exec_env = env(&i, &"e".repeat(64), &td);
        let out = broker_execute(
            &i, &input, &Mock, &Httpish,
            &ExecInput { bytes: b"x".to_vec() },
            &sd, &pd, &td, 1, exec_env,
        );
        assert!(matches!(out, Err(BrokerError::NoConnector(_))));
        assert!(!target.exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
