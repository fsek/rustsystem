#!/usr/bin/env bash
set -euo pipefail

PASSWORD="${1:?password required}"
OUT_DIR="${2:-out}"

echo "$SALT_HEX"
echo "$KEYGEN_ITERATIONS"
echo "$PASSWORD"

# Validate salt
if ! [[ "$SALT_HEX" =~ ^[0-9a-fA-F]+$ ]] || (( ${#SALT_HEX} % 2 != 0 )); then
  echo "salt_hex must be even-length hex" >&2
  exit 2
fi

mkdir -p "$OUT_DIR"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

# 1) Deterministic 32-byte seed via PBKDF2-HMAC-SHA256
#
# NOTE: "openssl enc -pbkdf2 -S" silently truncates the salt to 8 bytes
# (PKCS5_SALT_LEN), so it cannot be used here when SALT_HEX is longer.
# Python's hashlib.pbkdf2_hmac uses the full salt, matching WebCrypto exactly.
SEED_HEX="$(python3 - <<EOF
import hashlib, binascii
pw   = b"${PASSWORD}"
salt = binascii.unhexlify("${SALT_HEX}")
dk   = hashlib.pbkdf2_hmac("sha256", pw, salt, ${KEYGEN_ITERATIONS}, dklen=32)
print(binascii.hexlify(dk).decode())
EOF
)"

if [[ ${#SEED_HEX} -ne 64 ]]; then
  echo "Expected 32-byte seed (64 hex chars), got ${#SEED_HEX}" >&2
  echo "Got: $SEED_HEX" >&2
  exit 3
fi

echo "PBKDF2 seed (compare with frontend debug log): $SEED_HEX"

# 2) Build X25519 PKCS#8 DER explicitly from the seed.
#
# X25519 PKCS#8 DER layout (RFC 5958 + RFC 8410):
#   30 2e                    SEQUENCE (46 bytes)
#     02 01 00               INTEGER 0  (version)
#     30 05                  SEQUENCE   (AlgorithmIdentifier)
#       06 03 2b 65 6e       OID 1.3.101.110 (id-X25519)
#     04 22                  OCTET STRING (34 bytes)
#       04 20                OCTET STRING (32 bytes, CurvePrivateKey)
#         <32-byte scalar>
#
# Header is always the fixed 16 bytes below; seed follows immediately.
HEADER_HEX="302e020100300506032b656e04220420"
printf '%s%s' "$HEADER_HEX" "$SEED_HEX" | xxd -r -p > "$TMPDIR/key.der"

# 3) Output final PEM keys
PRIV_PEM="$OUT_DIR/private_x25519.pem"
PUB_PEM="$OUT_DIR/public_x25519.pem"

openssl pkey -inform DER -in "$TMPDIR/key.der" -out "$PRIV_PEM"
openssl pkey -in "$PRIV_PEM" -pubout -out "$PUB_PEM"

echo "Wrote:"
echo "  $PRIV_PEM"
echo "  $PUB_PEM"
echo
echo "Public key fingerprint:"
openssl pkey -pubin -in "$PUB_PEM" -outform DER | openssl dgst -sha256
