# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
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

### Current (v0.3.0)

- **macOS Keychain Storage** - Secrets encrypted by OS (macOS only)
- **Touch ID Security Levels** - Biometric authentication supported
- **Session Caching** - 15-minute cache (reduces keychain access)
- Local SQLite database (not encrypted at rest)
- Read-only Binance API access

### Backward Compatibility (v0.2.0)

- API keys in plaintext config.toml supported (file permissions: 0600)
- Migration to keychain is opt-in, not forced
- Mixed storage supported (keychain + TOML)

### Known Limitations

- **Touch ID Prompts**: Native biometric prompts not yet implemented
  - Security levels tracked in database
  - Secrets still OS-encrypted in keychain
  - Full Touch ID enforcement planned for v0.3.1 (FFI bindings)
- **Platform Support**: Keychain features macOS-only
  - Linux: libsecret support planned
  - Windows: Credential Manager support planned

## Security Features

### v0.3.0+ (Current)
- ✅ **macOS Keychain Integration** - OS-encrypted secret storage
- ✅ **Touch ID Security Levels** - Standard, Touch ID Protected, Touch ID Only
- ✅ **No Plaintext Secrets** - Eliminated from config files
- ✅ **Backup Protection** - Keychain items excluded from backups
- ✅ **Migration Wizard** - Safe upgrade from v0.2.0
- ✅ **Automatic Backup** - config.toml.backup before migration

### All Versions
- ✅ Local-first architecture (no cloud dependencies)
- ✅ Read-only exchange API integration
- ✅ Secure secret input methods
- ✅ Shell history protection
- ✅ File permissions enforcement
- ✅ HTTPS-only API communication

## Keychain Security (v0.3.0+)

### macOS Keychain Integration

**Storage:**
- Service: `com.cryptofolio.api-keys`
- Encryption: OS-level (protected by user's login keychain)
- Access: Controlled by macOS Keychain Access policies

**Security Levels:**

1. **Standard** (Default)
   - Protected by macOS login password
   - Accessible when Mac is unlocked
   - Suitable for automation, background jobs
   - Recommended for: Scripts, cron jobs, CI/CD

2. **Touch ID Protected** (Recommended)
   - Requires Touch ID or password for access
   - Best balance of security and usability
   - Recommended for: Interactive use, manual commands

3. **Touch ID Only** (Maximum Security)
   - Biometric authentication required
   - No password fallback
   - Recommended for: High-value accounts, shared computers

**Migration:**
```bash
# Check current secrets
cryptofolio config keychain-status

# Migrate from TOML to Keychain
cryptofolio config migrate-to-keychain

# Upgrade security level
cryptofolio config upgrade-security binance.api_secret --to touchid
```

### Threat Model

| Threat | Before (v0.2) | After (v0.3) | Status |
|--------|--------------|--------------|---------|
| Plaintext storage | ❌ TOML file | ✅ OS-encrypted | ELIMINATED |
| File system access | ❌ Any process (0600) | ✅ Keychain required | PROTECTED |
| Backup exposure | ❌ In backups | ✅ Keychain excluded | PROTECTED |
| Cloud sync risk | ❌ Synced to cloud | ✅ Not synced | PROTECTED |
| Malware | ❌ File read access | ✅ OS keychain access | MITIGATED |
| Unlocked Mac | ❌ File accessible | ⚠️ Partial (Touch ID) | PARTIAL |

---

**Last Updated:** February 21, 2026
