/**
 * BBS+ Blind Signature commitment implementation.
 *
 * Matches zkryptium 0.5.0 BbsBls12381Sha256 ciphersuite exactly.
 * The server uses `Commitment::commit` / `BlindSignature::blind_sign` from zkryptium.
 *
 * Wire format for registration (POST /api/voter/register):
 *   Sha256RegistrationInfo { context: ProofContext, commitment: Commitment<BbsBls12381Sha256> }
 *
 * The commitment JSON format mirrors zkryptium's serde output:
 *   { "BBSplus": { "commitment": "<hex>", "proof": { "s_cap": "<hex>", "m_cap": ["<hex>"], "challenge": "<hex>" } } }
 *
 * Wire format for voting (POST /api/voter/submit):
 *   Ballot { metadata, choice, validation: { proof: [u8;32], token: Vec<u8>, signature: BlindSignature }, _padding }
 */

import { bls12_381 } from "@noble/curves/bls12-381.js";
import { sha256 } from "@noble/hashes/sha2.js";
import { expand_message_xmd } from "@noble/curves/abstract/hash-to-curve.js";

const Fr = bls12_381.fields.Fr;
const G1 = bls12_381.G1;

// ─── Constants ───────────────────────────────────────────────────────────────

const EXPAND_LEN = 48; // bytes
const TOKEN_SIZE = 256; // bytes

// BbsBls12381Sha256 ciphersuite identifiers (from zkryptium/src/bbsplus/ciphersuites.rs)
const API_ID_BLIND = utf8("BBS_BLS12381G1_XMD:SHA-256_SSWU_RO_BLIND_H2G_HM2S_");

// Blind generator prefix: "BLIND_" + API_ID_BLIND
// Used as the api_id passed to Generators::create when creating blind generators
const BLIND_PREFIX = concat(utf8("BLIND_"), API_ID_BLIND);

// Generator DSTs (from create_generators in zkryptium/src/bbsplus/generators.rs)
const GENERATOR_SEED_DST = concat(BLIND_PREFIX, utf8("SIG_GENERATOR_SEED_"));
const GENERATOR_DST = concat(BLIND_PREFIX, utf8("SIG_GENERATOR_DST_"));
const GENERATOR_SEED = concat(BLIND_PREFIX, utf8("MESSAGE_GENERATOR_SEED"));

// Message-to-scalar DST (from BBSplusMessage::messages_to_scalar)
const MAP_MSG_DST = concat(API_ID_BLIND, utf8("MAP_MSG_TO_SCALAR_AS_HASH_"));

// Challenge DST (from calculate_blind_challenge)
// blind_challenge_dst = API_ID_BLIND + H2S where H2S = b"H2S_"
const BLIND_CHALLENGE_DST = concat(API_ID_BLIND, utf8("H2S_"));

// ─── Byte helpers ────────────────────────────────────────────────────────────

function utf8(s: string): Uint8Array {
	return new TextEncoder().encode(s);
}

function concat(...arrays: Uint8Array[]): Uint8Array {
	const total = arrays.reduce((n, a) => n + a.length, 0);
	const result = new Uint8Array(total);
	let offset = 0;
	for (const a of arrays) {
		result.set(a, offset);
		offset += a.length;
	}
	return result;
}

/** Integer to big-endian byte array (numbers only, max 6 safe bytes) */
function i2osp(n: number, len: number): Uint8Array {
	const bytes = new Uint8Array(len);
	for (let i = len - 1; i >= 0; i--) {
		bytes[i] = n & 0xff;
		n >>= 8;
	}
	return bytes;
}

/** BigInt to big-endian byte array */
function i2ospBig(n: bigint, len: number): Uint8Array {
	const bytes = new Uint8Array(len);
	for (let i = len - 1; i >= 0; i--) {
		bytes[i] = Number(n & 0xffn);
		n >>= 8n;
	}
	return bytes;
}

/** Big-endian bytes to bigint */
function os2ip(bytes: Uint8Array): bigint {
	let result = 0n;
	for (const b of bytes) {
		result = (result << 8n) | BigInt(b);
	}
	return result;
}

/** Encode a Uint8Array as a lowercase hex string. */
function toHex(bytes: Uint8Array): string {
	return Array.from(bytes)
		.map((b) => b.toString(16).padStart(2, "0"))
		.join("");
}

/** Encode bigint scalar as 32-byte big-endian hex string (for JSON to server) */
function scalarToHex(scalar: bigint): string {
	return toHex(i2ospBig(scalar, 32));
}

/** Encode G1 point as compressed 48-byte hex string (for JSON to server) */
function pointToHex(point: ReturnType<typeof G1.hashToCurve>): string {
	return toHex(point.toBytes());
}

// ─── Scalar operations ───────────────────────────────────────────────────────

/**
 * Hash bytes to a BLS12-381 scalar.
 *
 * Implements hash_to_scalar from zkryptium/src/utils/util.rs:
 *   uniform_bytes = expand_message_xmd(data, dst, 48)
 *   scalar = Scalar::from_okm(&uniform_bytes)  ≡  OS2IP(uniform_bytes) mod r
 */
export function hashToScalar(data: Uint8Array, dst: Uint8Array): bigint {
	const expanded = expand_message_xmd(data, dst, EXPAND_LEN, sha256);
	return Fr.create(os2ip(expanded));
}

/**
 * Hash a committed message (token) to its scalar representative.
 * Implements BBSplusMessage::messages_to_scalar with api_id = API_ID_BLIND.
 */
export function messageToScalar(msg: Uint8Array): bigint {
	return hashToScalar(msg, MAP_MSG_DST);
}

/** Generate a cryptographically random scalar. */
function randomScalar(): bigint {
	const bytes = crypto.getRandomValues(new Uint8Array(EXPAND_LEN));
	return Fr.create(os2ip(bytes));
}

// ─── Generator creation ──────────────────────────────────────────────────────

/**
 * Create `count` blind generators.
 *
 * Matches Generators::create::<BbsBls12381Sha256>(count, Some(&[b"BLIND_", API_ID_BLIND].concat()))
 * which calls create_generators from zkryptium/src/bbsplus/generators.rs.
 *
 * generators[0] = Q2 (blinding generator)
 * generators[1] = J0 (message 0 generator)
 */
export function createBlindGenerators(count: number) {
	// Initial seed expansion
	let v = expand_message_xmd(
		GENERATOR_SEED,
		GENERATOR_SEED_DST,
		EXPAND_LEN,
		sha256,
	);

	const generators = [];
	for (let i = 1; i <= count; i++) {
		// v = expand_message_xmd(v || i2osp(i, 8), seed_dst, 48)
		const seedInput = concat(v, i2osp(i, 8));
		v = expand_message_xmd(seedInput, GENERATOR_SEED_DST, EXPAND_LEN, sha256);
		// generator = hash_to_curve(v, generator_dst)
		const generator = G1.hashToCurve(v, { DST: GENERATOR_DST });
		generators.push(generator);
	}

	return generators;
}

// ─── Commitment ──────────────────────────────────────────────────────────────

/**
 * Compute a BBS+ blind commitment for one committed message (the token).
 *
 * Implements core_commit from zkryptium/src/bbsplus/commitment.rs with M=1.
 *
 * Returns the commitment in the exact JSON format that zkryptium's serde Deserialize expects,
 * plus the secret_prover_blind (= blind_factor) needed for vote submission.
 *
 * Commitment byte layout (commitment.to_bytes()):
 *   C.compressed (48B) || s_cap (32B) || m_cap[0] (32B) || challenge (32B) = 144 bytes
 */
export function commit(token: Uint8Array): {
	commitmentJson: CommitmentJson;
	blindFactor: Uint8Array; // 32 bytes, secret_prover_blind
} {
	const [Q2, J0] = createBlindGenerators(2);

	// Convert token to scalar
	const tokenScalar = messageToScalar(token);

	// Random scalars: secret_prover_blind, s_tilde, m_tilde[0]
	const secretProverBlind = randomScalar();
	const sTilde = randomScalar();
	const mTilde = randomScalar();

	// commitment = Q2 * secretProverBlind + J0 * tokenScalar
	const C = Q2.multiply(secretProverBlind).add(J0.multiply(tokenScalar));

	// Cbar = Q2 * sTilde + J0 * mTilde
	const Cbar = Q2.multiply(sTilde).add(J0.multiply(mTilde));

	// challenge = hash_to_scalar(i2osp(M, 8) || Q2 || J0 || C || Cbar, challenge_dst)
	// where M = number of committed messages = 1 (= generators.len() - 1)
	const cArr = concat(
		i2osp(1, 8), // M = 1 (number of committed messages)
		Q2.toBytes(), // 48 bytes
		J0.toBytes(), // 48 bytes
		C.toBytes(), // 48 bytes
		Cbar.toBytes(), // 48 bytes
	);
	const challenge = hashToScalar(cArr, BLIND_CHALLENGE_DST);

	// s_cap = s_tilde + secret_prover_blind * challenge (mod r)
	const sCap = Fr.add(sTilde, Fr.mul(secretProverBlind, challenge));

	// m_cap[0] = m_tilde + token_scalar * challenge (mod r)
	const mCap0 = Fr.add(mTilde, Fr.mul(tokenScalar, challenge));

	// Build the JSON structure matching zkryptium's serde output for Commitment<BbsBls12381Sha256>
	const commitmentJson: CommitmentJson = {
		BBSplus: {
			commitment: pointToHex(C),
			proof: {
				s_cap: scalarToHex(sCap),
				m_cap: [scalarToHex(mCap0)],
				challenge: scalarToHex(challenge),
			},
		},
	};

	return {
		commitmentJson,
		blindFactor: i2ospBig(secretProverBlind, 32),
	};
}

// ─── ProofContext ─────────────────────────────────────────────────────────────

/**
 * Build the ProofContext sent to the server with the registration request.
 *
 * The server deserializes this but does NOT validate the checksum in the register handler,
 * so checksum is zeroed. All fields serialize as number[] in JSON (serde default for Vec<u8>).
 */
export function buildProofContext(
	voterIdBytes: Uint8Array, // 16 bytes (UUID)
	meetingIdBytes: Uint8Array, // 16 bytes (UUID)
): ProofContext {
	const ts = BigInt(Math.floor(Date.now() / 1000));
	const registrationTimestamp = i2ospBig(ts, 8);

	return {
		voter_id: Array.from(voterIdBytes),
		meeting_id: Array.from(meetingIdBytes),
		registration_timestamp: Array.from(registrationTimestamp),
		checksum: Array.from(new Uint8Array(32)), // zeros — not validated
	};
}

// ─── Token generation ─────────────────────────────────────────────────────────

/**
 * Generate a fresh anonymous voting token.
 *
 * Returns everything needed for registration and later vote submission.
 * Keep `token` and `blindFactor` secret — never send them to the server until vote submission.
 */
export function generateToken(
	voterIdBytes: Uint8Array,
	meetingIdBytes: Uint8Array,
): GeneratedToken {
	const token = crypto.getRandomValues(new Uint8Array(TOKEN_SIZE));
	const { commitmentJson, blindFactor } = commit(token);
	const context = buildProofContext(voterIdBytes, meetingIdBytes);

	return { token, blindFactor, commitmentJson, context };
}

// ─── Ballot construction ──────────────────────────────────────────────────────

/**
 * Build a padded ballot for POST /api/voter/submit.
 *
 * The server enforces total JSON size >= 1024 bytes (BALLOT_SIZE in ballot.rs).
 * `_padding` is filled with random bytes to reach the minimum size.
 */
export function buildBallot(
	metadata: BallotMetaData,
	choice: number[] | null,
	token: Uint8Array,
	blindFactor: Uint8Array,
	signature: unknown, // BlindSignature JSON as received from /api/voter/register
): object {
	const validation = {
		proof: Array.from(blindFactor), // [u8; 32] → number[]
		token: Array.from(token), // Vec<u8> → number[]
		signature,
	};

	const base = { metadata, choice, validation, _padding: [] };
	const baseLen = JSON.stringify(base).length;
	const needed = Math.max(0, 1024 - baseLen);
	const padding = Array.from(crypto.getRandomValues(new Uint8Array(needed)));

	return { ...base, _padding: padding };
}

// ─── UUID helper ──────────────────────────────────────────────────────────────

/** Parse a UUID string into its 16 raw bytes (RFC 4122 big-endian order). */
export function uuidToBytes(uuid: string): Uint8Array {
	const hex = uuid.replace(/-/g, "");
	const bytes = new Uint8Array(16);
	for (let i = 0; i < 16; i++) {
		bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
	}
	return bytes;
}

// ─── Types ────────────────────────────────────────────────────────────────────

export interface CommitmentJson {
	BBSplus: {
		commitment: string; // hex string, 48 bytes compressed G1
		proof: {
			s_cap: string; // hex string, 32 bytes scalar
			m_cap: string[]; // one element for our use case
			challenge: string; // hex string, 32 bytes scalar
		};
	};
}

export interface ProofContext {
	voter_id: number[];
	meeting_id: number[];
	registration_timestamp: number[];
	checksum: number[];
}

export interface BallotMetaData {
	candidates: string[];
	max_choices: number;
	protocol_version: number;
}

export interface RegistrationSuccessResponse {
	signature: unknown;
	metadata: BallotMetaData;
}

export interface GeneratedToken {
	token: Uint8Array;
	blindFactor: Uint8Array;
	commitmentJson: CommitmentJson;
	context: ProofContext;
}
