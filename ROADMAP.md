# Roadmap

Tracks SPEC §30. One phase at a time; a phase is done when its negative test
vectors pass, not when its happy path demos well.

- [x] Repository, license, bilingual spec (RU normative, EN by meaning)
- [ ] **Phase 0 — protocol freeze**
  - [x] intent domain separator + canonical digest (provisional JSON form)
  - [x] verifier bridge, bound-only (`--expect` mandatory)
  - [ ] TL-GATE-WIRE/v1 length-prefixed binary form + cross-language vectors
  - [ ] receipt envelope schemas (permission/scope/tool/execution/validation/final)
  - [ ] negative vector suite: forged / replay / scope-escape / tool-substitution /
        output-substitution / validation-fail / finality-conflict
- [ ] Phase 1 — local universal gate (normalizer, pre-gate, broker: fs/process/http)
- [ ] Phase 2 — MCP control plane
- [ ] Phase 3 — multi-agent delegation
- [ ] Phase 4 — Second Brain bridge
- [ ] Phase 5 — strong isolation modes
