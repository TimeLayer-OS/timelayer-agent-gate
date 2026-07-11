# Security

## Reporting

Email vip.autoservice@gmail.com. Do not open public issues for
vulnerabilities. You will get an answer within 72 hours.

## Threat model (short form)

TL-Gate's security claims are boundary claims, spelled out in SPEC.md §10:
receipt transplant, scope escape, tool substitution, argument mutation after
ALLOW, replay, delegation amplification, output substitution, validator
spoofing, and evidence tampering are in scope and each must have negative
tests before a component leaves Phase 0.

## Honest limitations

- Cooperative mode does not survive a root-compromised host (TB-05). Stronger
  claims require broker-enforced, isolated, or hardware-backed modes.
- A receipt proves attestation and finality of a digest — not that the
  content is true or the business decision was wise.
- Phase 0 code implements intent canonicalization and bound verification
  only; every other command fails closed (STOP), and that is deliberate.
