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

use tl_gate_core::{ActionIntent, StopCode, INTENT_DOMAIN_V1};
use tl_gate_verifier_bridge::{verify, VerifierVerdict};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("intent") if args.get(2).map(String::as_str) == Some("digest") => {
            cmd_intent_digest(args.get(3));
        }
        Some("verify") => cmd_verify(&args),
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
                "init", "serve", "adapters", "tools", "policy", "check", "execute",
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
    println!("  tl-gate spec-version                                    protocol identifiers of this build");
    println!();
    println!("SPECIFIED, NOT YET IMPLEMENTED (exit 1, fail-closed):");
    println!("  init serve adapters tools policy check execute validate");
    println!("  finalize status stop recover export audit gc");
    println!();
    println!("Spec: SPEC.md (EN) / SPEC.ru.md (RU, normative for now).");
}
