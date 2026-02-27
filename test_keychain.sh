#!/bin/bash
# Test script to debug keychain operations
# Run this in your regular terminal (not automated)

set -x  # Print each command

echo "=========================================="
echo "Test 1: Version check"
echo "=========================================="
./target/release/cryptofolio --version

echo ""
echo "=========================================="
echo "Test 2: Keychain status (read-only)"
echo "=========================================="
./target/release/cryptofolio config keychain-status

echo ""
echo "=========================================="
echo "Test 3: Set secret with STANDARD level"
echo "=========================================="
echo "my_test_secret_123" | ./target/release/cryptofolio config set-secret test.standard.key --security-level standard

echo ""
echo "=========================================="
echo "Test 4: Verify it was stored"
echo "=========================================="
./target/release/cryptofolio config keychain-status | grep test.standard

echo ""
echo "=========================================="
echo "Test 5: Set secret with TOUCHID level"
echo "=========================================="
echo "my_test_secret_456" | ./target/release/cryptofolio config set-secret test.touchid.key --security-level touchid

echo ""
echo "=========================================="
echo "RESULTS:"
echo "=========================================="
echo "If Test 3 fails with exit 137 (killed) = keychain write issue"
echo "If Test 3 fails with -34018 = entitlements issue (expected)"
echo "If Test 5 fails with -34018 = entitlements issue (expected with ad-hoc signing)"
echo ""
