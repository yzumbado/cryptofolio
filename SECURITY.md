# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.6.x   | :white_check_mark: |
| < 0.6.0 | :x:                |

## Reporting a Vulnerability

Do **NOT** open a public GitHub issue. Use a
[GitHub private security advisory](https://github.com/yzumbado/cryptofolio/security/advisories/new)
— the report stays private while we coordinate a fix.

- **Initial response:** within 48 hours
- **Status update:** within 7 days
- Reporters are credited unless they prefer to remain anonymous

## Secrets Handling

- All API keys and xpub keys live in the **macOS Keychain**
  (service `com.cryptofolio.api-keys`) — never in `config.toml`, the database,
  logs, or shell history.
- Use `cryptofolio config set-secret <key>` (hidden input). Never `config set`
  for credentials.
- Use **read-only** exchange API keys. The app never signs or broadcasts.
- Suspected key exposure should be reported and the key rotated.

## Data at Rest

- The ledger is a local SQLite database in the app config directory
  (see `cryptofolio config show`). No cloud sync, no telemetry.
- Config files are created with 0600 permissions.
- The database is not encrypted at rest — protect the machine accordingly.

## Watch-Only Design

- Private keys and seed phrases are rejected at input
  (WIF / raw key / BIP-39 detection in `src/blockchain/security.rs`).
- No signing, no broadcasting, no withdrawal capability — by design.

## Known Limitations

- Keychain integration is macOS-only (Linux/Windows backends planned).
- The active Keychain backend uses the `security` CLI; Touch ID security levels
  are tracked but may degrade to standard access.
- Blockchain sync stores wallet addresses (public on-chain data) in the local
  ledger and audit log.

---

**Last updated:** 2026-10-03
