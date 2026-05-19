#!/usr/bin/env bash
# Create and trust a self-signed code-signing certificate named "stint-dev"
# in the user's login keychain. Idempotent — checks first, skips if already
# present. Run once per machine.
#
# This cert lets scripts/dev-cli.sh produce a stable code signature on every
# dev build, so macOS Keychain ACL ("Always Allow") persists across rebuilds
# and you stop getting prompted on every run.
set -euo pipefail

CERT_NAME="stint-dev"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

if security find-identity -v -p codesigning | grep -q "\"$CERT_NAME\""; then
  echo "$CERT_NAME identity already exists. Nothing to do."
  exit 0
fi

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Creating self-signed cert and key..."
# macOS Security framework requires BOTH Key Usage (digitalSignature) and
# Extended Key Usage (codeSigning); EKU alone produces "Invalid Key Usage
# for policy" when codesign tries to use the identity.
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$TMPDIR/key.pem" \
  -out "$TMPDIR/cert.pem" \
  -days 3650 \
  -subj "/CN=$CERT_NAME" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=codeSigning" \
  2>/dev/null

# OpenSSL 3.x defaults to PBKDF2 + HMAC-SHA256 for PKCS12 MAC, which
# macOS `security import` can't verify. -legacy + -macalg sha1 + the
# explicit PBE algorithms keep everything in the older format the
# Security framework expects.
echo "Packaging as PKCS12..."
P12_PASS="stint-dev"
openssl pkcs12 -export -legacy \
  -macalg sha1 \
  -keypbe PBE-SHA1-3DES \
  -certpbe PBE-SHA1-3DES \
  -out "$TMPDIR/$CERT_NAME.p12" \
  -inkey "$TMPDIR/key.pem" \
  -in "$TMPDIR/cert.pem" \
  -name "$CERT_NAME" \
  -passout pass:"$P12_PASS"

echo "Importing into login keychain..."
security import "$TMPDIR/$CERT_NAME.p12" \
  -k "$KEYCHAIN" \
  -P "$P12_PASS" \
  -A \
  -T /usr/bin/codesign

# Allow codesign to use the key without an interactive password prompt.
security set-key-partition-list \
  -S "apple-tool:,apple:,codesign:" \
  -s -k "" "$KEYCHAIN" >/dev/null 2>&1 || true

# Trust the self-signed cert for code signing so `codesign -s stint-dev`
# recognises it as a valid identity. This is user-domain trust only
# (your account on this machine); it does NOT affect Gatekeeper for
# anyone else.
echo "Trusting cert for code signing..."
security add-trusted-cert -p codeSign -k "$KEYCHAIN" "$TMPDIR/cert.pem"

echo
echo "$CERT_NAME identity created."
echo "Verify with:  security find-identity -v -p codesigning | grep $CERT_NAME"
