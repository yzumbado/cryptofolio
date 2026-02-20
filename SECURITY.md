# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| 0.1.x   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please follow these steps:

### How to Report

1. **Do NOT** create a public GitHub issue
2. Email security details to: [your-email@example.com]
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Initial Response:** Within 48 hours
- **Status Update:** Within 7 days
- **Fix Timeline:** Varies based on severity

### Disclosure Policy

- We will acknowledge receipt of your report
- We will investigate and validate the issue
- We will develop and test a fix
- We will release a security update
- We will credit you (unless you prefer to remain anonymous)

## Security Best Practices

### API Keys

- **Always** use READ-ONLY API keys
- Use `config set-secret` command (never `config set`)
- Never commit API keys to version control
- Store keys securely using macOS Keychain (v0.3+)

### Configuration Files

- Config files are automatically set to 0600 permissions on Unix
- Regularly backup your database
- Keep your Rust toolchain updated

### Database

- SQLite database stored locally at `~/.config/cryptofolio/`
- File permissions enforced automatically
- No remote access or cloud sync

## Known Security Considerations

### Current (v0.2.0)

- API keys stored in plaintext in config.toml (file permissions: 0600)
- Local SQLite database (not encrypted at rest)
- Read-only Binance API access

### Planned (v0.3.0)

- Encrypted keychain storage (macOS Keychain)
- Automatic migration from plaintext config

## Security Features

- ✅ Local-first architecture (no cloud dependencies)
- ✅ Read-only exchange API integration
- ✅ Secure secret input methods
- ✅ Shell history protection
- ✅ File permissions enforcement
- ✅ HTTPS-only API communication

---

**Last Updated:** February 19, 2026
