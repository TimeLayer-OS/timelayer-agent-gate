//! Bridge to the official `timelayer-verifier` binary (spec §8.6).
//!
//! TL-Gate never re-implements verification and never verifies UNBOUND:
//! `verify` requires the expected subject digest and passes it via `--expect`.
//! A receipt that is valid in itself but attests a different subject is
//! NOT_VALID for this gate — that is the whole point (see the 2026-07-11
//! audit, P0-01 "receipt transplant").

use std::path::Path;
use std::process::Command;

/// The four verdicts of the verifier interface (spec §8.6). Anything that is
/// not `ValidFinal` stops the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifierVerdict {
    ValidFinal,
    NotValid,
    Unverifiable,
    /// Bridge-level failure: binary missing, cannot execute, no --expect
    /// support. Fail-closed: treated as STOP by every caller.
    Error(String),
}

/// Verify a receipt pair BOUND to `expected_subject_hex`.
///
/// Only exact stdout `VALID FINAL` + exit code 0 count as valid. Unexpected
/// output, a nonzero exit, a missing binary — all fail closed.
pub fn verify(
    verifier: &Path,
    cert: &Path,
    bundle: &Path,
    expected_subject_hex: &str,
) -> VerifierVerdict {
    if expected_subject_hex.len() != 64
        || !expected_subject_hex.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return VerifierVerdict::Error(format!(
            "expected subject must be 64 hex chars, got {} chars",
            expected_subject_hex.len()
        ));
    }
    if !supports_expect(verifier) {
        return VerifierVerdict::Error(
            "installed timelayer-verifier does not support --expect (need v2.0.0+); \
             refusing to verify unbound (fail-closed)"
                .into(),
        );
    }
    let out = match Command::new(verifier)
        .arg("verify")
        .arg(cert)
        .arg(bundle)
        .arg("--expect")
        .arg(expected_subject_hex)
        .output()
    {
        Ok(o) => o,
        Err(e) => return VerifierVerdict::Error(format!("cannot run verifier: {e}")),
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stdout = stdout.trim();
    if out.status.success() && stdout == "VALID FINAL" {
        VerifierVerdict::ValidFinal
    } else if stdout.starts_with("UNVERIFIABLE") {
        VerifierVerdict::Unverifiable
    } else {
        VerifierVerdict::NotValid
    }
}

/// Probe once whether the verifier knows `--expect`. Probe failure = false
/// (fail-closed).
pub fn supports_expect(verifier: &Path) -> bool {
    Command::new(verifier)
        .args(["verify", "--help"])
        .output()
        .map(|o| {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            text.contains("--expect")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn malformed_expected_digest_is_error() {
        let v = verify(
            &PathBuf::from("/nonexistent"),
            &PathBuf::from("c"),
            &PathBuf::from("b"),
            "not-hex",
        );
        assert!(matches!(v, VerifierVerdict::Error(_)));
    }

    #[test]
    fn missing_verifier_is_error_not_valid_final() {
        let v = verify(
            &PathBuf::from("/nonexistent-verifier-binary"),
            &PathBuf::from("c"),
            &PathBuf::from("b"),
            &"0".repeat(64),
        );
        assert!(matches!(v, VerifierVerdict::Error(_)));
    }
}
