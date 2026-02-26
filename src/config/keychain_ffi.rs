//! FFI bindings to macOS Security Framework for Touch ID integration
//!
//! This module provides low-level Foreign Function Interface (FFI) bindings to
//! macOS Security Framework APIs that are not exposed in the security-framework crate.
//!
//! **Why FFI?**
//! - security-framework 2.9 lacks SecAccessControl API
//! - Needed for native Touch ID prompts
//! - Direct C API access gives full control
//!
//! **Safety:**
//! - All C functions are wrapped in safe Rust abstractions
//! - Memory management handled via Core Foundation helpers
//! - Errors converted to Result<T, Error>

#![cfg(target_os = "macos")]

use core_foundation::base::{CFAllocatorRef, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::CFMutableDictionary;
use core_foundation::error::{CFError, CFErrorRef};
use core_foundation::string::{CFString, CFStringRef};
use std::ffi::c_void;
use std::ptr;

use crate::error::{CryptofolioError, Result};

// ================================================================================================
// C Types & Constants
// ================================================================================================

/// Opaque type for SecAccessControl
pub type SecAccessControlRef = CFTypeRef;

/// Opaque type for keychain item attributes dictionary
pub type SecItemRef = CFTypeRef;

/// OSStatus - macOS security status codes
pub type OSStatus = i32;

// OSStatus codes
pub const ERR_SEC_SUCCESS: OSStatus = 0;
pub const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;
pub const ERR_SEC_DUPLICATE_ITEM: OSStatus = -25299;
pub const ERR_SEC_AUTH_FAILED: OSStatus = -25293;
pub const ERR_SEC_USER_CANCELED: OSStatus = -128;

// SecAccessControlCreateFlags
pub const K_SEC_ACCESS_CONTROL_USER_PRESENCE: u64 = 1 << 0; // Touch ID OR password
pub const K_SEC_ACCESS_CONTROL_BIOMETRY_ANY: u64 = 1 << 1; // Touch ID ONLY (no password)
pub const K_SEC_ACCESS_CONTROL_BIOMETRY_CURRENT_SET: u64 = 1 << 3;
pub const K_SEC_ACCESS_CONTROL_DEVICE_PASSCODE: u64 = 1 << 4;
pub const K_SEC_ACCESS_CONTROL_OR: u64 = 1 << 14;
pub const K_SEC_ACCESS_CONTROL_AND: u64 = 1 << 15;

// Keychain item attribute keys (CFString constants)
// These match the constants from Security.framework
const K_SEC_CLASS: &str = "class";
const K_SEC_CLASS_GENERIC_PASSWORD: &str = "genp";
const K_SEC_ATTR_SERVICE: &str = "svce";
const K_SEC_ATTR_ACCOUNT: &str = "acct";
const K_SEC_VALUE_DATA: &str = "v_Data";
const K_SEC_RETURN_DATA: &str = "r_Data";
const K_SEC_MATCH_LIMIT: &str = "m_Limit";
const K_SEC_MATCH_LIMIT_ONE: &str = "mLmt1";
const K_SEC_ATTR_ACCESS_CONTROL: &str = "accc";
const K_SEC_USE_OPERATION_PROMPT: &str = "u_OpPrompt";
const K_SEC_ATTR_ACCESSIBLE: &str = "pdmn";
const K_SEC_ATTR_ACCESSIBLE_WHEN_UNLOCKED: &str = "ak";

// ================================================================================================
// Extern C Declarations
// ================================================================================================

#[link(name = "Security", kind = "framework")]
extern "C" {
    /// Create access control constraint for keychain items
    ///
    /// # Arguments
    /// * `allocator` - CFAllocator (usually kCFAllocatorDefault)
    /// * `protection` - Protection level (usually kSecAttrAccessibleWhenUnlocked)
    /// * `flags` - Access control flags (Touch ID requirements)
    /// * `error` - Output parameter for errors
    ///
    /// # Returns
    /// SecAccessControlRef on success, null on failure
    fn SecAccessControlCreateWithFlags(
        allocator: CFAllocatorRef,
        protection: CFTypeRef,
        flags: u64,
        error: *mut CFErrorRef,
    ) -> SecAccessControlRef;

    /// Add item to keychain
    ///
    /// # Arguments
    /// * `attributes` - Dictionary of keychain item attributes
    /// * `result` - Output parameter for created item (can be null)
    ///
    /// # Returns
    /// OSStatus (0 = success, negative = error code)
    fn SecItemAdd(attributes: CFTypeRef, result: *mut CFTypeRef) -> OSStatus;

    /// Search for keychain item
    ///
    /// # Arguments
    /// * `query` - Dictionary of search criteria
    /// * `result` - Output parameter for found item data
    ///
    /// # Returns
    /// OSStatus (0 = success, -25300 = not found)
    fn SecItemCopyMatching(query: CFTypeRef, result: *mut CFTypeRef) -> OSStatus;

    /// Delete keychain item
    ///
    /// # Arguments
    /// * `query` - Dictionary of search criteria
    ///
    /// # Returns
    /// OSStatus (0 = success)
    fn SecItemDelete(query: CFTypeRef) -> OSStatus;

    /// Update existing keychain item
    ///
    /// # Arguments
    /// * `query` - Dictionary of search criteria
    /// * `attributes_to_update` - Dictionary of attributes to change
    ///
    /// # Returns
    /// OSStatus (0 = success)
    fn SecItemUpdate(query: CFTypeRef, attributes_to_update: CFTypeRef) -> OSStatus;

    // Note: We load kSecAttrAccessibleWhenUnlocked dynamically via dlsym
    // instead of using extern static to avoid link-time issues
    fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
    fn dlopen(filename: *const i8, flag: i32) -> *mut c_void;
}

// RTLD_DEFAULT - search all loaded libraries
const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

/// Get kSecAttrAccessibleWhenUnlocked at runtime using dlsym
/// This avoids static linking issues with extern static
fn get_accessible_when_unlocked() -> CFStringRef {
    unsafe {
        let symbol_name = b"kSecAttrAccessibleWhenUnlocked\0".as_ptr() as *const i8;
        let sym_ptr = dlsym(RTLD_DEFAULT, symbol_name);
        if sym_ptr.is_null() {
            panic!("Failed to load kSecAttrAccessibleWhenUnlocked symbol");
        }
        // The symbol is a pointer to CFStringRef, so dereference it
        *(sym_ptr as *const CFStringRef)
    }
}

// ================================================================================================
// Safe Rust Wrappers
// ================================================================================================

/// Create SecAccessControl with Touch ID requirements
///
/// # Arguments
/// * `flags` - Access control flags (K_SEC_ACCESS_CONTROL_*)
///
/// # Returns
/// Result<SecAccessControlRef> - Access control reference or error
///
/// # Safety
/// This function is safe because:
/// - CFError is properly managed
/// - Result is checked before returning
/// - Caller owns returned CFTypeRef (must release)
pub fn create_access_control(flags: u64) -> Result<SecAccessControlRef> {
    unsafe {
        let mut error: CFErrorRef = ptr::null_mut();

        let accessible_when_unlocked = get_accessible_when_unlocked();
        let access_control = SecAccessControlCreateWithFlags(
            ptr::null_mut(), // kCFAllocatorDefault
            accessible_when_unlocked as CFTypeRef,
            flags,
            &mut error,
        );

        if access_control.is_null() {
            if !error.is_null() {
                let cf_error = CFError::wrap_under_create_rule(error);
                return Err(CryptofolioError::Config(format!(
                    "Failed to create access control: {}",
                    cf_error.description()
                )));
            }
            return Err(CryptofolioError::Config(
                "Failed to create access control: unknown error".to_string(),
            ));
        }

        Ok(access_control)
    }
}

/// Store password in keychain with Touch ID protection
///
/// # Arguments
/// * `service` - Service name (e.g., "com.cryptofolio.api-keys")
/// * `account` - Account name (e.g., "binance.api_secret")
/// * `password` - Secret to store
/// * `access_control` - Optional access control (Touch ID settings)
/// * `prompt` - Optional Touch ID prompt message
///
/// # Returns
/// Result<()> - Success or error
pub fn keychain_add_password(
    service: &str,
    account: &str,
    password: &str,
    access_control: Option<SecAccessControlRef>,
    prompt: Option<&str>,
) -> Result<()> {
    unsafe {
        let service_cf = CFString::new(service);
        let account_cf = CFString::new(account);
        let password_data = CFData::from_buffer(password.as_bytes());

        // Create mutable dictionary for keychain attributes
        let mut attributes_dict = CFMutableDictionary::new();

        // Add base attributes
        attributes_dict.set(
            CFString::from_static_string(K_SEC_CLASS).as_CFTypeRef(),
            CFString::from_static_string(K_SEC_CLASS_GENERIC_PASSWORD).as_CFTypeRef(),
        );
        attributes_dict.set(
            CFString::from_static_string(K_SEC_ATTR_SERVICE).as_CFTypeRef(),
            service_cf.as_CFTypeRef(),
        );
        attributes_dict.set(
            CFString::from_static_string(K_SEC_ATTR_ACCOUNT).as_CFTypeRef(),
            account_cf.as_CFTypeRef(),
        );
        attributes_dict.set(
            CFString::from_static_string(K_SEC_VALUE_DATA).as_CFTypeRef(),
            password_data.as_CFTypeRef(),
        );

        // Add access control if provided
        if let Some(ac) = access_control {
            attributes_dict.set(
                CFString::from_static_string(K_SEC_ATTR_ACCESS_CONTROL).as_CFTypeRef(),
                ac,
            );

            // Add Touch ID prompt message if provided
            if let Some(msg) = prompt {
                let prompt_cf = CFString::new(msg);
                attributes_dict.set(
                    CFString::from_static_string(K_SEC_USE_OPERATION_PROMPT).as_CFTypeRef(),
                    prompt_cf.as_CFTypeRef(),
                );
            }
        } else {
            // Standard keychain access (no Touch ID)
            attributes_dict.set(
                CFString::from_static_string(K_SEC_ATTR_ACCESSIBLE).as_CFTypeRef(),
                CFString::from_static_string(K_SEC_ATTR_ACCESSIBLE_WHEN_UNLOCKED)
                    .as_CFTypeRef(),
            );
        }

        let status = SecItemAdd(attributes_dict.as_CFTypeRef(), ptr::null_mut());

        match status {
            ERR_SEC_SUCCESS => Ok(()),
            ERR_SEC_DUPLICATE_ITEM => {
                // Item exists, try to update instead
                keychain_update_password(service, account, password, access_control, prompt)
            }
            ERR_SEC_USER_CANCELED => Err(CryptofolioError::Config(
                "Touch ID authentication canceled by user".to_string(),
            )),
            ERR_SEC_AUTH_FAILED => {
                Err(CryptofolioError::Config("Touch ID authentication failed".to_string()))
            }
            _ => Err(CryptofolioError::Config(format!(
                "Failed to add keychain item: OSStatus {}",
                status
            ))),
        }
    }
}

/// Retrieve password from keychain (may trigger Touch ID prompt)
///
/// # Arguments
/// * `service` - Service name
/// * `account` - Account name
/// * `prompt` - Optional Touch ID prompt message
///
/// # Returns
/// Result<String> - Password or error
pub fn keychain_get_password(service: &str, account: &str, prompt: Option<&str>) -> Result<String> {
    unsafe {
        let service_cf = CFString::new(service);
        let account_cf = CFString::new(account);

        // Create mutable dictionary for query
        let mut query_dict = CFMutableDictionary::new();

        query_dict.set(
            CFString::from_static_string(K_SEC_CLASS).as_CFTypeRef(),
            CFString::from_static_string(K_SEC_CLASS_GENERIC_PASSWORD).as_CFTypeRef(),
        );
        query_dict.set(
            CFString::from_static_string(K_SEC_ATTR_SERVICE).as_CFTypeRef(),
            service_cf.as_CFTypeRef(),
        );
        query_dict.set(
            CFString::from_static_string(K_SEC_ATTR_ACCOUNT).as_CFTypeRef(),
            account_cf.as_CFTypeRef(),
        );
        query_dict.set(
            CFString::from_static_string(K_SEC_RETURN_DATA).as_CFTypeRef(),
            CFBoolean::true_value().as_CFTypeRef(),
        );
        query_dict.set(
            CFString::from_static_string(K_SEC_MATCH_LIMIT).as_CFTypeRef(),
            CFString::from_static_string(K_SEC_MATCH_LIMIT_ONE).as_CFTypeRef(),
        );

        // Add Touch ID prompt if provided
        if let Some(msg) = prompt {
            let prompt_cf = CFString::new(msg);
            query_dict.set(
                CFString::from_static_string(K_SEC_USE_OPERATION_PROMPT).as_CFTypeRef(),
                prompt_cf.as_CFTypeRef(),
            );
        }

        let mut result: CFTypeRef = ptr::null();

        let status = SecItemCopyMatching(query_dict.as_CFTypeRef(), &mut result);

        match status {
            ERR_SEC_SUCCESS => {
                if result.is_null() {
                    return Err(CryptofolioError::Config("Keychain returned null data".to_string()));
                }

                let data = CFData::wrap_under_create_rule(result as *const _);
                let bytes = data.bytes();
                let password = String::from_utf8(bytes.to_vec()).map_err(|_| {
                    CryptofolioError::Config("Keychain data is not valid UTF-8".to_string())
                })?;

                Ok(password)
            }
            ERR_SEC_ITEM_NOT_FOUND => {
                Err(CryptofolioError::Config("Keychain item not found".to_string()))
            }
            ERR_SEC_USER_CANCELED => Err(CryptofolioError::Config(
                "Touch ID authentication canceled by user".to_string(),
            )),
            ERR_SEC_AUTH_FAILED => {
                Err(CryptofolioError::Config("Touch ID authentication failed".to_string()))
            }
            _ => Err(CryptofolioError::Config(format!(
                "Failed to retrieve keychain item: OSStatus {}",
                status
            ))),
        }
    }
}

/// Delete password from keychain
///
/// # Arguments
/// * `service` - Service name
/// * `account` - Account name
///
/// # Returns
/// Result<()> - Success or error
pub fn keychain_delete_password(service: &str, account: &str) -> Result<()> {
    unsafe {
        let service_cf = CFString::new(service);
        let account_cf = CFString::new(account);

        // Create mutable dictionary for query
        let mut query_dict = CFMutableDictionary::new();

        query_dict.set(
            CFString::from_static_string(K_SEC_CLASS).as_CFTypeRef(),
            CFString::from_static_string(K_SEC_CLASS_GENERIC_PASSWORD).as_CFTypeRef(),
        );
        query_dict.set(
            CFString::from_static_string(K_SEC_ATTR_SERVICE).as_CFTypeRef(),
            service_cf.as_CFTypeRef(),
        );
        query_dict.set(
            CFString::from_static_string(K_SEC_ATTR_ACCOUNT).as_CFTypeRef(),
            account_cf.as_CFTypeRef(),
        );

        let status = SecItemDelete(query_dict.as_CFTypeRef());

        match status {
            ERR_SEC_SUCCESS => Ok(()),
            ERR_SEC_ITEM_NOT_FOUND => Ok(()), // Already deleted, not an error
            _ => Err(CryptofolioError::Config(format!(
                "Failed to delete keychain item: OSStatus {}",
                status
            ))),
        }
    }
}

/// Update existing password in keychain
///
/// # Arguments
/// * `service` - Service name
/// * `account` - Account name
/// * `password` - New secret value
/// * `access_control` - Optional new access control
/// * `prompt` - Optional Touch ID prompt message
///
/// # Returns
/// Result<()> - Success or error
fn keychain_update_password(
    service: &str,
    account: &str,
    password: &str,
    access_control: Option<SecAccessControlRef>,
    prompt: Option<&str>,
) -> Result<()> {
    unsafe {
        let service_cf = CFString::new(service);
        let account_cf = CFString::new(account);
        let password_data = CFData::from_buffer(password.as_bytes());

        // Create mutable dictionary for query
        let mut query_dict = CFMutableDictionary::new();

        query_dict.set(
            CFString::from_static_string(K_SEC_CLASS).as_CFTypeRef(),
            CFString::from_static_string(K_SEC_CLASS_GENERIC_PASSWORD).as_CFTypeRef(),
        );
        query_dict.set(
            CFString::from_static_string(K_SEC_ATTR_SERVICE).as_CFTypeRef(),
            service_cf.as_CFTypeRef(),
        );
        query_dict.set(
            CFString::from_static_string(K_SEC_ATTR_ACCOUNT).as_CFTypeRef(),
            account_cf.as_CFTypeRef(),
        );

        // Create mutable dictionary for updates
        let mut updates_dict = CFMutableDictionary::new();

        updates_dict.set(
            CFString::from_static_string(K_SEC_VALUE_DATA).as_CFTypeRef(),
            password_data.as_CFTypeRef(),
        );

        // Add access control if provided
        if let Some(ac) = access_control {
            updates_dict.set(
                CFString::from_static_string(K_SEC_ATTR_ACCESS_CONTROL).as_CFTypeRef(),
                ac,
            );

            if let Some(msg) = prompt {
                let prompt_cf = CFString::new(msg);
                updates_dict.set(
                    CFString::from_static_string(K_SEC_USE_OPERATION_PROMPT).as_CFTypeRef(),
                    prompt_cf.as_CFTypeRef(),
                );
            }
        }

        let status = SecItemUpdate(query_dict.as_CFTypeRef(), updates_dict.as_CFTypeRef());

        match status {
            ERR_SEC_SUCCESS => Ok(()),
            ERR_SEC_USER_CANCELED => Err(CryptofolioError::Config(
                "Touch ID authentication canceled by user".to_string(),
            )),
            ERR_SEC_AUTH_FAILED => {
                Err(CryptofolioError::Config("Touch ID authentication failed".to_string()))
            }
            _ => Err(CryptofolioError::Config(format!(
                "Failed to update keychain item: OSStatus {}",
                status
            ))),
        }
    }
}

// ================================================================================================
// Tests
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_access_control_user_presence() {
        let ac = create_access_control(K_SEC_ACCESS_CONTROL_USER_PRESENCE);
        assert!(ac.is_ok(), "Should create access control with user presence");
    }

    #[test]
    fn test_create_access_control_biometry() {
        let ac = create_access_control(K_SEC_ACCESS_CONTROL_BIOMETRY_ANY);
        assert!(ac.is_ok(), "Should create access control with biometry");
    }

    #[test]
    fn test_keychain_add_get_delete() {
        let service = "com.cryptofolio.test";
        let account = "test.key";
        let password = "test_secret_123";

        // Clean up any existing test data
        let _ = keychain_delete_password(service, account);

        // Add
        let add_result = keychain_add_password(service, account, password, None, None);
        assert!(add_result.is_ok(), "Should add keychain item");

        // Get
        let get_result = keychain_get_password(service, account, None);
        assert!(get_result.is_ok(), "Should retrieve keychain item");
        assert_eq!(get_result.unwrap(), password, "Password should match");

        // Delete
        let delete_result = keychain_delete_password(service, account);
        assert!(delete_result.is_ok(), "Should delete keychain item");

        // Verify deleted
        let get_after_delete = keychain_get_password(service, account, None);
        assert!(
            get_after_delete.is_err(),
            "Should not find deleted keychain item"
        );
    }
}
