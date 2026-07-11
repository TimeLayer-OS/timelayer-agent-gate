//! Internal: build a coherent receipt workspace bound to a proposal, printing
//! each receipt's wire bytes (hex) and receipt_digest so an external script can
//! notarize the digests and drop the cert/bundle pairs next to the .tlgw files.
use tl_gate_core::normalizer::{normalize, Proposal};
use tl_gate_core::receipts::*;

fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

fn env(i: &tl_gate_core::ActionIntent, subject: &str, prev: &str) -> ReceiptEnvelope {
    ReceiptEnvelope {
        receipt_id: "r".into(), chain_id: i.chain_id.clone(), action_id: i.action_id.clone(),
        attempt: i.attempt, principal_id: i.principal.clone(),
        agent_instance_id: i.agent_instance.clone(), orchestrator_id: i.orchestrator.clone(),
        subject_digest: subject.into(), policy_digest: "3".repeat(64),
        causal_parent_digest: String::new(), previous_receipt_digest: prev.into(),
        local_poh_tick: 1, wall_clock_hint: String::new(), nonce: "n".into(),
        issuer_ref: "issuer:cabinet".into(),
    }
}

fn main() {
    let text = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let p: Proposal = serde_json::from_str(&text).unwrap();
    let i = normalize(&p).unwrap();
    let idig = i.intent_digest().unwrap();
    let perm = Receipt::PermissionReceipt { envelope: env(&i, &idig, ""), payload: PermissionPayload {
        capability: i.capability.clone(), intent_binding: IntentBinding::ExactIntent,
        action_template_digest: String::new(), delegation_parent_digest: String::new(),
        revocation_epoch: 2, max_attempts: 3, required_validation_policy_digest: "4".repeat(64) } };
    let pd = perm.receipt_digest().unwrap();
    let scope = Receipt::ScopeReceipt { envelope: env(&i, &idig, &pd), payload: ScopePayload {
        capability: i.capability.clone(), resource_namespace: "workspace".into(),
        target_selectors: vec!["/workspace/project-a/src/**".into()],
        allowed_operations: vec!["write".into()], denied_operations: vec!["delete".into()],
        network_policy: "{}".into(), path_policy: "{}".into(), data_classification: "internal".into(),
        max_payload: 1, max_result: 1, max_attempts: 3, validity_window: "{}".into(),
        revocation_epoch: 2, human_approval_requirement: false } };
    let sd = scope.receipt_digest().unwrap();
    let tool = Receipt::ToolReceipt { envelope: env(&i, &idig, &sd), payload: ToolPayload {
        tool_id: i.tool_id.clone(), tool_version: i.tool_version.clone(),
        binary_or_image_digest: i.tool_digest.clone(), connector_id: "filesystem".into(),
        connector_version: "1".into(), input_schema_digest: "5".repeat(64),
        output_schema_digest: "6".repeat(64), environment_profile_digest: "7".repeat(64),
        secret_handle_policy: "{}".into(), allowed_endpoints: vec![], isolation_profile: "cooperative".into() } };
    let td = tool.receipt_digest().unwrap();
    for (name, r, d) in [("permission",&perm,&pd),("scope",&scope,&sd),("tool",&tool,&td)] {
        println!("{name}\t{}\t{d}", hex(&r.wire_bytes().unwrap()));
    }
}
