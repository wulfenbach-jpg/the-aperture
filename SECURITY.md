# Security

This is pre-release software. It accepts untrusted deck URLs, deck text, save/replay files and optional local-model responses. No security certification is claimed.

**Do not put secrets or exploit payloads containing private data in public issues.** Use this repository's GitHub **Security → Report a vulnerability** private reporting flow. If private reporting is unavailable, open a minimal issue asking the maintainer for a private channel, without vulnerability details or credentials.

Reports should identify the affected commit, operating system, reproduction conditions and impact. Maintainers should reproduce the defect, add a regression test, review the patch and describe the supported versions. No response-time guarantee is promised.

## Boundaries

* Keep model services loopback-only unless the user separately configures deployment.
* Validate URL schemes/hosts/redirects, payload sizes, quantities, action legality and replay schemas.
* Run networking, disk and model calls off the native frame thread; drop stale results.
* Never send hidden opponent information or full authoritative state to a normal-play policy.
* Never serialize GitHub/model credentials into the browser board, logs or game evidence.
