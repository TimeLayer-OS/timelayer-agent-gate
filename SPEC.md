# TimeLayer Agent Gate (TL-Gate) — Architecture Specification, English edition

**Status:** Proposed Architecture Specification, v0.1 (2026-07-11).
**Normative text:** for now, the Russian edition [`SPEC.ru.md`](SPEC.ru.md) is
normative; this English edition is a faithful condensed translation by meaning.
Where they disagree, the Russian edition wins until the protocol freeze
(Phase 0) completes.

---

## 0. Architectural decision

TL-Gate is **independent of any particular AI agent, model, framework, or
orchestrator**. It is not a new orchestrator and does not replace planning,
swarm coordination, task routing, or model selection. Its purpose:

> Put a mandatory, fail-closed, receipt-driven boundary between any agent's
> intention and the real side effect; provably link permission, scope, chosen
> tool, actual execution, result validation, and finality; then optionally
> hand the proven result to Second Brain.

```text
Orchestrator plans.
Agent proposes.
TL-Gate governs and executes through a controlled boundary.
Verifier checks receipts.
Second Brain stores only receipt-bound knowledge.
```

## 1. The problem

Modern agent stacks can decompose tasks, pick agents and models, call tools,
and keep memory — but none of that *proves* who allowed an action, in what
scope, with which tool and version, whether the arguments matched what was
allowed, what actually executed, who validated the result, and whether
anything changed after validation. Hooks, callbacks, tracing, and logs live
inside the same trusted environment, can be disabled or edited, record
*claims* rather than cryptographically linked chains, and rarely stand
*before* the side effect.

## 2. Product boundary

**TL-Gate does:** accept an action from any external orchestrator; normalize
it into a canonical `ActionIntent`; establish initiator and agent identity;
check `permission_receipt`, `scope_receipt`, `tool_receipt`; refuse on any
conflict; route allowed actions only through the Controlled Tool Broker;
capture exact input and output; produce `execution_receipt`; run assigned
validators; produce `validation_receipt`; produce `final_receipt` only for an
accepted result; store a bounded evidence capsule; optionally promote to
Second Brain; automatically strip downstream trust on digest mismatch; support
offline verification via the separate `timelayer-verifier`.

**TL-Gate does not:** choose the user's business goal; plan instead of the
orchestrator; act as an LLM or vector memory; declare content true; replace
the verifier; absorb TL-Agent or Second Brain; keep user content in the
TimeLayer network; treat agent text or hooks as proof; let an agent issue its
own valid receipts; keep an infinite mutable journal as the source of truth.

## 3. Relation to the four existing repositories

TL-Gate is the **fifth, separate repository** — not a merger.
`timelayer-verifier` stays the only verification implementation (external
binary or official library build; any output/exit/schema mismatch = STOP).
`TL-Agent` bundles become one optional source of permission receipts via an
adapter. `timelayer-second-brain` receives only `final_receipt`-bearing
results through an optional bridge. `receipt-driven-examples` supplies
reference patterns and test fixtures, never runtime code.

## 4. Principles

- **P-01 Orchestrator neutrality** — the core sees only canonical intents.
- **P-02 No valid receipt → no action.**
- **P-03 Pre-effect enforcement** — checks stand before the side effect.
- **P-04 Exact binding** — permission ties to exact principal, agent instance,
  action kind, target, tool identity, arguments digest, policy digest, causal
  parent, attempt number.
- **P-05 Agents cannot self-authorize.**
- **P-06 Model output is not proof.**
- **P-07 Validation is explicit.**
- **P-08 Finality is explicit** — output ≠ FINAL; until the final receipt the
  result is NON_FINAL.
- **P-09 Logless by default** — bounded evidence capsules + immutable
  receipts, not an endless mutable journal.
- **P-10 BLAKE3 only** — with domain separation; SHA-256 is not a canonical
  TL-Gate digest.
- **P-11 LocalPoH orders; wall clock informs.**
- **P-12 Fail-closed propagation** — UNKNOWN/INCONCLUSIVE/timeout/verifier
  unavailable/schema mismatch never pass.
- **P-13 Delegation cannot amplify trust** — child scope ⊆ parent scope.
- **P-14 Separate truth domains** — plan (orchestrator), governed execution
  (TL-Gate), each check (its validator), attestation/finality (TimeLayer
  receipt), knowledge state (Second Brain).

## 5. Components

1. **Adapter Gateway** — accepts harness calls (MCP proxy/server, Unix socket,
   named pipe, stdio, local HTTP loopback, Rust SDK, generic JSON-RPC);
   version handshake; capability manifest; no public network listener by
   default.
2. **Intent Normalizer** — resolves targets to canonical form, computes the
   BLAKE3 arguments digest, assigns `action_id`/`chain_id`/`attempt`, links to
   the parent. After normalization the intent is immutable; any change is a
   new action.
3. **Identity & Delegation Resolver** — principal/orchestrator/agent/session
   identity, delegation lineage, no scope amplification, revocation, replay
   protection.
4. **Policy Compiler** — human policy → deterministic policy IR (capabilities,
   path/host selectors, methods, tool allowlists, version constraints, size &
   attempt limits, validation and human-approval requirements, isolation
   profile). Only the compiled policy digest participates in the action path.
5. **Receipt Resolver** — obtains receipts (offline bundle, TimeLayer issue
   client, TL-Agent adapter, enterprise issuer, human workflow); trusts no
   source until verified.
6. **Verifier Bridge** — `verify(cert, bundle, expected_subject_digest) →
   VALID_FINAL | NOT_VALID | UNVERIFIABLE | ERROR`; only exact `VALID FINAL` +
   exit 0 continues; expected-subject binding is checked always; verifier
   version and binary digest go into the evidence capsule.
7. **Pre-Execution Gate** — requires the mandatory triple
   `permission_receipt`, `scope_receipt`, `tool_receipt`; decision is only
   `ALLOW` or `STOP(reason)`.
8. **Controlled Tool Broker** — the only point of real side effects
   (filesystem, process, MCP, HTTP, browser, database, messaging, cloud,
   custom connectors); re-checks the intent digest before execution; passes
   only allowed secret handles; blocks environment inheritance; records exact
   input/output; enforces timeouts and limits.
9. **Result Capture** — immutable `ExecutionEvidence`: input/tool/environment/
   output digests, exit status, side-effect summary, created/modified/deleted
   object digests, LocalPoH positions, causal parent, bounded stdout/stderr
   references.
10. **Validation Engine** — policy-defined validators (deterministic, schema,
    tests, diff policy, security scanner, independent model judge, human,
    external service); verdicts PASS/FAIL/INCONCLUSIVE; only PASS satisfying
    the threshold policy allows finalization.
11. **Finalizer** — final subject digest over the intent, all receipt digests,
    the final result digest, chain position, and policy digest → TimeLayer
    proof → `final_receipt`.
12. **Evidence Capsule Store** — bounded capsules per chain (manifest, intent,
    receipts, refs, state), user content may stay in user-owned storage.
13. **Stop-State / Recovery Controller** — verifier error, receipt mismatch,
    scope violation, tool substitution, replay, validation failure,
    finalization conflict, corruption, bypass detection → Stop-State; resume
    only via a new recovery authorization linked to the stopped chain.

## 6. Action lifecycle

Proposal → canonical intent
(`intent_digest = BLAKE3("TL-GATE/INTENT/v1" || canonical_intent)`) →
authorization (permission receipt bound to principal, agent, capability,
intent digest or allowed template, delegation lineage, validity/revocation) →
scope enforcement (symlinks, traversal, and mount escapes resolved *before*
comparison) → tool binding (tool ID, semver constraint, executable/container
digest, adapter version, input/output schema digests, environment profile) →
pre-gate decision (all receipts valid ∧ subjects match ∧ no revocation ∧ no
replay ∧ topology/delegation valid ∧ policy match → ALLOW, else STOP) →
controlled execution (any argument change after ALLOW = new intent) →
execution receipt → validation → validation receipt → finalization →
final receipt → evidence capsule → optional Second Brain promotion.

## 7. Receipt model

Every receipt shares a common envelope (schema, type, subject digest, domain,
issuer, causal parent, chain position) and its own **domain separator**:
`TL-GATE/INTENT/v1`, `TL-GATE/PERMISSION/v1`, `TL-GATE/SCOPE/v1`,
`TL-GATE/TOOL/v1`, `TL-GATE/EXECUTION/v1`, `TL-GATE/VALIDATION/v1`,
`TL-GATE/FINAL/v1` — the same 32 bytes can never be replayed across domains.
The mandatory chain for one action:
permission → scope → tool → execution → validation → final; each link carries
the previous receipt's commitment, sequence number, subject commitment, and
transition type. Additional receipts: `delegation_receipt`,
`revocation_receipt`, `stop_receipt`, `recovery_receipt`.

## 8. Action classes and enforcement modes

Classes: **R0** pure computation, **R1** read-only external, **W1** reversible
write, **W2** transactional external effect, **W3** irreversible/high-impact
(W3 typically demands human approval and the strictest validation).
Modes, weakest to strongest: **cooperative** (in-process, honest-agent
assumption), **broker-enforced** (separate broker process holds the
credentials), **isolated** (sandbox/container boundary), **air-gapped
permission** (receipts issued off-host), **hardware-backed**. A compromised
host defeats cooperative mode — the spec says so out loud (TB-05).

## 9. State machine honesty

Beyond the usual states, two are first-class: `EXECUTED_UNFINALIZED` (effect
happened, no final receipt yet — not the same as FINAL) and `EFFECT_UNKNOWN`
(crash mid-effect; requires explicit reconciliation, never silent retry).

## 10. Security model

Main threats: receipt transplant, scope escape (path traversal, symlink,
redirect, DNS rebinding), tool substitution, argument mutation after ALLOW,
replay, delegation amplification, output substitution, validator spoofing,
evidence tampering. Secrets: the broker passes handles, never values, and
never inherits the environment. Prompt injection: text from tools and sources
is data; it cannot change policy or issue receipts. **Honest limitations:**
TL-Gate proves governance of the boundary it controls; it does not make the
model smarter, does not guarantee business correctness of a validated result,
and cannot survive a fully compromised host in cooperative mode.

## 11. Wire format, CLI, testing

Canonical wire format `TL-GATE-WIRE/v1`: length-prefixed deterministic binary
(same model as the verifier), with a human-readable JSON mirror that is
informational, never authoritative. Digests: 64 lowercase hex, always paired
with algorithm + domain + subject type + version. CLI: `tl-gate init | serve |
adapters | tools | policy compile | intent submit | check | execute |
validate | finalize | status | stop | recover | verify | export | audit | gc`;
exit codes 0 ALLOW/FINAL, 1 STOP/not valid, 2 malformed input, 3 unavailable
dependency, 4 EXECUTED_UNFINALIZED, 5 EFFECT_UNKNOWN. Testing: unit tests,
cross-language canonical vectors, negative vectors (forged, replayed,
scope-escape, tool-substitution, output-substitution, validation-fail,
finality-conflict), integration chains, security tests.

## 12. Implementation phases

- **Phase 0 — protocol freeze**: wire format, domains, receipt envelopes,
  canonical vectors. ← *this repository is here*
- **Phase 1 — local universal gate**: normalizer, pre-gate, broker
  (filesystem/process/HTTP), capture, capsules.
- **Phase 2 — MCP control plane**: MCP proxy profile, tool manifests.
- **Phase 3 — multi-agent delegation**: delegation receipts, lineage,
  amplification checks.
- **Phase 4 — Second Brain bridge**: promotion of final results only.
- **Phase 5 — strong isolation**: broker-enforced and isolated modes.

## 13. Product formula

> TL-Gate turns "the agent said it did X" into "here is an offline-verifiable
> receipt chain proving who allowed X, what exactly ran, what came out, who
> checked it, and that nothing changed since." Everything else in the stack
> keeps its own job; TL-Gate guards the boundary.
