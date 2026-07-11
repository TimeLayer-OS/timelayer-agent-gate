# Roadmap

Tracks SPEC §30. One phase at a time; a phase is done when its negative test
vectors pass, not when its happy path demos well.

- [x] Repository, license, bilingual spec (RU normative, EN by meaning)
- [x] **Phase 0 — protocol freeze — COMPLETE 2026-07-11**
  - [x] intent domain separator + canonical digest
  - [x] verifier bridge, bound-only (`--expect` mandatory)
  - [x] **TL-GATE-WIRE/v1 FROZEN 2026-07-11** (schemas/TL-GATE-WIRE-v1.md):
        length-prefixed binary, frozen domain set, typed commitment object
        (schemas/commitment-v1.json); intent digest now hashes wire bytes
  - [x] cross-language vectors: Rust generator + independent Python
        implementation, byte-for-byte PASS (testvectors/)
  - [x] receipt envelope schemas per SPEC §10–11: common envelope +
        permission/scope/tool/execution/validation/final (schemas/*.json,
        human-readable mirrors; binary receipt encoding freezes before Phase 0 exit)
  - [x] wire reader (decode_intent_v1), fail-closed per WIRE §6, with negative
        vectors: forged byte, bad magic, truncation at every length, trailing
        bytes, unknown enum, cross-domain replay — all reject
  - [x] binary wire encoding for the six receipt kinds (receipts.rs): envelope
        §10.1 + payloads §11.1–11.6, fail-closed decoder, per-kind domains,
        kind-substitution impossible by construction; roundtrip + tamper +
        truncation + trailing + bad-enum/bool tests for every kind
  - [ ] gate-level negative suite (scope-escape / tool-substitution /
        output-substitution / validation-fail / finality-conflict) — needs the
        Phase 1 gate to exist first
- [~] **Phase 1 — local universal gate** (in progress)
  - [x] Intent Normalizer (§8.2): fs/http/process canonicalization, args digest,
        content-derived action_id; traversal/relative/dot rejected pre-scope
  - [x] Pre-Execution Gate (§8.7): mandatory triple via ReceiptVerifier trait
        (bound-only), exact-intent subject binding, envelope coherence, chain
        integrity (permission→scope→tool), scope enforcement (selectors +
        allowed/denied ops), tool binding (id+version+digest), attempt limits
  - [x] gate negatives: transplant, scope-escape, out-of-scope target,
        denied op, tool substitution, broken chain, attempt-over-limit,
        forged cert — all STOP; live e2e ALLOW + STOP on the real network
  - [x] `tl-gate check <workspace> <proposal.json>` — real verifier adapter
  - [ ] action-template binding matcher (slice 2)
  - [ ] Controlled Tool Broker: fs/process/http (slice 3) — real side effects,
        exact input/output capture, execution_receipt
- [ ] Phase 2 — MCP control plane
- [ ] Phase 3 — multi-agent delegation
- [ ] Phase 4 — Second Brain bridge
- [ ] Phase 5 — strong isolation modes
