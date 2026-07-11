//! Pre-Execution Gate (SPEC §8.7, §9.3–9.6): the mandatory triple —
//! permission, scope, tool — checked BEFORE any side effect.
//!
//! The decision has exactly two values: ALLOW or STOP(reason). There is no
//! best-effort allow. Every fork fails closed. Verification of receipt
//! authenticity is delegated to a `ReceiptVerifier` (the official
//! timelayer-verifier behind a trait, so this logic is testable without
//! spawning processes — and NEVER replaceable by a "simplified" check, §3.1).

use crate::receipts::{decode_receipt_v1, IntentBinding, Receipt, ReceiptKind};
use crate::{ActionIntent, GateDecision, StopCode};

/// Authenticity oracle for one bound verification. Implementations call the
/// official verifier with `--expect` — bound-only, like everywhere in TL-Gate.
pub trait ReceiptVerifier {
    /// Must return true ONLY for "VALID FINAL" on exactly this subject digest.
    fn verify_bound(&self, cert: &[u8], bundle: &[u8], expected_subject_hex: &str) -> bool;
}

/// One receipt as it arrives from the resolver: canonical wire bytes plus the
/// TimeLayer attestation pair minted over its receipt_digest.
pub struct BoundReceipt {
    pub wire: Vec<u8>,
    pub cert: Vec<u8>,
    pub bundle: Vec<u8>,
}

pub struct GateInput {
    pub permission: BoundReceipt,
    pub scope: BoundReceipt,
    pub tool: BoundReceipt,
}

/// The §9.6 pre-gate decision for one canonical intent.
pub fn pre_gate(
    intent: &ActionIntent,
    input: &GateInput,
    verifier: &dyn ReceiptVerifier,
) -> GateDecision {
    // 0. Intent must produce a digest at all (fail-closed on malformed hex).
    let Ok(intent_digest) = intent.intent_digest() else {
        return GateDecision::Stop(StopCode::SchemaMismatch);
    };

    // 1. Decode all three; the kind lives in the wire bytes, so a scope
    //    receipt can never impersonate a permission receipt (§10.3).
    let slots = [
        (&input.permission, ReceiptKind::PermissionReceipt),
        (&input.scope, ReceiptKind::ScopeReceipt),
        (&input.tool, ReceiptKind::ToolReceipt),
    ];
    let mut decoded: Vec<Receipt> = Vec::with_capacity(3);
    for (br, want) in &slots {
        let Ok(r) = decode_receipt_v1(&br.wire) else {
            return GateDecision::Stop(StopCode::ReceiptNotValid);
        };
        if r.kind() != *want {
            return GateDecision::Stop(match want {
                ReceiptKind::PermissionReceipt => StopCode::NoPermissionReceipt,
                ReceiptKind::ScopeReceipt => StopCode::NoScopeReceipt,
                _ => StopCode::NoToolReceipt,
            });
        }
        decoded.push(r);
    }
    let (permission, scope, tool) = (&decoded[0], &decoded[1], &decoded[2]);

    // 2. Authenticity, bound to each receipt's own commitment. A receipt that
    //    is valid "in itself" but attests other bytes is nothing (P0-01).
    for (br, r) in [
        (&input.permission, permission),
        (&input.scope, scope),
        (&input.tool, tool),
    ] {
        let Ok(digest) = r.receipt_digest() else {
            return GateDecision::Stop(StopCode::ReceiptNotValid);
        };
        if !verifier.verify_bound(&br.cert, &br.bundle, &digest) {
            return GateDecision::Stop(StopCode::ReceiptNotValid);
        }
    }

    // 3. Subject binding: the permission must be about THIS intent (§9.3).
    let (Receipt::PermissionReceipt { envelope: pe, payload: pp },
         Receipt::ScopeReceipt { envelope: se, payload: sp },
         Receipt::ToolReceipt { envelope: te, payload: tp }) =
        (permission, scope, tool)
    else {
        return GateDecision::Stop(StopCode::ReceiptNotValid); // unreachable by §1
    };

    match pp.intent_binding {
        IntentBinding::ExactIntent => {
            if pe.subject_digest != intent_digest {
                return GateDecision::Stop(StopCode::SubjectMismatch);
            }
        }
        // Template binding needs the template matcher (Phase 1 slice 2) —
        // refusing is the only honest answer until it exists.
        IntentBinding::ActionTemplate => {
            return GateDecision::Stop(StopCode::NotImplemented);
        }
    }

    // 4. Envelope coherence across the triple: same chain, same action, same
    //    principal/agent; a receipt from another chain is a foreign receipt.
    for e in [se, te] {
        if e.chain_id != pe.chain_id
            || e.action_id != pe.action_id
            || e.principal_id != pe.principal_id
            || e.agent_instance_id != pe.agent_instance_id
        {
            return GateDecision::Stop(StopCode::SubjectMismatch);
        }
    }
    if pe.action_id != intent.action_id || pe.chain_id != intent.chain_id {
        return GateDecision::Stop(StopCode::SubjectMismatch);
    }
    if pe.principal_id != intent.principal || pe.agent_instance_id != intent.agent_instance {
        return GateDecision::Stop(StopCode::SubjectMismatch);
    }

    // 5. Chain integrity (§10.3): permission → scope → tool by digest links.
    let (Ok(p_digest), Ok(s_digest)) = (permission.receipt_digest(), scope.receipt_digest())
    else {
        return GateDecision::Stop(StopCode::ReceiptNotValid);
    };
    if se.previous_receipt_digest != p_digest || te.previous_receipt_digest != s_digest {
        return GateDecision::Stop(StopCode::ReceiptNotValid);
    }

    // 6. Attempt discipline.
    if intent.attempt == 0 || intent.attempt > pp.max_attempts || intent.attempt > sp.max_attempts
    {
        return GateDecision::Stop(StopCode::Replay);
    }

    // 7. Capability + scope enforcement (§9.4): exact capability, target
    //    inside a selector, operation allowed and not denied.
    if pp.capability != intent.capability || sp.capability != intent.capability {
        return GateDecision::Stop(StopCode::ScopeViolation);
    }
    if !sp.target_selectors.iter().any(|sel| selector_matches(sel, &intent.target)) {
        return GateDecision::Stop(StopCode::ScopeViolation);
    }
    let op = intent.capability.split('.').nth(1).unwrap_or("");
    if sp.denied_operations.iter().any(|d| d == op) {
        return GateDecision::Stop(StopCode::ScopeViolation);
    }
    if !sp.allowed_operations.is_empty() && !sp.allowed_operations.iter().any(|a| a == op) {
        return GateDecision::Stop(StopCode::ScopeViolation);
    }

    // 8. Tool binding (§9.5): id, version, executable digest — exact.
    if tp.tool_id != intent.tool_id
        || tp.tool_version != intent.tool_version
        || tp.binary_or_image_digest != intent.tool_digest
    {
        return GateDecision::Stop(StopCode::ToolSubstitution);
    }

    GateDecision::Allow
}

/// Selector language, Phase 1: exact path, or `<prefix>/**` matching any
/// descendant of `<prefix>`. The normalizer has already rejected `..`, `.`
/// and relative targets, so lexical prefix matching cannot be escaped
/// lexically; live symlink resolution belongs to the broker boundary.
pub fn selector_matches(selector: &str, target: &str) -> bool {
    if let Some(prefix) = selector.strip_suffix("/**") {
        target == prefix || target.starts_with(&format!("{prefix}/"))
    } else {
        selector == target
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalizer::{normalize, Proposal};
    use crate::receipts::*;
    use crate::SideEffectClass;

    /// Mock oracle: accepts a pair iff cert == b"OK:<subject>". Lets tests
    /// exercise gate logic without the external binary; the real oracle is
    /// the official verifier behind the same trait (bound-only).
    struct Mock;
    impl ReceiptVerifier for Mock {
        fn verify_bound(&self, cert: &[u8], _bundle: &[u8], expected: &str) -> bool {
            cert == format!("OK:{expected}").as_bytes()
        }
    }

    fn intent() -> ActionIntent {
        normalize(&Proposal {
            principal: "user:owner".into(),
            orchestrator: "orch:generic".into(),
            agent_instance: "agent:demo#1".into(),
            session_ref: "s1".into(),
            capability: "filesystem.write".into(),
            target: "/workspace/project-a/src/main.rs".into(),
            arguments: serde_json::json!({"patch_ref": "user-owned://p/1"}),
            tool_id: "fs-connector".into(),
            tool_version: "1.0.0".into(),
            tool_digest: "2".repeat(64),
            side_effect_class: SideEffectClass::W1,
            chain_id: String::new(),
            parent_digest: String::new(),
            attempt: 1,
        })
        .unwrap()
    }

    fn envelope(i: &ActionIntent, subject: &str, previous: &str) -> ReceiptEnvelope {
        ReceiptEnvelope {
            receipt_id: format!("r-{}", &subject[..8]),
            chain_id: i.chain_id.clone(),
            action_id: i.action_id.clone(),
            attempt: i.attempt,
            principal_id: i.principal.clone(),
            agent_instance_id: i.agent_instance.clone(),
            orchestrator_id: i.orchestrator.clone(),
            subject_digest: subject.into(),
            policy_digest: "3".repeat(64),
            causal_parent_digest: String::new(),
            previous_receipt_digest: previous.into(),
            local_poh_tick: 1,
            wall_clock_hint: String::new(),
            nonce: "n".into(),
            issuer_ref: "issuer:test".into(),
        }
    }

    /// A coherent triple bound to the intent; returns (input, receipts).
    fn triple(i: &ActionIntent) -> GateInput {
        let idig = i.intent_digest().unwrap();
        let permission = Receipt::PermissionReceipt {
            envelope: envelope(i, &idig, ""),
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
        let p_digest = permission.receipt_digest().unwrap();
        let scope = Receipt::ScopeReceipt {
            envelope: envelope(i, &idig, &p_digest),
            payload: ScopePayload {
                capability: i.capability.clone(),
                resource_namespace: "workspace".into(),
                target_selectors: vec!["/workspace/project-a/src/**".into()],
                allowed_operations: vec!["write".into()],
                denied_operations: vec!["delete".into()],
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
        let s_digest = scope.receipt_digest().unwrap();
        let tool = Receipt::ToolReceipt {
            envelope: envelope(i, &idig, &s_digest),
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
        let bind = |r: &Receipt| BoundReceipt {
            wire: r.wire_bytes().unwrap(),
            cert: format!("OK:{}", r.receipt_digest().unwrap()).into_bytes(),
            bundle: vec![],
        };
        GateInput { permission: bind(&permission), scope: bind(&scope), tool: bind(&tool) }
    }

    #[test]
    fn coherent_triple_allows() {
        let i = intent();
        assert_eq!(pre_gate(&i, &triple(&i), &Mock), GateDecision::Allow);
    }

    #[test]
    fn transplant_from_other_intent_stops() {
        // receipts minted for intent A, presented for intent B (different
        // target) — the audit's receipt transplant, now at gate level
        let a = intent();
        let input = triple(&a);
        let mut p = Proposal {
            principal: a.principal.clone(),
            orchestrator: a.orchestrator.clone(),
            agent_instance: a.agent_instance.clone(),
            session_ref: "s1".into(),
            capability: a.capability.clone(),
            target: "/workspace/project-a/src/other.rs".into(),
            arguments: serde_json::json!({"patch_ref": "user-owned://p/1"}),
            tool_id: a.tool_id.clone(),
            tool_version: a.tool_version.clone(),
            tool_digest: a.tool_digest.clone(),
            side_effect_class: SideEffectClass::W1,
            chain_id: String::new(),
            parent_digest: String::new(),
            attempt: 1,
        };
        let b = normalize(&mut p).unwrap();
        assert_eq!(pre_gate(&b, &input, &Mock), GateDecision::Stop(StopCode::SubjectMismatch));
    }

    #[test]
    fn scope_escape_stops() {
        let i = intent();
        let mut input = triple(&i);
        // shrink the scope selector so the target falls outside
        let Receipt::ScopeReceipt { envelope, mut payload } =
            decode_receipt_v1(&input.scope.wire).unwrap()
        else {
            unreachable!()
        };
        payload.target_selectors = vec!["/workspace/project-b/**".into()];
        let forged = Receipt::ScopeReceipt { envelope, payload };
        input.scope.wire = forged.wire_bytes().unwrap();
        input.scope.cert = format!("OK:{}", forged.receipt_digest().unwrap()).into_bytes();
        // chain link now breaks first? no: scope digest changed, tool.previous
        // points at the OLD scope → the gate must stop either way, and it does
        // via the chain check — a forged scope cannot slip through
        assert_ne!(pre_gate(&i, &input, &Mock), GateDecision::Allow);
    }

    #[test]
    fn out_of_scope_target_stops_via_scope_check() {
        // honest chain issued for a WIDE selector, но target вне селектора:
        // готовим intent на чужой путь и цепочку, честно перевыпущенную под
        // него, кроме селектора — остаётся только scope-проверка
        let mut p = Proposal {
            principal: "user:owner".into(),
            orchestrator: "orch:generic".into(),
            agent_instance: "agent:demo#1".into(),
            session_ref: "s1".into(),
            capability: "filesystem.write".into(),
            target: "/etc/passwd".into(),
            arguments: serde_json::json!({}),
            tool_id: "fs-connector".into(),
            tool_version: "1.0.0".into(),
            tool_digest: "2".repeat(64),
            side_effect_class: SideEffectClass::W1,
            chain_id: String::new(),
            parent_digest: String::new(),
            attempt: 1,
        };
        let i = normalize(&mut p).unwrap();
        let input = triple(&i); // selectors are /workspace/project-a/src/**
        assert_eq!(pre_gate(&i, &input, &Mock), GateDecision::Stop(StopCode::ScopeViolation));
    }

    #[test]
    fn denied_operation_stops() {
        let mut p = Proposal {
            principal: "user:owner".into(),
            orchestrator: "orch:generic".into(),
            agent_instance: "agent:demo#1".into(),
            session_ref: "s1".into(),
            capability: "filesystem.delete".into(),
            target: "/workspace/project-a/src/main.rs".into(),
            arguments: serde_json::json!({}),
            tool_id: "fs-connector".into(),
            tool_version: "1.0.0".into(),
            tool_digest: "2".repeat(64),
            side_effect_class: SideEffectClass::W3,
            chain_id: String::new(),
            parent_digest: String::new(),
            attempt: 1,
        };
        let i = normalize(&mut p).unwrap();
        let input = triple(&i);
        assert_eq!(pre_gate(&i, &input, &Mock), GateDecision::Stop(StopCode::ScopeViolation));
    }

    #[test]
    fn tool_substitution_stops() {
        let a = intent();
        let input = triple(&a);
        // same everything, different executable digest → new intent digest →
        // subject mismatch; and even with a re-bound permission the tool
        // receipt still names the old binary. Check the direct path: forge an
        // intent whose only difference is the tool digest and re-bind the
        // permission subject to it.
        let mut i2 = a.clone();
        i2.tool_digest = "9".repeat(64);
        // rebuild a triple honestly bound to i2 EXCEPT the tool payload,
        // which still attests the old binary
        let mut input2 = triple(&i2);
        input2.tool = BoundReceipt {
            wire: input.tool.wire.clone(),
            cert: input.tool.cert.clone(),
            bundle: vec![],
        };
        assert_ne!(pre_gate(&i2, &input2, &Mock), GateDecision::Allow);
    }

    #[test]
    fn broken_chain_stops() {
        let i = intent();
        let mut input = triple(&i);
        // re-issue scope with previous_receipt_digest pointing nowhere
        let Receipt::ScopeReceipt { mut envelope, payload } =
            decode_receipt_v1(&input.scope.wire).unwrap()
        else {
            unreachable!()
        };
        envelope.previous_receipt_digest = "0".repeat(64);
        let forged = Receipt::ScopeReceipt { envelope, payload };
        input.scope.wire = forged.wire_bytes().unwrap();
        input.scope.cert = format!("OK:{}", forged.receipt_digest().unwrap()).into_bytes();
        assert_eq!(pre_gate(&i, &input, &Mock), GateDecision::Stop(StopCode::ReceiptNotValid));
    }

    #[test]
    fn attempt_over_limit_stops() {
        let a = intent();
        // attempt 4 > max_attempts 3, with a chain honestly re-bound to it
        let mut p = Proposal {
            principal: a.principal.clone(),
            orchestrator: a.orchestrator.clone(),
            agent_instance: a.agent_instance.clone(),
            session_ref: "s1".into(),
            capability: a.capability.clone(),
            target: a.target.clone(),
            arguments: serde_json::json!({"patch_ref": "user-owned://p/1"}),
            tool_id: a.tool_id.clone(),
            tool_version: a.tool_version.clone(),
            tool_digest: a.tool_digest.clone(),
            side_effect_class: SideEffectClass::W1,
            chain_id: String::new(),
            parent_digest: String::new(),
            attempt: 4,
        };
        let i = normalize(&mut p).unwrap();
        let input = triple(&i);
        assert_eq!(pre_gate(&i, &input, &Mock), GateDecision::Stop(StopCode::Replay));
    }

    #[test]
    fn forged_cert_stops() {
        let i = intent();
        let mut input = triple(&i);
        input.permission.cert = b"OK:0000".to_vec();
        assert_eq!(pre_gate(&i, &input, &Mock), GateDecision::Stop(StopCode::ReceiptNotValid));
    }

    #[test]
    fn selector_language() {
        assert!(selector_matches("/a/b/**", "/a/b/c/d"));
        assert!(selector_matches("/a/b/**", "/a/b"));
        assert!(!selector_matches("/a/b/**", "/a/bc"));
        assert!(selector_matches("/a/b", "/a/b"));
        assert!(!selector_matches("/a/b", "/a/b/c"));
    }
}
