# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Masday, please report it responsibly.

**Do NOT open a public GitHub issue.**

Instead, please:

1. **Email**: Send details to security@dayartcrew.com (replace with actual)
2. **GitHub**: Use [Security Advisories](https://github.com/dayartcrew-web/masday-workflow-rust/security/advisories/new)

Include:
- Type of vulnerability (e.g., injection, XSS, privilege escalation)
- Full paths of source file(s) related to the vulnerability
- Step-by-step instructions to reproduce
- Potential impact

## Response Time

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 7 days
- **Fix**: Depends on severity — critical within 72 hours

## Responsible Disclosure

We ask that you:
- Give us reasonable time to fix the issue before public disclosure
- Do not access or modify user data
- Do not degrade service quality

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.3.x   | ✅ Active |
| < 0.3   | ❌ End of life |

## Scope

**In scope:**
- Masday CLI binary and source code
- MCP server implementation
- Install scripts

**Out of scope:**
- Third-party dependencies (report upstream)
- Social engineering
- Denial of service
