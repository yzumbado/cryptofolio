# Touch ID Integration - Implementation Plan (v0.3.1)

**Status:** Planning
**Target Release:** v0.3.1
**Priority:** Medium (Enhancement)
**Effort Estimate:** 2-3 days

---

## Current State (v0.3.0)

### What Works ✅
- ✅ Secrets stored in macOS Keychain (OS-encrypted)
- ✅ Security levels tracked in database (standard, touchid, touchid-only)
- ✅ Security level upgrade/downgrade commands
- ✅ Session caching (15-minute timeout)
- ✅ SSH detection and graceful fallback
- ✅ All metadata and database tracking

### What's Missing ⚠️
- ⚠️ Native macOS Touch ID prompts
- ⚠️ Biometric authentication enforcement
- ⚠️ SecAccessControl integration
- ⚠️ Touch ID vs password differentiation

### Current Limitation

**Issue:** `security-framework` crate (v2.9) doesn't expose `SecAccessControl` API

The Rust `security-framework` crate provides high-level bindings to macOS Security Framework, but doesn't expose the `SecAccessControl` APIs needed for:
- Creating keychain items with biometric requirements
- Triggering Touch ID authentication prompts
- Setting access control flags (biometry required, etc.)

**Current Behavior:**
```rust
// Current implementation (v0.3.0)
use security_framework::passwords::set_generic_password;

// This stores in keychain but doesn't enforce Touch ID
set_generic_password(SERVICE_NAME, key, value.as_bytes())?;
```

**Desired Behavior:**
```rust
// Desired implementation (v0.3.1)
// Native macOS Touch ID prompt appears when accessing secret
// User must authenticate with fingerprint or password
```

---

## Solution Options

### Option 1: FFI (Foreign Function Interface) ✅ Recommended

**Use Rust FFI to call macOS Security Framework directly**

**Pros:**
- ✅ Complete control over SecAccessControl
- ✅ Native Touch ID prompts
- ✅ Full security features
- ✅ No external dependencies
- ✅ Direct access to all macOS Security APIs

**Cons:**
- ⚠️ Requires unsafe Rust code
- ⚠️ More complex than high-level bindings
- ⚠️ Need to handle memory management manually
- ⚠️ Platform-specific (macOS only)

**Complexity:** Medium-High
**Reliability:** High (once implemented)
**Recommendation:** **YES** - Best long-term solution

### Option 2: Fork security-framework Crate

**Fork and extend the security-framework crate**

**Pros:**
- ✅ Stay within Rust ecosystem
- ✅ Could contribute back upstream
- ✅ Type-safe bindings

**Cons:**
- ⚠️ Maintenance burden (keep fork updated)
- ⚠️ Larger scope (maintain entire crate)
- ⚠️ Still requires FFI internally
- ⚠️ Delayed updates from upstream

**Complexity:** High
**Reliability:** Medium
**Recommendation:** **NO** - Too much maintenance overhead

### Option 3: Use macOS `security` Command

**Shell out to `security` command-line tool**

**Pros:**
- ✅ Simple implementation
- ✅ No FFI needed
- ✅ Uses official Apple tools

**Cons:**
- ⚠️ Performance overhead (process spawning)
- ⚠️ No session caching possible
- ⚠️ Parsing command output fragile
- ⚠️ No programmatic prompt control
- ⚠️ User experience degraded

**Complexity:** Low
**Reliability:** Low
**Recommendation:** **NO** - Poor user experience

### Option 4: Wait for security-framework Updates

**Wait for upstream crate to add SecAccessControl**

**Pros:**
- ✅ No work needed
- ✅ Type-safe when available
- ✅ Maintained by community

**Cons:**
- ⚠️ Unknown timeline (could be months/years)
- ⚠️ No guarantee it will happen
- ⚠️ Leaves feature incomplete indefinitely

**Complexity:** None
**Reliability:** Unknown
**Recommendation:** **NO** - Indefinite wait not acceptable

---

## Recommended Approach: FFI Implementation

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  src/config/keychain_macos.rs                               │
│  (Current Rust Implementation)                              │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ Calls
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  src/config/keychain_ffi.rs (NEW)                           │
│  (FFI Bindings to macOS Security Framework)                 │
│                                                              │
│  - SecAccessControlCreateWithFlags()                        │
│  - SecItemAdd() with access control                         │
│  - SecItemCopyMatching() with prompts                       │
│  - SecItemUpdate() with access control                      │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ FFI Calls (unsafe)
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  macOS Security.framework                                   │
│  (Native macOS APIs)                                        │
│                                                              │
│  - Touch ID Prompts                                         │
│  - Biometric Authentication                                 │
│  - Keychain Access Control                                  │
└─────────────────────────────────────────────────────────────┘
```

### Implementation Plan

#### Phase 1: FFI Bindings (1 day)

**File:** `src/config/keychain_ffi.rs`

**Tasks:**
1. ✅ Define C types and constants
2. ✅ Declare extern "C" functions
3. ✅ Wrap FFI calls in safe Rust functions
4. ✅ Handle memory management (CFRelease, etc.)
5. ✅ Error handling for CF types

**Key APIs to Bind:**

```rust
// Core FFI declarations needed
#[link(name = "Security", kind = "framework")]
extern "C" {
    // Create access control with flags
    fn SecAccessControlCreateWithFlags(
        allocator: CFAllocatorRef,
        protection: CFTypeRef,
        flags: SecAccessControlCreateFlags,
        error: *mut CFErrorRef,
    ) -> SecAccessControlRef;

    // Add item to keychain with access control
    fn SecItemAdd(
        attributes: CFDictionaryRef,
        result: *mut CFTypeRef,
    ) -> OSStatus;

    // Copy matching items (triggers Touch ID)
    fn SecItemCopyMatching(
        query: CFDictionaryRef,
        result: *mut CFTypeRef,
    ) -> OSStatus;

    // Update item with new access control
    fn SecItemUpdate(
        query: CFDictionaryRef,
        attributesToUpdate: CFDictionaryRef,
    ) -> OSStatus;
}

// Access control flags
type SecAccessControlCreateFlags = u64;

const kSecAccessControlUserPresence: SecAccessControlCreateFlags = 1 << 0;
const kSecAccessControlBiometryAny: SecAccessControlCreateFlags = 1 << 1;
const kSecAccessControlBiometryCurrentSet: SecAccessControlCreateFlags = 1 << 3;

// Protection class
const kSecAttrAccessibleWhenUnlocked: CFStringRef = ...;
const kSecAttrAccessibleWhenUnlockedThisDeviceOnly: CFStringRef = ...;
```

#### Phase 2: Integration (0.5 day)

**File:** `src/config/keychain_macos.rs`

**Tasks:**
1. ✅ Refactor `store_with_security()` to use FFI
2. ✅ Update `retrieve()` to trigger Touch ID
3. ✅ Handle Touch ID authentication failures
4. ✅ Implement prompt reason strings
5. ✅ Session caching integration

**Example Implementation:**

```rust
impl KeychainStorage for MacOSKeychain {
    fn store_with_security(
        &self,
        key: &str,
        secret: &str,
        level: KeychainSecurityLevel,
    ) -> Result<()> {
        // Determine access control flags based on security level
        let flags = match level {
            KeychainSecurityLevel::Standard => None,
            KeychainSecurityLevel::TouchIdProtected => {
                Some(kSecAccessControlUserPresence)
            }
            KeychainSecurityLevel::TouchIdOnly => {
                Some(kSecAccessControlBiometryCurrentSet)
            }
        };

        if let Some(flags) = flags {
            // Use FFI to create item with access control
            keychain_ffi::add_with_access_control(
                SERVICE_NAME,
                key,
                secret,
                flags,
                "Cryptofolio needs access to your API keys"
            )?;
        } else {
            // Standard keychain (existing implementation)
            set_generic_password(SERVICE_NAME, key, secret.as_bytes())?;
        }

        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<String> {
        // Check session cache first
        if let Some(cached) = self.get_cached(key) {
            return Ok(cached);
        }

        // Retrieve from keychain (triggers Touch ID if configured)
        let value = keychain_ffi::get_with_prompt(
            SERVICE_NAME,
            key,
            "Cryptofolio needs to access your API keys"
        )?;

        // Cache the value
        self.cache_value(key, &value);

        Ok(value)
    }
}
```

#### Phase 3: Testing (0.5 day)

**Tasks:**
1. ✅ Unit tests for FFI functions
2. ✅ Integration tests with real keychain
3. ✅ Touch ID prompt testing (manual)
4. ✅ Error handling tests (Touch ID cancelled, failed, etc.)
5. ✅ Session cache tests

**Test Cases:**

```rust
#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn test_store_with_touch_id() {
        // Store secret with Touch ID protection
        // Manually verify Touch ID prompt appears
    }

    #[test]
    fn test_retrieve_triggers_touch_id() {
        // Retrieve Touch ID protected secret
        // Verify prompt appears
    }

    #[test]
    fn test_touch_id_cancellation() {
        // User cancels Touch ID prompt
        // Should return error, not crash
    }

    #[test]
    fn test_touch_id_failure() {
        // Touch ID authentication fails
        // Should offer password fallback
    }

    #[test]
    fn test_session_cache_avoids_prompt() {
        // Second access within 15 minutes
        // Should not show Touch ID prompt
    }
}
```

#### Phase 4: Documentation (0.5 day)

**Tasks:**
1. ✅ Update README.md with Touch ID examples
2. ✅ Update SECURITY.md with biometric details
3. ✅ Add FFI documentation
4. ✅ Update validation tests
5. ✅ Update CHANGELOG.md for v0.3.1

---

## Technical Details

### SecAccessControl Flags

| Flag | Behavior | Use Case |
|------|----------|----------|
| `kSecAccessControlUserPresence` | Touch ID **OR** password | Recommended (Touch ID Protected) |
| `kSecAccessControlBiometryAny` | Any enrolled fingerprint | Shared devices |
| `kSecAccessControlBiometryCurrentSet` | Current fingerprints only | Maximum security (Touch ID Only) |

### Prompt Context

**macOS Touch ID Prompt:**
```
┌──────────────────────────────────────────┐
│  🔐 Touch ID Required                    │
│                                          │
│  Cryptofolio needs access to your        │
│  API keys                                │
│                                          │
│  👆 Touch the sensor to continue...      │
│                                          │
│  [Use Password Instead]                  │
└──────────────────────────────────────────┘
```

**Prompt Customization:**
```rust
let prompt = match key {
    "binance.api_secret" => "Access Binance API credentials",
    "coinbase.api_key" => "Access Coinbase API credentials",
    _ => "Access your API keys",
};
```

### Error Handling

**Possible Errors:**
- `-128` (errSecUserCanceled) - User cancelled Touch ID
- `-25293` (errSecAuthFailed) - Authentication failed
- `-25300` (errSecItemNotFound) - Secret not found
- `-34018` (errSecMissingEntitlement) - App not signed correctly

**Rust Error Mapping:**
```rust
fn map_osstatus_error(status: OSStatus) -> CryptofolioError {
    match status {
        -128 => CryptofolioError::KeychainAuthCancelled("User cancelled Touch ID".into()),
        -25293 => CryptofolioError::TouchIdAuthFailed("Touch ID authentication failed".into()),
        -25300 => CryptofolioError::Keychain("Secret not found".into()),
        other => CryptofolioError::Keychain(format!("Keychain error: {}", other)),
    }
}
```

### Memory Management

**Core Foundation Ownership:**
```rust
unsafe fn create_access_control(flags: SecAccessControlCreateFlags) -> Result<SecAccessControlRef> {
    let mut error: CFErrorRef = std::ptr::null_mut();

    let access_control = SecAccessControlCreateWithFlags(
        kCFAllocatorDefault,
        kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
        flags,
        &mut error,
    );

    if access_control.is_null() {
        if !error.is_null() {
            CFRelease(error as CFTypeRef); // Must release error!
        }
        return Err(CryptofolioError::Keychain("Failed to create access control".into()));
    }

    Ok(access_control)
    // Caller must CFRelease(access_control) when done!
}
```

---

## Testing Strategy

### Manual Testing

**Scenario 1: First Touch ID Prompt**
1. Migrate secret to keychain with Touch ID level
2. Run command that needs secret
3. **Expected:** Touch ID prompt appears
4. Touch sensor
5. **Expected:** Command succeeds

**Scenario 2: Session Cache**
1. Access Touch ID protected secret
2. Immediately access same secret again
3. **Expected:** No second Touch ID prompt (cached)

**Scenario 3: Touch ID Cancellation**
1. Access Touch ID protected secret
2. Click "Cancel" on prompt
3. **Expected:** Error message, command fails gracefully

**Scenario 4: Touch ID Failure**
1. Access Touch ID protected secret
2. Use wrong finger 3 times
3. **Expected:** Falls back to password prompt

**Scenario 5: SSH Fallback**
1. SSH into Mac
2. Try to access Touch ID protected secret
3. **Expected:** Clear error (Touch ID not available in SSH)

### Automated Testing

**Unit Tests:**
- FFI function correctness
- Error mapping
- Memory leak detection (valgrind)

**Integration Tests:**
- Real keychain operations
- Access control verification
- Cache behavior

**Validation Tests:**
- Update existing validation suite
- Add Touch ID specific test cases

---

## Dependencies

### Crates

No new crate dependencies needed! Pure FFI using:
- `std::os::raw` - C types (c_void, c_char, etc.)
- Existing `security-framework` for high-level operations

### Build Requirements

**Xcode Command Line Tools:**
```bash
xcode-select --install
```

**Cargo.toml:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
security-framework = "2.9"
# No new dependencies needed for FFI!
```

### Runtime Requirements

- macOS 10.12+ (Sierra or later)
- Touch ID capable hardware (or password fallback)

---

## Migration Path (v0.3.0 → v0.3.1)

### User Experience

**v0.3.0 (Current):**
```bash
$ cryptofolio sync --account Binance
Syncing with Binance...
# Secret retrieved from keychain silently
[OK] Synced 15 holdings
```

**v0.3.1 (After FFI):**
```bash
$ cryptofolio sync --account Binance
# Touch ID prompt appears (if secret is Touch ID Protected)
🔐 Touch the sensor to continue...
# User authenticates
Syncing with Binance...
[OK] Synced 15 holdings
```

### Code Changes

**No breaking changes!**
- Existing secrets continue working
- No re-migration needed
- Security levels automatically work
- Session cache prevents excessive prompts

**Automatic Enhancement:**
- Secrets with `touchid` or `touchid-only` level automatically start using Touch ID
- No user action required

---

## Risks & Mitigation

### Risk 1: FFI Memory Leaks

**Risk:** Incorrect CFRelease usage causes memory leaks
**Severity:** Medium
**Mitigation:**
- Comprehensive testing with Instruments (macOS memory profiler)
- RAII wrappers for CF types
- Code review focused on memory management

### Risk 2: Touch ID Prompt Confusion

**Risk:** Users don't understand why prompt appears
**Severity:** Low
**Mitigation:**
- Clear prompt messages
- Documentation updates
- Help text with examples

### Risk 3: Platform-Specific Bugs

**Risk:** FFI works differently on different macOS versions
**Severity:** Medium
**Mitigation:**
- Test on multiple macOS versions (10.12, 10.15, 11.0, 12.0+)
- Graceful degradation for older systems
- Clear error messages for unsupported features

### Risk 4: Compilation Complexity

**Risk:** FFI makes compilation harder for contributors
**Severity:** Low
**Mitigation:**
- Well-documented build process
- FFI isolated in separate module
- Non-macOS builds unaffected

---

## Success Criteria

### Functional
- ✅ Native Touch ID prompts appear for `touchid` and `touchid-only` levels
- ✅ Authentication required before secret access
- ✅ Password fallback works when Touch ID fails
- ✅ Session caching prevents excessive prompts
- ✅ SSH detection shows clear error message

### Technical
- ✅ No memory leaks (verified with Instruments)
- ✅ Zero crashes during Touch ID operations
- ✅ Error handling covers all OSStatus codes
- ✅ Builds cleanly on macOS
- ✅ Non-macOS builds unaffected

### User Experience
- ✅ Touch ID prompt appears < 1 second after command
- ✅ Prompt messages clear and helpful
- ✅ Session cache reduces prompts to ~1 per 15 minutes
- ✅ Graceful degradation when Touch ID unavailable
- ✅ Documentation clear and complete

---

## Timeline

**Total Estimate:** 2-3 days

| Phase | Duration | Tasks |
|-------|----------|-------|
| **FFI Bindings** | 1 day | Declare extern functions, wrap in safe Rust |
| **Integration** | 0.5 day | Refactor keychain_macos.rs to use FFI |
| **Testing** | 0.5 day | Unit, integration, manual validation |
| **Documentation** | 0.5 day | Update all docs, examples, guides |
| **Buffer** | 0.5 day | Unexpected issues, review, polish |

**Target Release:** v0.3.1 (after Phase 3 completion)

---

## References

### Apple Documentation
- [Keychain Services - Security Framework](https://developer.apple.com/documentation/security/keychain_services)
- [SecAccessControl](https://developer.apple.com/documentation/security/secaccesscontrol)
- [SecAccessControlCreateFlags](https://developer.apple.com/documentation/security/secaccesscontrolcreateflags)
- [Local Authentication Framework](https://developer.apple.com/documentation/localauthentication)

### Code Examples
- [Storing Keys in the Keychain](https://developer.apple.com/documentation/security/certificate_key_and_trust_services/keys/storing_keys_in_the_keychain)
- [Restricting Keychain Item Accessibility](https://developer.apple.com/documentation/security/keychain_services/keychain_items/restricting_keychain_item_accessibility)

### Rust FFI
- [The Rustonomicon - FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [Rust FFI Omnibus](http://jakegoulding.com/rust-ffi-omnibus/)

---

## Decision: Proceed with FFI Implementation?

**Recommendation:** **YES** ✅

**Rationale:**
1. Clean, maintainable solution
2. Full control over Touch ID behavior
3. No external dependencies
4. Enables complete security feature
5. Reasonable implementation effort (2-3 days)

**Next Steps:**
1. Approve this plan
2. Create implementation tasks
3. Implement FFI bindings (Phase 1)
4. Integrate with keychain_macos.rs (Phase 2)
5. Test thoroughly (Phase 3)
6. Document and release v0.3.1 (Phase 4)

---

**Status:** Awaiting approval to proceed with FFI implementation
**Priority:** Medium (v0.3.1 enhancement, not critical for Phase 3)
**Owner:** TBD
