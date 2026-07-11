#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Independent TL-GATE-WIRE/v1 implementation (Python) checked against the
Rust-generated vectors in intent-v1.json. Any language implementing the format
must reproduce wire_hex and intent_digest byte-for-byte.

Requires: pip install blake3
Run:      python3 testvectors/check.py
"""
import json
import pathlib
import struct
import sys

try:
    from blake3 import blake3
except ImportError:
    sys.exit("нужен пакет blake3:  pip install blake3")

MAGIC = b"TLG1"
DOMAIN = b"TL-GATE/INTENT/v1"
SIDE_EFFECT = {"R0": 0, "R1": 1, "W1": 2, "W2": 3, "W3": 4}


def put_str(s: str) -> bytes:
    b = s.encode("utf-8")
    return struct.pack("<I", len(b)) + b


def put_bytes(b: bytes) -> bytes:
    return struct.pack("<I", len(b)) + b


def hex32(s: str) -> bytes:
    if len(s) != 64 or s != s.lower():
        raise ValueError(f"digest field must be 64 lowercase hex: {s!r}")
    return bytes.fromhex(s)


def encode_intent_v1(i: dict) -> bytes:
    body = b"".join([
        put_str(i["schema"]),
        put_str(i["principal"]),
        put_str(i["orchestrator"]),
        put_str(i["agent_instance"]),
        put_str(i["session_ref"]),
        put_str(i["capability"]),
        put_str(i["target"]),
        hex32(i["arguments_digest"]),
        put_str(i["tool_id"]),
        put_str(i["tool_version"]),
        hex32(i["tool_digest"]),
        bytes([SIDE_EFFECT[i["side_effect_class"]]]),
        put_str(i["action_id"]),
        put_str(i["chain_id"]),
        struct.pack("<Q", i["attempt"]),
        put_bytes(b"" if not i["parent_digest"] else hex32(i["parent_digest"])),
    ])
    return MAGIC + put_str("tl-gate.action-intent/1") + put_bytes(body)


def intent_digest(i: dict) -> str:
    return blake3(DOMAIN + encode_intent_v1(i)).hexdigest()


def main() -> int:
    data = json.loads((pathlib.Path(__file__).parent / "intent-v1.json").read_text("utf-8"))
    ok = True
    for v in data["vectors"]:
        wire = encode_intent_v1(v["intent"]).hex()
        digest = intent_digest(v["intent"])
        w = "OK" if wire == v["wire_hex"] else "MISMATCH"
        d = "OK" if digest == v["intent_digest"] else "MISMATCH"
        ok &= (w == "OK" and d == "OK")
        print(f"{v['name']}: wire {w} · digest {d}")
    print("CROSS-LANGUAGE:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
