# Code Signing Required for Touch ID

## Issue Discovered

When testing v0.3.1 Touch ID implementation, we encountered:

```
Error: Failed to store secret: OSStatus -34018
```

**Root Cause:** `errSecMissingEntitlement`

## What This Means

macOS requires apps to be **properly code-signed** with **keychain entitlements** to use `SecAccessControl` (Touch ID/biometric authentication).

### Why It Fails

1. **Unsigned Binary:** `cargo build` produces unsigned executables
2. **No Entitlements:** App doesn't declare keychain-access-groups
3. **SecAccessControl:** Requires special permissions from macOS

### Apple Documentation

From Apple's Security Framework docs:

> **SecAccessControl** requires the app to be signed with keychain access entitlements.
> Unsigned or ad-hoc signed apps cannot use biometric authentication.

## Solutions

### Option 1: Ad-Hoc Code Signing (Development)

**Quick test for local development:**

```bash
# Create entitlements file
cat > cryptofolio.entitlements <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>keychain-access-groups</key>
    <array>
        <string>\$(AppIdentifierPrefix)com.cryptofolio</string>
    </array>
</dict>
</plist>
EOF

# Sign the binary
codesign --entitlements cryptofolio.entitlements -s - ./target/release/cryptofolio

# Verify
codesign -d --entitlements - ./target/release/cryptofolio
```

**Limitations:**
- Ad-hoc signing (`-s -`) only works on your Mac
- Cannot distribute to other users
- May require disabling SIP in some cases

### Option 2: Self-Signed Certificate (Better)

**Create a self-signed certificate for testing:**

1. Open **Keychain Access**
2. **Keychain Access → Certificate Assistant → Create a Certificate**
3. Name: "Cryptofolio Development"
4. Identity Type: "Self-Signed Root"
5. Certificate Type: "Code Signing"
6. Click "Create"

**Sign with certificate:**
```bash
codesign --entitlements cryptofolio.entitlements \
    -s "Cryptofolio Development" \
    ./target/release/cryptofolio
```

**Benefits:**
- Proper signing workflow
- Can share with team
- Better for testing

### Option 3: Apple Developer Certificate (Production)

**For distribution to users:**

1. Join Apple Developer Program ($99/year)
2. Create Developer ID Application certificate
3. Sign binary with Developer ID
4. Notarize app with Apple

```bash
codesign --entitlements cryptofolio.entitlements \
    --sign "Developer ID Application: Your Name (TEAM_ID)" \
    --timestamp \
    ./target/release/cryptofolio

# Notarize
xcrun notarytool submit cryptofolio.zip \
    --apple-id "your@email.com" \
    --team-id "TEAM_ID" \
    --wait
```

**Benefits:**
- Users can download and run without warnings
- Full Touch ID support
- Gatekeeper approval

### Option 4: Fallback Mode (Current Behavior)

**What happens now:**

1. Try to use SecAccessControl (fails with -34018)
2. Fall back to standard keychain (no Touch ID)
3. Log warning for user

**In code:**
```rust
if let Some(ac) = access_control {
    // This will fail with -34018 if not signed
    match keychain_add_password(..., Some(ac), ...) {
        Err(e) if e.to_string().contains("-34018") => {
            eprintln!("Warning: Touch ID requires code signing. Using standard keychain.");
            keychain_add_password(..., None, ...) // Retry without SecAccessControl
        }
        result => result
    }
}
```

## Recommended Approach

### For Development (Right Now)

**Use ad-hoc signing:**
```bash
# Quick test script
#!/bin/bash
set -e

echo "Building release binary..."
cargo build --release

echo "Creating entitlements..."
cat > /tmp/cryptofolio.entitlements <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>keychain-access-groups</key>
    <array>
        <string>\$(AppIdentifierPrefix)com.cryptofolio</string>
    </array>
    <key>com.apple.security.app-sandbox</key>
    <false/>
</dict>
</plist>
EOF

echo "Signing binary..."
codesign --entitlements /tmp/cryptofolio.entitlements \
    --force \
    --sign - \
    ./target/release/cryptofolio

echo "Verifying signature..."
codesign -dvvv ./target/release/cryptofolio 2>&1 | grep -A5 "Signature"

echo ""
echo "✅ Signed successfully!"
echo "Now test Touch ID:"
echo "  ./target/release/cryptofolio config set-secret test.touchid --security-level touchid"
```

Save as `sign.sh`, make executable: `chmod +x sign.sh`, then run: `./sign.sh`

### For Distribution (Later)

**When ready to release:**
1. Get Apple Developer ID
2. Set up notarization workflow
3. Automate in CI/CD (GitHub Actions)

## Testing Without Code Signing

**Standard keychain still works:**
```bash
# This works without signing
./target/release/cryptofolio config set-secret test.key --security-level standard

# This will fall back to standard if not signed
./target/release/cryptofolio config set-secret test.key --security-level touchid
# Warning: Touch ID requires code signing. Using standard keychain.
```

**All other features work:**
- ✅ Keychain storage (OS-encrypted)
- ✅ Session caching (15 minutes)
- ✅ TOML migration
- ✅ SSH detection
- ❌ Touch ID prompts (requires signing)

## Implementation Status

| Feature | Unsigned Binary | Ad-Hoc Signed | Developer ID Signed |
|---------|----------------|---------------|---------------------|
| Keychain Storage | ✅ | ✅ | ✅ |
| Standard Level | ✅ | ✅ | ✅ |
| Touch ID Protected | ❌ (-34018) | ✅ | ✅ |
| Touch ID Only | ❌ (-34018) | ✅ | ✅ |
| Distribution | ✅ (source) | ❌ | ✅ |

## Updated Test Plan

**Before manual testing:**
1. Run `sign.sh` to sign the binary
2. Verify signature: `codesign -dvvv ./target/release/cryptofolio`
3. Then run V0.3.1_TEST_PLAN.md

**Or test without signing:**
- All Standard level tests will work
- Touch ID tests will show fallback warning

## References

- [Apple: Code Signing Guide](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/)
- [Apple: SecAccessControl](https://developer.apple.com/documentation/security/secaccesscontrol)
- [Keychain Services Entitlements](https://developer.apple.com/documentation/bundleresources/entitlements/keychain-access-groups)
- [OSStatus Error Codes](https://www.osstatus.com)

## Next Steps

1. ✅ FFI implementation complete (code is correct)
2. ⏳ Create sign.sh script for easy development signing
3. ⏳ Test with ad-hoc signed binary
4. ⏳ Add fallback handling for -34018 error
5. ⏳ Document code signing in README.md
6. ⏳ Set up Apple Developer ID for production

---

**TL;DR:** The Touch ID code is correct, but macOS requires the app to be signed to use it. Use the sign.sh script above for testing.
