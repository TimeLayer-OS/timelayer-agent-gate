//! Internal: regenerate testvectors/intent-v1.json (run from repo root):
//!   cargo run -p tl-gate-cli --bin gen-vectors > testvectors/intent-v1.json
use tl_gate_core::{ActionIntent, SideEffectClass};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn main() {
    let vectors = vec![
        (
            "minimal chain root",
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
            },
        ),
        (
            "unicode + child with parent",
            ActionIntent {
                schema: "tl-gate.action-intent/1".into(),
                principal: "user:владелец".into(),
                orchestrator: "orchestrator:штаб".into(),
                agent_instance: "agent:глаз#7".into(),
                session_ref: "сессия-42".into(),
                capability: "http.post".into(),
                target: "https://api.example.com/v1/pay".into(),
                arguments_digest:
                    "a0b1c2d3e4f5061728394a5b6c7d8e9fa0b1c2d3e4f5061728394a5b6c7d8e9f".into(),
                tool_id: "http-connector".into(),
                tool_version: "2.3.1".into(),
                tool_digest: "deadbeef".repeat(8),
                side_effect_class: SideEffectClass::W3,
                action_id: "оплата-1".into(),
                chain_id: "цепь-9".into(),
                attempt: 3,
                parent_digest: "f".repeat(64),
            },
        ),
    ];
    let mut out = Vec::new();
    for (name, i) in vectors {
        let wire = i.wire_bytes().unwrap();
        out.push(serde_json::json!({
            "name": name,
            "intent": serde_json::to_value(&i).unwrap(),
            "wire_hex": hex(&wire),
            "intent_digest": i.intent_digest().unwrap(),
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "format": "TL-GATE-WIRE/v1",
            "frozen": "2026-07-11",
            "domain": "TL-GATE/INTENT/v1",
            "vectors": out,
        }))
        .unwrap()
    );
}
