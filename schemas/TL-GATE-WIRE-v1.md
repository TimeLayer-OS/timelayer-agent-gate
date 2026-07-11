# TL-GATE-WIRE/v1 — canonical binary wire format

**Status: FROZEN 2026-07-11.** Any change to the rules below is TL-GATE-WIRE/v2,
never an edit. The human-readable JSON mirror is informational; **only the wire
bytes are ever hashed** (spec §24: explicit types and lengths, the same model
the verifier uses — no canonical-JSON ambiguity, no Unicode normalization
debates, no `-0`/exponent traps).

## 1. Primitives

All integers are **little-endian, fixed width**. No varints.

| Type | Encoding |
|---|---|
| `u8` | 1 byte |
| `u32` | 4 bytes LE |
| `u64` | 8 bytes LE |
| `bytes` | `u32` length prefix, then exactly that many bytes |
| `str` | `bytes` holding UTF-8; **no normalization** — the writer must emit NFC, the reader hashes what it got |
| `digest` | exactly 32 raw bytes (no length prefix) |
| `list<T>` | `u32` count, then each element encoded as `T` |
| `enum` | `u8` (values defined per field; unknown value = reject, fail-closed) |

Absent optional string = empty `str` (length 0). There is no null.

## 1b. Additional primitives (receipts)

| Type | Encoding |
|---|---|
| `i64` | 8 bytes LE, two's complement (used by `exit_status`) |
| `bool` | `u8`: 0 = false, 1 = true; any other value = reject |
| `json-str` | a `str` holding canonical JSON (alphabetically sorted keys, no insignificant whitespace) — object-typed policy fields travel as opaque committed text in v1 |
| `digest?` | optional digest: `u32` length 0 (absent) or 32, then the raw bytes |
| `list<digest>` | `u32` count, then each digest as 32 raw bytes |

## 2. Envelope of every wire object

```text
magic     : 4 bytes = "TLG1" (0x54 0x4C 0x47 0x31)
kind      : str     (e.g. "tl-gate.action-intent/1")
body      : bytes   (the object's fields, concatenated per its section below)
```

The **commitment** of a wire object is:

```text
BLAKE3-256( domain_separator_utf8 || full_wire_bytes )
```

where `domain_separator` is the object's domain string (§4). The domain is NOT
part of the wire bytes — it is prepended at hash time, so identical bytes can
never collide across domains.

## 3. `tl-gate.action-intent/1` body

Field order is **fixed**; encoders MUST write exactly this sequence:

```text
 1. schema             str    = "tl-gate.action-intent/1"
 2. principal          str
 3. orchestrator       str
 4. agent_instance     str
 5. session_ref        str
 6. capability         str
 7. target             str
 8. arguments_digest   digest (32 raw bytes)
 9. tool_id            str
10. tool_version       str
11. tool_digest        digest
12. side_effect_class  enum u8: R0=0, R1=1, W1=2, W2=3, W3=4
13. action_id          str
14. chain_id           str
15. attempt            u64
16. parent_digest      bytes  (0 bytes for a chain root, else exactly 32)
```

`intent_digest = BLAKE3-256("TL-GATE/INTENT/v1" || wire_bytes)` — lowercase hex
when rendered as text.

## 3b. Receipt bodies (six mandatory kinds, SPEC §10–11)

Kind strings: `tl-gate.permission-receipt/1`, `…scope…`, `…tool…`, `…execution…`,
`…validation…`, `…final…` (same envelope wrapper as §2). Every body starts with
the **common envelope** in exactly this order (§10.1):

```text
receipt_id str · chain_id str · action_id str · attempt u64 ·
principal_id str · agent_instance_id str · orchestrator_id str ·
subject_digest digest · policy_digest digest ·
causal_parent_digest digest? · previous_receipt_digest digest? ·
local_poh_tick u64 · wall_clock_hint str · nonce str · issuer_ref str
```

`receipt_digest` from §10.1 is NOT encoded — it IS the commitment over the wire
bytes (`BLAKE3(kind_domain || wire)`); a hash cannot contain itself. It appears
only in the JSON mirror, computed.

Then the payload, in the exact field order of its SPEC section:

- **permission** (§11.1): capability str · intent_binding enum{exact_intent=0,
  action_template=1} · action_template_digest digest? · delegation_parent_digest
  digest? · revocation_epoch u64 · max_attempts u64 ·
  required_validation_policy_digest digest
- **scope** (§11.2): capability str · resource_namespace str · target_selectors
  list<str> · allowed_operations list<str> · denied_operations list<str> ·
  network_policy json-str · path_policy json-str · data_classification str ·
  max_payload u64 · max_result u64 · max_attempts u64 · validity_window json-str ·
  revocation_epoch u64 · human_approval_requirement bool
- **tool** (§11.3): tool_id str · tool_version str · binary_or_image_digest digest ·
  connector_id str · connector_version str · input_schema_digest digest ·
  output_schema_digest digest · environment_profile_digest digest ·
  secret_handle_policy json-str · allowed_endpoints list<str> · isolation_profile str
- **execution** (§11.4): intent_digest digest · permission_digest digest ·
  scope_digest digest · tool_digest digest · exact_input_digest digest ·
  execution_environment_digest digest · local_poh_start u64 · local_poh_end u64 ·
  exit_status i64 · output_digest digest · side_effect_digest digest ·
  connector_attestation_digest digest? · bounded_error_digest digest?
- **validation** (§11.5): validator_id str · validator_type enum{deterministic=0,
  schema=1, tests=2, diff_policy=3, security_scanner=4, model_judge=5, human=6,
  external_service=7} · validator_version_or_model_digest str ·
  validation_policy_digest digest · input_result_digest digest ·
  evidence_digests list<digest> · verdict enum{PASS=0, FAIL=1, INCONCLUSIVE=2} ·
  limitations str · human_signer_ref str
- **final** (§11.6): chain_root_digest digest · intent_digest digest ·
  permission_digest digest · scope_digest digest · tool_digest digest ·
  execution_digest digest · validation_digest_set list<digest> ·
  final_result_digest digest · supersedes_digest digest? · local_poh_final_tick u64 ·
  network_finality_proof_ref str

## 4. Domain separators (frozen set, v1)

```text
TL-GATE/INTENT/v1        TL-GATE/PERMISSION/v1   TL-GATE/SCOPE/v1
TL-GATE/TOOL/v1          TL-GATE/EXECUTION/v1    TL-GATE/VALIDATION/v1
TL-GATE/FINAL/v1         TL-GATE/DELEGATION/v1   TL-GATE/REVOCATION/v1
TL-GATE/STOP/v1          TL-GATE/RECOVERY/v1     TL-GATE/ARGS/v1
TL-GATE/POLICY/v1        TL-GATE/CAPSULE/v1
```

## 5. Typed commitment object (the P0-04 answer)

A bare 64-hex digest is NOT a commitment. Wherever a commitment crosses a
repository or API boundary, it travels as:

```json
{
  "schema": "timelayer.commitment/1",
  "domain": "TL-GATE/INTENT/v1",
  "hash_alg": "blake3-256",
  "canonicalization": "tl-gate-wire/1",
  "subject_type": "action_intent",
  "subject_version": "1",
  "payload_digest": "<64 lowercase hex>"
}
```

Consumers MUST reject a commitment whose `schema`, `domain`, `hash_alg`, or
`canonicalization` they do not recognize (fail-closed), and MUST NOT accept a
bare hex string where a commitment is expected.

## 6. Reader rules (fail-closed)

- wrong magic, unknown `kind`, unknown enum value → reject;
- trailing bytes after the last field → reject;
- length prefix pointing past the buffer → reject;
- `digest` field not exactly 32 bytes → reject;
- reader never "fixes up" anything — bytes are either exactly canonical or invalid.

## 7. Test vectors

`testvectors/intent-v1.json` holds input objects with their expected wire hex
and expected `intent_digest`. Any implementation in any language MUST reproduce
them byte-for-byte. The Rust implementation and an independent Python
implementation (`testvectors/check.py`, stdlib + blake3) are both checked in CI
against the same vectors.
