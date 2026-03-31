# Secure Credential Storage Options Analysis

**Date:** 2026-03-28
**Issue:** macOS 26.3.1 blocks ad-hoc signed binaries with keychain entitlements (exit code 137 / SIGKILL)
**Requirement:** Secure storage for exchange API keys (Binance, etc.) - plaintext config is unacceptable

---

## 🔍 Root Cause Analysis

**What's Happening:**
```
codesign --sign - --entitlements keychain.plist binary
./binary --version
[1] killed
Exit code: 137 (SIGKILL from macOS security)
```

**Why:**
- macOS 26+ enforces stricter code signing requirements
- Ad-hoc signatures (--sign -) cannot use keychain-access-groups entitlement
- Even debug builds are affected when signed with keychain entitlements
- This is a security feature to prevent unauthorized keychain access

**Current State:**
- ✅ Unsigned binary runs fine
- ❌ Unsigned binary cannot access keychain (no entitlements)
- ❌ Signed binary with entitlements gets killed
- ✅ We use `keyring` crate for cross-platform keychain access

---

## 🎯 Evaluation Criteria

Each option evaluated on:
1. **Security** - Protection level for API keys
2. **Cost** - Financial investment required
3. **UX** - User experience and ease of use
4. **Cross-platform** - Works on macOS/Linux/Windows
5. **Implementation** - Development effort required

---

## 💡 Option 1: Apple Developer ID Certificate

**Description:** Pay $99/year for Apple Developer Program membership, get real code signing certificate

**How it Works:**
```bash
# One-time setup (after enrolling)
1. Download Developer ID certificate from Apple
2. Sign: codesign --sign "Developer ID Application: Your Name" --entitlements keychain.plist binary
3. Binary runs without being killed
4. Keychain access works with entitlements
```

**Pros:**
- ✅ Solves the signing issue permanently
- ✅ Enables keychain storage (most secure on macOS)
- ✅ Allows distribution to other users
- ✅ Professional solution
- ✅ Can notarize the app for Gatekeeper

**Cons:**
- ❌ Costs $99/year
- ❌ Requires Apple ID and enrollment process
- ❌ Only helps on macOS (still need solution for Linux/Windows)
- ❌ Takes 1-2 days to get certificate after enrollment

**Security:** ⭐⭐⭐⭐⭐ (Best on macOS)
**Cost:** $99/year
**UX:** ⭐⭐⭐⭐⭐ (Seamless after setup)
**Cross-platform:** ❌ (macOS only)
**Implementation:** ⭐⭐⭐⭐ (Easy, just signing changes)

**Verdict:** Best option if distributing to others or building a product, but overkill for personal use.

---

## 💡 Option 2: Use `security` Command Instead of Keychain APIs

**Description:** Use macOS `security` command-line tool to store/retrieve secrets instead of Security Framework APIs

**How it Works:**
```rust
// Instead of keyring crate:
use std::process::Command;

fn store_secret(service: &str, account: &str, secret: &str) -> Result<()> {
    Command::new("security")
        .args(&["add-generic-password", "-s", service, "-a", account, "-w", secret])
        .output()?;
    Ok(())
}

fn get_secret(service: &str, account: &str) -> Result<String> {
    let output = Command::new("security")
        .args(&["find-generic-password", "-s", service, "-a", account, "-w"])
        .output()?;
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
```

**Pros:**
- ✅ No code signing required
- ✅ Uses secure macOS keychain
- ✅ Free solution
- ✅ No Apple Developer account needed
- ✅ Can prompt for Touch ID with additional flags

**Cons:**
- ❌ Requires user to manually approve first access (popup dialog)
- ❌ macOS-specific (need different solution for Linux/Windows)
- ❌ Parsing command output is fragile
- ❌ Error handling more complex

**Security:** ⭐⭐⭐⭐⭐ (Uses system keychain)
**Cost:** Free
**UX:** ⭐⭐⭐ (One-time approval dialog per secret)
**Cross-platform:** ❌ (Need platform-specific implementations)
**Implementation:** ⭐⭐⭐ (Moderate refactoring needed)

**Verdict:** Good free alternative to Developer ID, but requires code changes.

---

## 💡 Option 3: GPG-Encrypted Configuration File

**Description:** Encrypt config.toml with GPG, decrypt on read

**How it Works:**
```bash
# User setup
gpg --gen-key  # Create key if needed
echo "api_key=secret" | gpg --encrypt --recipient your@email.com > config.gpg

# Application reads:
gpg --decrypt config.gpg  # Prompts for passphrase or uses agent
```

**Pros:**
- ✅ Cross-platform (GPG available everywhere)
- ✅ No code signing required
- ✅ Free and open source
- ✅ Can use GPG agent for caching
- ✅ Industry-standard encryption

**Cons:**
- ❌ Requires GPG installed (not default on macOS/Windows)
- ❌ User must manage GPG keys
- ❌ Passphrase prompts can be annoying
- ❌ More complex setup for non-technical users

**Security:** ⭐⭐⭐⭐ (Strong encryption, but file-based)
**Cost:** Free
**UX:** ⭐⭐ (Complex for average users)
**Cross-platform:** ✅
**Implementation:** ⭐⭐⭐ (GPG integration needed)

**Verdict:** Good for technical users, too complex for general audience.

---

## 💡 Option 4: Password Manager CLI Integration

**Description:** Integrate with existing password managers (1Password, Bitwarden, Pass, etc.)

**How it Works:**
```bash
# 1Password CLI
op item get "Binance API" --fields api_key

# Bitwarden CLI
bw get password "Binance API Key"

# Pass (Unix password manager)
pass show binance/api_key
```

**Pros:**
- ✅ Users already use password managers
- ✅ Cross-platform (most have CLI tools)
- ✅ No code signing needed
- ✅ Secure and audited by vendors
- ✅ Free (or user already pays)

**Cons:**
- ❌ Requires specific password manager installed
- ❌ Different CLI for each password manager
- ❌ User must manually add secrets to password manager
- ❌ Extra dependency

**Security:** ⭐⭐⭐⭐⭐ (Professional-grade)
**Cost:** Free (user already has)
**UX:** ⭐⭐⭐ (If user already uses one)
**Cross-platform:** ✅
**Implementation:** ⭐⭐ (Support multiple password managers)

**Verdict:** Great if user already uses password manager, but not everyone does.

---

## 💡 Option 5: Environment Variables with .envrc (direnv)

**Description:** Use direnv to automatically load environment variables from secure .envrc file

**How it Works:**
```bash
# .envrc (git-ignored, permissions 600)
export BINANCE_API_KEY="secret"
export BINANCE_API_SECRET="secret"

# direnv automatically loads when you cd into directory
cd /path/to/cryptofolio
# direnv: loading .envrc
```

**Pros:**
- ✅ Simple and familiar to developers
- ✅ No code signing required
- ✅ Cross-platform
- ✅ Free
- ✅ Works with existing code (env var fallback already exists)

**Cons:**
- ❌ File still on disk (encrypted filesystem helps)
- ❌ Requires direnv installed
- ❌ Must secure file permissions (600)
- ❌ Not ideal for non-developers
- ⚠️ **Still a file on disk - moderate risk**

**Security:** ⭐⭐⭐ (Better than plaintext config, but still file-based)
**Cost:** Free
**UX:** ⭐⭐⭐⭐ (For developers)
**Cross-platform:** ✅
**Implementation:** ⭐⭐⭐⭐⭐ (Already works, just document)

**Verdict:** Acceptable for personal use, not for distribution.

---

## 💡 Option 6: Hybrid Keyring with Fallback

**Description:** Try to use system keychain, fall back to other methods if unavailable

**How it Works:**
```rust
fn get_secret(key: &str) -> Result<String> {
    // Try 1: System keychain (keyring crate or security command)
    if let Ok(secret) = try_keychain(key) {
        return Ok(secret);
    }

    // Try 2: Environment variable
    if let Ok(secret) = env::var(key) {
        return Ok(secret);
    }

    // Try 3: Password manager CLI (if available)
    if let Ok(secret) = try_password_manager(key) {
        return Ok(secret);
    }

    // Try 4: Prompt user interactively
    prompt_for_secret(key)
}
```

**Pros:**
- ✅ Works for everyone (multiple fallbacks)
- ✅ Best security for those who can use keychain
- ✅ Graceful degradation
- ✅ User chooses their preferred method

**Cons:**
- ❌ More complex codebase
- ❌ Must test all paths
- ❌ Documentation becomes longer

**Security:** ⭐⭐⭐⭐ (Depends on method used)
**Cost:** Free
**UX:** ⭐⭐⭐⭐ (Something works for everyone)
**Cross-platform:** ✅
**Implementation:** ⭐⭐⭐ (More code paths to maintain)

**Verdict:** Best balance of security, usability, and compatibility.

---

## 💡 Option 7: Interactive Prompt Only (No Storage)

**Description:** Always prompt for secrets when needed, never store them

**How it Works:**
```bash
cryptofolio sync --account Binance
# Enter Binance API Key: ****
# Enter Binance API Secret: ****
# (Stored in memory for session only)
```

**Pros:**
- ✅ Maximum security (no persistent storage)
- ✅ No code signing issues
- ✅ Cross-platform
- ✅ Simple implementation
- ✅ Free

**Cons:**
- ❌ Annoying to type secrets repeatedly
- ❌ Poor UX for automation/scripts
- ❌ Secrets still in memory (could be swapped to disk)

**Security:** ⭐⭐⭐⭐⭐ (Nothing persisted)
**Cost:** Free
**UX:** ⭐ (Tedious for repeated use)
**Cross-platform:** ✅
**Implementation:** ⭐⭐⭐⭐ (Simple, remove storage code)

**Verdict:** Too inconvenient for regular use, but good for paranoid users.

---

## 📊 Comparison Matrix

| Option | Security | Cost | UX | Cross-Platform | Implementation | Overall |
|--------|----------|------|----|--------------|--------------| --------|
| 1. Developer ID | ⭐⭐⭐⭐⭐ | $99/yr | ⭐⭐⭐⭐⭐ | ❌ | ⭐⭐⭐⭐ | Best if distributing |
| 2. `security` CLI | ⭐⭐⭐⭐⭐ | Free | ⭐⭐⭐ | ❌ | ⭐⭐⭐ | Good macOS-only fix |
| 3. GPG Encryption | ⭐⭐⭐⭐ | Free | ⭐⭐ | ✅ | ⭐⭐⭐ | For technical users |
| 4. Password Manager | ⭐⭐⭐⭐⭐ | Free | ⭐⭐⭐ | ✅ | ⭐⭐ | If already using |
| 5. direnv + .envrc | ⭐⭐⭐ | Free | ⭐⭐⭐⭐ | ✅ | ⭐⭐⭐⭐⭐ | Developer-friendly |
| 6. Hybrid Fallback | ⭐⭐⭐⭐ | Free | ⭐⭐⭐⭐ | ✅ | ⭐⭐⭐ | **RECOMMENDED** |
| 7. Prompt Only | ⭐⭐⭐⭐⭐ | Free | ⭐ | ✅ | ⭐⭐⭐⭐ | Paranoid mode |

---

## 🎯 Recommended Approach

### **For Immediate Validation (Next 1-2 weeks):**

**Use Option 5 (direnv) with secure file permissions:**

```bash
# Create secure .envrc
touch .envrc
chmod 600 .envrc
echo 'export BINANCE_API_KEY="your-key"' >> .envrc
echo 'export BINANCE_API_SECRET="your-secret"' >> .envrc

# Install direnv (one-time)
brew install direnv
echo 'eval "$(direnv hook zsh)"' >> ~/.zshrc

# Allow this directory
direnv allow .

# Now commands automatically have credentials
./target/debug/cryptofolio sync --account Binance
```

**Why:**
- ✅ Works immediately without code changes
- ✅ Reasonably secure for personal development use
- ✅ Already supported by existing code (env var fallback)
- ⚠️ File is on disk, but with proper permissions and FileVault encryption, acceptable risk for testing

### **For Long-Term Solution (v0.4.1+):**

**Implement Option 6 (Hybrid Keyring with Fallback):**

Priority order:
1. Try `security` command (macOS) - no signing needed
2. Try keyring crate (if we can fix signing issue)
3. Fall back to environment variables
4. Fall back to interactive prompt

This gives everyone the best security they can get on their platform.

### **If Building a Product:**

**Get Apple Developer ID (Option 1):**
- Necessary for distribution anyway
- Enables full keychain security
- Professional solution
- Worth the $99/year investment

---

## 🔧 Immediate Action Items

**For validation to proceed TODAY:**

1. ✅ Use environment variables (already works)
2. ✅ Document secure usage in validation guide
3. ✅ Add .envrc to .gitignore
4. ✅ Warn user about file permissions

**For next release (v0.4.1):**

1. Investigate Option 2 (`security` command approach)
2. Test if it works without signing
3. Implement hybrid fallback system
4. Update documentation

**For production:**

1. Decide: Is this a product or personal tool?
2. If product → Get Apple Developer ID
3. If personal → Hybrid approach is sufficient

---

## 🔒 Security Best Practices (Meanwhile)

Even with temporary env var solution:

1. ✅ Use .envrc with mode 600 (only you can read)
2. ✅ Add .envrc to .gitignore (never commit)
3. ✅ Use FileVault (whole-disk encryption)
4. ✅ Use API keys with minimal permissions (read-only if possible)
5. ✅ Set up Binance IP whitelist (restrict API key to your IP)
6. ✅ Enable Binance API key restrictions (no withdrawals)
7. ✅ Regular key rotation (change keys monthly)

---

## 📝 Testing Plan for Option 2 (`security` command)

**Before committing to code changes, let's test:**

```bash
# Test 1: Store a secret
security add-generic-password -a "cryptofolio-test" -s "test-service" -w "test-secret"

# Test 2: Retrieve it
security find-generic-password -a "cryptofolio-test" -s "test-service" -w

# Test 3: Delete it
security delete-generic-password -a "cryptofolio-test" -s "test-service"

# If all three work without the app being signed, we have our solution!
```

---

**Decision needed:** Which approach should we take?

- **Option A:** Use env vars for now (validate today, fix later)
- **Option B:** Stop and implement `security` command now (delay validation 1-2 hours)
- **Option C:** Get Apple Developer ID (delay validation 1-2 days)

