//! tl-gate — CLI of the TimeLayer Agent Gate.
//!
//! Phase 0 honesty: only what is really implemented answers; every command
//! the spec defines but this build does not implement exits with
//! STOP(NOT_IMPLEMENTED) and code 1 — fail-closed by construction, never a
//! silent pretend-ALLOW.
//!
//! Implemented today:
//!   tl-gate intent digest <proposal.json>   canonical ActionIntent + commitment
//!   tl-gate verify <cert> <bundle> --expect <digest>   bound offline verification
//!   tl-gate spec-version                    protocol identifiers of this build
//!
//! Exit codes (spec §25): 0 success/ALLOW, 1 STOP/not valid, 2 malformed
//! input, 3 unavailable dependency.

use std::path::PathBuf;
use std::process::exit;

use tl_gate_core::gate::{pre_gate, BoundReceipt, GateInput, ReceiptVerifier};
use tl_gate_core::normalizer::{normalize, Proposal};
use tl_gate_core::{ActionIntent, GateDecision, StopCode, INTENT_DOMAIN_V1};
use tl_gate_verifier_bridge::{verify, VerifierVerdict};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("intent") if args.get(2).map(String::as_str) == Some("digest") => {
            cmd_intent_digest(args.get(3));
        }
        Some("verify") => cmd_verify(&args),
        Some("check") => cmd_check(&args),
        Some("spec-version") => {
            println!("tl-gate {} (Phase 0)", env!("CARGO_PKG_VERSION"));
            println!("intent domain : {INTENT_DOMAIN_V1}");
            println!("wire format   : TL-GATE-WIRE/v1 (frozen 2026-07-11, schemas/TL-GATE-WIRE-v1.md)");
            exit(0);
        }
        Some("help") | Some("--help") | Some("-h") | None => {
            usage();
            exit(0);
        }
        Some(other) => {
            // Every spec-defined command that is not implemented yet answers
            // honestly and fails closed.
            let known = [
                "init", "serve", "adapters", "tools", "policy", "execute",
                "validate", "finalize", "status", "stop", "recover", "export", "audit", "gc",
            ];
            if known.contains(&other) {
                eprintln!("STOP({}): '{other}' is specified but not implemented in Phase 0 — refusing to pretend", StopCode::NotImplemented);
                exit(1);
            }
            eprintln!("tl-gate: unknown command '{other}'");
            usage();
            exit(2);
        }
    }
}

// ── check: the Pre-Execution Gate over a receipt workspace ─────────────────
//
// Workspace layout (one action), under <workspace>/receipts/:
//   {permission,scope,tool}.tlgw            wire bytes of each receipt
//   {permission,scope,tool}.tlcert/.tlbundle   TimeLayer attestation pair
//
// tl-gate check <workspace> <proposal.json> [--verifier <path>]
// Exit: 0 ALLOW · 1 STOP · 2 malformed input · 3 verifier unavailable.

struct RealVerifier {
    verifier: PathBuf,
    tmp: PathBuf,
}

impl ReceiptVerifier for RealVerifier {
    fn verify_bound(&self, cert: &[u8], bundle: &[u8], expected: &str) -> bool {
        let c = self.tmp.join(format!("{expected}.tlcert"));
        let b = self.tmp.join(format!("{expected}.tlbundle"));
        if std::fs::write(&c, cert).is_err() || std::fs::write(&b, bundle).is_err() {
            return false;
        }
        let out = matches!(verify(&self.verifier, &c, &b, expected), VerifierVerdict::ValidFinal);
        let _ = std::fs::remove_file(&c);
        let _ = std::fs::remove_file(&b);
        out
    }
}

fn load_bound(dir: &std::path::Path, name: &str) -> Result<BoundReceipt, String> {
    let read = |ext: &str| {
        std::fs::read(dir.join(format!("{name}.{ext}")))
            .map_err(|e| format!("cannot read receipts/{name}.{ext}: {e}"))
    };
    Ok(BoundReceipt { wire: read("tlgw")?, cert: read("tlcert")?, bundle: read("tlbundle")? })
}

fn cmd_check(args: &[String]) {
    let (Some(workspace), Some(proposal_path)) = (args.get(2), args.get(3)) else {
        eprintln!("usage: tl-gate check <workspace> <proposal.json> [--verifier <path>]");
        exit(2);
    };
    let verifier = flag_value(args, "--verifier")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("TL_VERIFIER").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("timelayer-verifier"));
    if !verifier.exists() {
        eprintln!("STOP(VERIFIER_UNAVAILABLE): verifier not found at {}", verifier.display());
        exit(3);
    }

    let text = match std::fs::read_to_string(proposal_path) {
        Ok(t) => t,
        Err(e) => { eprintln!("cannot read {proposal_path}: {e}"); exit(2); }
    };
    let proposal: Proposal = match serde_json::from_str(&text) {
        Ok(p) => p,
        Err(e) => { eprintln!("malformed proposal: {e}"); exit(2); }
    };
    let intent = match normalize(&proposal) {
        Ok(i) => i,
        Err(e) => { eprintln!("STOP(SCHEMA_MISMATCH): {e}"); exit(1); }
    };

    let receipts_dir = std::path::Path::new(workspace).join("receipts");
    let input = match (
        load_bound(&receipts_dir, "permission"),
        load_bound(&receipts_dir, "scope"),
        load_bound(&receipts_dir, "tool"),
    ) {
        (Ok(p), Ok(s), Ok(t)) => GateInput { permission: p, scope: s, tool: t },
        (p, s, t) => {
            for e in [p.err(), s.err(), t.err()].into_iter().flatten() {
                eprintln!("STOP(NO_RECEIPT): {e}");
            }
            exit(1);
        }
    };

    let tmp = std::env::temp_dir().join(format!("tl-gate-{}", std::process::id()));
    if std::fs::create_dir_all(&tmp).is_err() {
        eprintln!("STOP(VERIFIER_UNAVAILABLE): cannot create tmp dir");
        exit(3);
    }
    let rv = RealVerifier { verifier, tmp: tmp.clone() };
    let decision = pre_gate(&intent, &input, &rv);
    let _ = std::fs::remove_dir_all(&tmp);

    match decision {
        GateDecision::Allow => {
            println!("ALLOW");
            println!("  intent  : {}", intent.intent_digest().unwrap_or_default());
            println!("  action  : {}", intent.action_id);
            println!("  target  : {}", intent.target);
            exit(0);
        }
        GateDecision::Stop(code) => {
            println!("STOP({code})");
            println!("  action  : {}", intent.action_id);
            exit(1);
        }
    }
}

fn cmd_intent_digest(path: Option<&String>) {
    let Some(path) = path else {
        eprintln!("usage: tl-gate intent digest <intent.json>");
        exit(2);
    };
    let text = match std::fs::read_to_string(PathBuf::from(path)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            exit(2);
        }
    };
    let intent: ActionIntent = match serde_json::from_str(&text) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("malformed ActionIntent: {e}");
            exit(2);
        }
    };
    match intent.intent_digest() {
        Ok(d) => {
            println!("{d}");
            exit(0);
        }
        Err(e) => {
            eprintln!("malformed ActionIntent: {e}");
            exit(2);
        }
    }
}

fn cmd_verify(args: &[String]) {
    // tl-gate verify <cert> <bundle> --expect <digest> [--verifier <path>]
    let expect = flag_value(args, "--expect");
    let verifier = flag_value(args, "--verifier")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("TL_VERIFIER").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("timelayer-verifier"));
    let (Some(cert), Some(bundle)) = (args.get(2), args.get(3)) else {
        eprintln!("usage: tl-gate verify <cert.tlcert> <bundle.tlbundle> --expect <digest>");
        exit(2);
    };
    let Some(expect) = expect else {
        // Unbound verification is not a thing in TL-Gate. Ever.
        eprintln!("STOP(SUBJECT_MISMATCH): --expect is mandatory — TL-Gate never verifies a receipt unbound from its subject");
        exit(1);
    };
    match verify(
        &verifier,
        &PathBuf::from(cert),
        &PathBuf::from(bundle),
        &expect,
    ) {
        VerifierVerdict::ValidFinal => {
            println!("VALID FINAL (bound to {expect})");
            exit(0);
        }
        VerifierVerdict::NotValid => {
            eprintln!("STOP(RECEIPT_NOT_VALID)");
            exit(1);
        }
        VerifierVerdict::Unverifiable => {
            eprintln!("STOP(RECEIPT_NOT_VALID): receipt does not attest the expected digest");
            exit(1);
        }
        VerifierVerdict::Error(e) => {
            eprintln!("STOP(VERIFIER_UNAVAILABLE): {e}");
            exit(3);
        }
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}

fn usage() {
    println!("tl-gate — TimeLayer Agent Gate CLI (Phase 0)");
    println!();
    println!("IMPLEMENTED:");
    println!("  tl-gate intent digest <intent.json>                     canonical commitment (BLAKE3, {INTENT_DOMAIN_V1})");
    println!("  tl-gate verify <cert> <bundle> --expect <digest>        bound offline verification, fail-closed");
    println!("  tl-gate check <workspace> <proposal.json>               Pre-Execution Gate: permission+scope+tool,");
    println!("                                                          bound, chain-linked; ALLOW or STOP(reason)");
    println!("  tl-gate spec-version                                    protocol identifiers of this build");
    println!();
    println!("SPECIFIED, NOT YET IMPLEMENTED (exit 1, fail-closed):");
    println!("  init serve adapters tools policy execute validate");
    println!("  finalize status stop recover export audit gc");
    println!();
    println!("Spec: SPEC.md (EN) / SPEC.ru.md (RU, normative for now).");
}
