# Roadmap

Tracks SPEC §30. One phase at a time; a phase is done when its negative test
vectors pass, not when its happy path demos well.

- [x] Repository, license, bilingual spec (RU normative, EN by meaning)
- [ ] **Phase 0 — protocol freeze**
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
  - [ ] binary wire encoding for the six receipt kinds (last Phase 0 item)
  - [ ] gate-level negative suite (scope-escape / tool-substitution /
        output-substitution / validation-fail / finality-conflict) — needs the
        Phase 1 gate to exist first
- [ ] Phase 1 — local universal gate (normalizer, pre-gate, broker: fs/process/http)
- [ ] Phase 2 — MCP control plane
- [ ] Phase 3 — multi-agent delegation
- [ ] Phase 4 — Second Brain bridge
- [ ] Phase 5 — strong isolation modes
