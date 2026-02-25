/**
 * Browser-side decryption of .enc tally files produced by the server.
 *
 * File format (matches tally_encrypt.rs):
 *   ephemeral_pk (32 bytes) || nonce (12 bytes) || ChaCha20-Poly1305 ciphertext+tag
 *
 * Key derivation:
 *   shared_secret = X25519(private_key, ephemeral_pk)
 *   key           = HKDF-SHA256(ikm=shared_secret, salt=ephemeral_pk, info="rustsystem-tally-v1")[0..32]
 */

import sodium from "libsodium-wrappers";

function base64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/**
 * Decrypt a single base64-encoded tally .enc file.
 *
 * @param encryptedBase64 - base64-encoded bytes as returned by /api/host/get-all-tally
 * @param privateKey      - 32-byte X25519 private key (= PBKDF2 seed from deriveX25519PrivateKeyFromPassword)
 * @returns parsed JSON content of the tally file
 */
export async function decryptTallyFile(
  encryptedBase64: string,
  privateKey: Uint8Array,
): Promise<unknown> {
  await sodium.ready;

  const encrypted = base64ToBytes(encryptedBase64);
  // Minimum: 32 (ephemeral pk) + 12 (nonce) + 16 (Poly1305 tag) = 60 bytes
  if (encrypted.length < 60) {
    throw new Error("File too short to be a valid tally.enc");
  }

  const ephemeralPk = encrypted.slice(0, 32);
  const nonce = encrypted.slice(32, 44);
  const ciphertext = encrypted.slice(44);

  // X25519 ECDH: shared_secret = private_key * ephemeral_pk
  const sharedSecret = sodium.crypto_scalarmult(privateKey, ephemeralPk);

  // HKDF-SHA256: salt=ephemeralPk, ikm=sharedSecret, info="rustsystem-tally-v1", length=32
  const hkdfKey = await crypto.subtle.importKey(
    "raw",
    sharedSecret,
    { name: "HKDF" },
    false,
    ["deriveBits"],
  );
  const okmBuffer = await crypto.subtle.deriveBits(
    {
      name: "HKDF",
      hash: "SHA-256",
      salt: ephemeralPk,
      info: new TextEncoder().encode("rustsystem-tally-v1"),
    },
    hkdfKey,
    32 * 8, // 32 bytes = the ChaCha20 key
  );
  const key = new Uint8Array(okmBuffer);

  // ChaCha20-Poly1305 IETF decrypt (12-byte nonce, matches Rust's ChaCha20Poly1305)
  const plaintext = sodium.crypto_aead_chacha20poly1305_ietf_decrypt(
    null, // secret_nonce (unused in this API)
    ciphertext,
    null, // no additional data
    nonce,
    key,
  );

  return JSON.parse(new TextDecoder().decode(plaintext));
}
