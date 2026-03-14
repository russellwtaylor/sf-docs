# Security

## Reporting a vulnerability

If you believe you've found a security vulnerability in sfdoc, please report it responsibly.

**Preferred:** Open a [GitHub Security Advisory](https://github.com/russellwtaylor/sf-docs/security/advisories/new) (private by default). This allows us to triage and fix the issue before it is disclosed.

**Alternative:** If you prefer not to use GitHub’s advisory flow, you can email the maintainer with details. Please include:

- Description of the vulnerability and how it might be exploited
- Steps to reproduce (if applicable)
- Suggested fix or mitigation (if you have one)

We will acknowledge receipt within a few days and will send a more detailed response once we’ve had a chance to assess the report. We may ask for clarification or more information.

We ask that you do not open a public issue for security-sensitive bugs. We’ll work with you to understand the issue and coordinate a fix and disclosure (e.g. release notes, CVE if appropriate) before any public discussion.

## Supported versions

We release security fixes for the current stable release. If you are on an older version, we encourage upgrading to the latest release.

## Scope

Security issues we care about include (but are not limited to):

- Remote code execution or privilege escalation in the sfdoc binary
- Exposure of API keys or other secrets (e.g. keychain or environment handling)
- Unsafe handling of user-controlled input that could compromise the user’s system or data

General dependency updates are handled via normal development and Dependabot; for dependency-related security concerns, a private report is still appreciated so we can prioritize.

Thank you for helping keep sfdoc and its users safe.
