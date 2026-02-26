#!/bin/bash
# Code signing script for Cryptofolio development
# This enables Touch ID functionality by signing the binary with keychain entitlements

set -e

echo "🔨 Building Cryptofolio..."
cargo build --release

echo ""
echo "📝 Creating keychain entitlements..."
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

echo ""
echo "✍️  Signing binary (ad-hoc)..."
codesign --entitlements /tmp/cryptofolio.entitlements \
    --force \
    --sign - \
    ./target/release/cryptofolio

echo ""
echo "🔍 Verifying signature..."
if codesign -dvvv ./target/release/cryptofolio 2>&1 | grep -q "Signature=adhoc"; then
    echo "✅ Binary signed successfully (ad-hoc)"
else
    echo "❌ Signature verification failed"
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Cryptofolio is ready with Touch ID support!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Try these commands:"
echo "  ./target/release/cryptofolio config set-secret test.touchid --security-level touchid"
echo "  ./target/release/cryptofolio config keychain-status"
echo ""
echo "Note: Ad-hoc signing only works on this Mac. For distribution, use Apple Developer ID."
echo ""
