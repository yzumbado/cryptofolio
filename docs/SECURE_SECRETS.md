# Secure Secret Handling

**Version:** 0.2
**Status:** Implemented
**Security Level:** ⭐⭐⭐⭐ (Good - Plaintext with warnings)

---

## Overview

Cryptofolio v0.2 implements secure secret input to prevent API keys and secrets from appearing in shell history. This document explains how to use the new `config set-secret` command and the security considerations.

## The Problem

**Before v0.2:**
```bash
$ cryptofolio config set binance.api_secret "abc123secret456"
$ history
  ...
  1042  cryptofolio config set binance.api_secret "abc123secret456"  # ⚠️ EXPOSED!
```

**Security Issues:**
- Secrets stored in shell history (`~/.bash_history`, `~/.zsh_history`)
- Visible in process list while command runs
- Easy to accidentally share in documentation/Slack

## The Solution

### New Command: `config set-secret`

```bash
# Interactive (recommended for first-time setup)
$ cryptofolio config set-secret binance.api_secret

  ╔═══════════════════════════════════════════════════════════════╗
  ║                     SECURITY NOTICE                           ║
  ╚═══════════════════════════════════════════════════════════════╝

  Cryptofolio v0.2 stores API keys in PLAINTEXT on your filesystem:
    ~/.config/cryptofolio/config.toml

  ⚠️  IMPORTANT: Use READ-ONLY API keys ONLY

  When creating your Binance API key:
    ✅ Enable: 'Enable Reading'
    ❌ DISABLE: 'Enable Spot & Margin Trading'
    ❌ DISABLE: 'Enable Withdrawals'
    ❌ DISABLE: 'Enable Internal Transfer'

  Why? If your computer is compromised, attackers could:
    • Read your API key from config.toml
    • Use WRITE permissions to steal funds

  With READ-ONLY keys, they can only:
    • View your portfolio (no financial loss)

  🔮 Coming in v0.3: Encrypted keychain storage
     (macOS Keychain, Windows Credential Manager, Linux Secret Service)

  I understand, continue? [y/N] y

Enter secret (hidden): ********

✓ Secret saved to ~/.config/cryptofolio/config.toml

  ⚠️  Remember: Use READ-ONLY API keys only!
```

## Usage Methods

### 1. Interactive (Hidden Input)

**Best for:** First-time setup, manual configuration

```bash
cryptofolio config set-secret binance.api_secret
# Prompts for secret (hidden input)
```

**Pros:**
- ✅ Most user-friendly
- ✅ No shell history
- ✅ Not visible in process list
- ✅ Familiar UX (like sudo password)

### 2. From Stdin (Piped)

**Best for:** Automation, scripts, CI/CD

```bash
echo "abc123secret456" | cryptofolio config set-secret binance.api_secret
```

**Pros:**
- ✅ Works in non-interactive contexts
- ✅ Can pipe from password managers

**Integration with password managers:**
```bash
# 1Password CLI
op read "op://vault/Binance/api_secret" | cryptofolio config set-secret binance.api_secret

# pass (Unix password manager)
pass show binance/api_secret | cryptofolio config set-secret binance.api_secret

# macOS Keychain
security find-generic-password -w -a cryptofolio -s binance | cryptofolio config set-secret binance.api_secret
```

### 3. From File

**Best for:** Deployment scripts, containerized environments

```bash
cryptofolio config set-secret binance.api_secret --secret-file ~/.secrets/binance.key
```

**Setup:**
```bash
# Create secure secret file
echo "abc123secret456" > ~/.secrets/binance.key
chmod 600 ~/.secrets/binance.key  # Owner read/write only
```

**Pros:**
- ✅ Works in automation
- ✅ Can set file permissions

**Cons:**
- ⚠️ Secret still on disk in plaintext
- ⚠️ Risk of accidental git commit

### 4. From Environment Variable

**Best for:** Docker containers, CI/CD pipelines

```bash
export BINANCE_API_SECRET="abc123secret456"
cryptofolio config set-secret binance.api_secret --from-env BINANCE_API_SECRET
```

**Docker example:**
```dockerfile
# Dockerfile
FROM rust:latest
ENV BINANCE_API_SECRET=${BINANCE_API_SECRET}
RUN cryptofolio config set-secret binance.api_secret --from-env BINANCE_API_SECRET
```

## Security Protection Layers

| Layer | Protection | Version |
|-------|-----------|---------|
| **Shell History** | ✅ Eliminated (hidden input) | v0.2 |
| **Process List** | ✅ Eliminated (no CLI args) | v0.2 |
| **File Permissions** | ✅ Auto-fixed (0600) | v0.2 |
| **User Education** | ✅ Clear warnings | v0.2 |
| **API Restrictions** | ✅ Read-only requirement | v0.2 |
| **At-Rest Encryption** | ⏳ Keychain integration | v0.3 |

## File Permissions

### Automatic Protection

Cryptofolio automatically sets config file permissions to `0600` (owner read/write only) on Unix systems.

```bash
$ ls -la ~/.config/cryptofolio/config.toml
-rw-------  1 user  group  512 Feb 16 10:30 config.toml  # ✓ Secure (0600)
```

### Manual Verification

```bash
# Check permissions
ls -la ~/.config/cryptofolio/config.toml

# Fix if needed (Unix/macOS/Linux)
chmod 600 ~/.config/cryptofolio/config.toml
```

### Windows

On Windows, ensure only your user account has read access:

1. Right-click `config.toml` → Properties
2. Security tab → Advanced
3. Disable inheritance
4. Remove all users except your account
5. Grant only Read and Write to your account

## Binance API Key Setup

### Creating a Read-Only API Key

1. **Login to Binance:** https://www.binance.com
2. **Navigate:** Profile → API Management
3. **Create API Key:**
   - Name: "Cryptofolio Read-Only"
   - API restrictions: **Enable Reading** ✅
   - **DISABLE** all other permissions:
     - ❌ Enable Spot & Margin Trading
     - ❌ Enable Withdrawals
     - ❌ Enable Internal Transfer
     - ❌ Enable Futures
     - ❌ Enable Margin

4. **Optional (Recommended):** Add IP restrictions
   - Whitelist your home/office IP
   - More secure but less flexible

5. **Save API Key and Secret:**
   ```bash
   cryptofolio config set-secret binance.api_key
   # Enter API key (hidden)

   cryptofolio config set-secret binance.api_secret
   # Enter API secret (hidden)
   ```

### Why Read-Only?

**If your computer is compromised:**

| API Permissions | Attacker Can |
|-----------------|--------------|
| **READ-ONLY** | View portfolio | No financial loss ✅ |
| **TRADING** | Make trades | Manipulate markets, fees ❌ |
| **WITHDRAWALS** | Steal all funds | Total loss ❌❌❌ |

**Cryptofolio only needs READ permissions to:**
- Fetch current prices
- View portfolio balances
- Sync holdings

**You should NEVER grant WRITE permissions for portfolio tracking.**

## Migration from v0.1

If you previously set API keys using `config set`:

1. **Warning Added:** The old command now warns you:
   ```bash
   $ cryptofolio config set binance.api_secret "secret"

   ⚠️  WARNING: Setting secrets via command line arguments is insecure!
   ⚠️  Your secret will be visible in shell history.

      Use this instead:
      cryptofolio config set-secret binance.api_secret

   Continue anyway? [y/N]
   ```

2. **Recommended Action:**
   - Delete old secrets from shell history
   - Re-set using `config set-secret`

3. **Clean Shell History:**
   ```bash
   # Bash
   history -d <line_number>  # Delete specific line
   history -c                # Clear all history (careful!)

   # Zsh
   fc -W; history -p <pattern>  # Remove matching entries
   ```

## Security Limitations (v0.2)

### Current Constraints

1. **Plaintext Storage**
   - Secrets stored in `~/.config/cryptofolio/config.toml`
   - Protected by file permissions (0600)
   - NOT encrypted at rest

2. **Local Machine Only**
   - No cloud sync
   - Secrets lost if disk fails (backup responsibility)

3. **No Audit Log**
   - No tracking of who accessed secrets
   - No notification on secret access

### Planned Improvements (v0.3)

- **Encrypted Keychain Storage:**
  - macOS: Keychain
  - Windows: Credential Manager
  - Linux: Secret Service API
  - Encrypted at rest by OS

- **Secret Rotation:**
  - Automatic expiration warnings
  - Easy rotation workflow

## Troubleshooting

### "Permission denied" on config.toml

```bash
# Fix permissions
chmod 600 ~/.config/cryptofolio/config.toml
```

### "Empty secret provided"

- Ensure no extra whitespace
- Check file encoding (UTF-8)
- Verify environment variable is set

### Interactive mode not working

- Check if running in TTY: `tty`
- Try piping instead: `echo "secret" | cryptofolio config set-secret key`

### File still world-readable after set-secret

- Check umask: `umask` (should be 022 or stricter)
- Manually fix: `chmod 600 ~/.config/cryptofolio/config.toml`

## Best Practices

1. **✅ DO:** Use read-only API keys
2. **✅ DO:** Rotate keys periodically (every 90 days)
3. **✅ DO:** Use IP restrictions when possible
4. **✅ DO:** Backup config.toml securely
5. **✅ DO:** Use different keys for testnet vs mainnet

6. **❌ DON'T:** Grant trading permissions
7. **❌ DON'T:** Enable withdrawals
8. **❌ DON'T:** Share API keys
9. **❌ DON'T:** Commit config.toml to git
10. **❌ DON'T:** Store secrets in environment variables permanently

## Testing

Run secret handling tests:

```bash
# Unit tests
cargo test config::secrets

# Integration test
echo "test-secret" | ./target/release/cryptofolio config set-secret test.key
cryptofolio config show  # Should show test.key: ***configured***
```

## Related Documentation

- [CLI Guidelines Review](../CLI_GUIDELINES_REVIEW.md) - Security fixes context
- [Conversational CLI](CONVERSATIONAL_CLI.md) - AI features
- [README](../README.md) - Main documentation

## Support

**Report security issues:** security@yourcompany.com (or GitHub private security advisory)

**General questions:** https://github.com/yzumbado/cryptofolio/issues

---

**Built with 🦀 Rust and 🔒 Security in mind.**
