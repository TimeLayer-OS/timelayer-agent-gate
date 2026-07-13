# TL-Gate — TimeLayer Agent Gate

**English** · [Русский](README.ru.md)

> **FOR AGENTS.** The gate over any agent's side effect, fail-closed:
> ```bash
> tl-gate check <workspace> <proposal.json>   # ALLOW | STOP(reason)
> ```
> Working method: [`AGENTS.md`](AGENTS.md).

## In plain words

AI agents can now write files, call APIs, move money. The uncomfortable question
is not "can they act" — it's **"who said this exact action was allowed, and can
you prove it afterwards?"**

Today the answer is usually a log line the agent itself wrote, or a hook that
can be switched off. Both are stories, not evidence.

**TL-Gate is a gate that stands between an agent's intention and the real
side effect.** Before anything happens, the action must show a signed,
independently verifiable receipt that says: *this* principal allowed *this*
agent to do *this* action on *this* target with *this* tool. No receipt for
this exact action — nothing happens. That's the whole idea:

```text
Orchestrator plans.
Agent proposes.
TL-Gate governs and executes through a controlled boundary.
Verifier checks receipts.
Second Brain stores only receipt-bound knowledge.
```

TL-Gate is **not** another orchestrator, not an LLM, and not a memory system.
It does one job: a mandatory, fail-closed, receipt-driven boundary — provably
linking permission, scope, tool, execution, validation, and finality.

## Why you might care

- **You run AI agents** and want a boundary they cannot talk their way past —
  a model saying "done" is not proof of anything here.
- **You answer to auditors or the EU AI Act** (arts. 12/19: tamper-evident
  records of high-risk AI actions, enforced from 2026-08-02) and need evidence
  that survives hostile review, offline, without trusting the vendor.
- **You build agent tooling** and want governed execution without adopting a
  whole framework: TL-Gate is orchestrator-neutral by design (principle P-01).

## Status: Phase 0 — protocol freeze

This repository is being built strictly against the
[specification](SPEC.ru.md) (Russian edition is normative for now;
[English edition](SPEC.md) tracks it by meaning). What exists today is small
and honest:

| Piece | State |
|---|---|
| `tl-gate-core` — canonical `ActionIntent`, BLAKE3 domain-separated commitments, gate decisions, chain states | implemented + tested |
| `tl-gate-verifier-bridge` — bound-only offline verification via the official `timelayer-verifier` (`--expect` is mandatory) | implemented + tested |
| `tl-gate` CLI — `intent digest`, `verify`, `spec-version` | implemented |
| Everything else the spec defines (broker, policy, validation, finalization, capsules) | **not yet — and fails closed**: unimplemented commands exit `STOP(NOT_IMPLEMENTED)`, never a pretend-ALLOW |

Try it:

```bash
cargo build
./target/debug/tl-gate intent digest examples/intent.json
./target/debug/tl-gate verify cert.tlcert bundle.tlbundle --expect <digest>
```

## The rules that don't bend

1. **No valid receipt for this exact action → no action.** Missing, expired,
   revoked, or somebody-else's receipt all mean STOP.
2. **The gate stands before the side effect**, not after it.
3. **Agents cannot self-authorize**, and a model's "success" text is not proof.
4. **Uncertainty never becomes permission**: timeouts, unknown schemas, an
   unavailable verifier — all STOP.
5. **BLAKE3 only, domain-separated** — a bare 32-byte hash with no type is not
   a commitment (see the 2026-07-11 audit, P0-04).
6. **Logless by default**: bounded, immutable evidence capsules instead of an
   endless editable journal. Verification works offline, against the network's
   quorum signatures — not against our word.

## Relation to the other TimeLayer repositories

- [`timelayer-verifier`](https://github.com/TimeLayer-OS/timelayer-verifier) —
  the independent offline verifier. TL-Gate calls it; it never re-implements it.
- [`TL-Agent`](https://github.com/TimeLayer-OS/TL-Agent) — receipt-gated agent
  SDK; its permission bundles become one possible receipt source via an adapter.
- [`timelayer-second-brain`](https://github.com/TimeLayer-OS/timelayer-second-brain) —
  the knowledge base on receipts; only `final_receipt`-bearing results may be
  promoted into it.
- [`receipt-driven-examples`](https://github.com/TimeLayer-OS/receipt-driven-examples) —
  minimal copy-me patterns; reference material, not runtime.

## License

Apache-2.0 — see [LICENSE](LICENSE).
