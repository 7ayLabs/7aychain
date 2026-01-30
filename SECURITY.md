# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### Reporting Process

1. **DO NOT** disclose the vulnerability publicly until it has been addressed
2. Email security findings to: **security@7aylabs.com**
3. Include the following information:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Any suggested remediation

### Response Timeline

| Phase | Timeline |
|-------|----------|
| Initial Response | Within 24 hours |
| Triage & Assessment | Within 72 hours |
| Patch Development | Severity dependent |
| Public Disclosure | After patch release |

### Severity Classification

| Severity | Description | Examples |
|----------|-------------|----------|
| **Critical** | Immediate network risk | Consensus bypass, fund theft |
| **High** | Significant impact | State corruption, DoS vectors |
| **Medium** | Limited impact | Information disclosure |
| **Low** | Minimal impact | Minor issues |

## Security Measures

### Protocol Invariants

The 7aychain implementation enforces 78 protocol invariants (INV1-78) as defined in the [PoP Specification](https://github.com/7ayLabs/7ay-presence). Key security invariants include:

- **INV43**: Chain binding for replay protection
- **INV44**: Key destruction attestation
- **INV45**: Discovery rate limiting
- **INV46-49**: Validator economic security
- **INV57-60**: Recovery and governance controls

### Code Security

- All code undergoes clippy analysis with strict security lints
- No `unsafe` code without explicit security review
- Saturating/checked arithmetic for all numeric operations
- Constant-time comparisons for cryptographic operations

### Audit Status

| Audit | Status | Date |
|-------|--------|------|
| Internal Review | Ongoing | - |
| External Audit | Planned | TBD |

## Bug Bounty Program

A bug bounty program will be announced prior to mainnet launch.

## Security Contacts

- **Email**: security@7aylabs.com
- **PGP Key**: Available upon request

---

Thank you for helping keep 7aychain secure.
