//! What makes a ballot valid (`docs/PROTOCOL.md` §5.3 and §6).
//!
//! A ballot arrives as `prepared = randomizer (32 bytes) ‖ msg` plus trustauth's signature on
//! it. `msg` is a small JSON object; its exact bytes are what was signed:
//!
//! ```json
//! {"v":1,"round":"3f2c…","choice":[0,2],"nonce":"q8Lw…"}
//! ```
//!
//! The signature is checked over the raw bytes *before* they are parsed, so there is no
//! canonicalisation to get wrong. The browser applies the same rules before blinding
//! (`frontend/src/voting/ballot.ts`), so a ballot trustauth signed is never rejected here.

use serde::Deserialize;
use uuid::Uuid;

use rustsystem_core::{
    ApiError, ApiResult, ErrorCode,
    blind::{BlindError, RANDOMIZER_LEN, RoundPublicKey, SIGNATURE_LEN},
    internal::RoundId,
    secret::{b64_decode, sha256},
};

/// The longest `prepared` accepted. A ballot naming 100 candidates is about 500 bytes.
pub const MAX_PREPARED_LEN: usize = 1024;

pub const BALLOT_VERSION: u8 = 1;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Message {
    v: u8,
    round: Uuid,
    choice: Option<Vec<usize>>,
    nonce: String,
}

/// A ballot that passed every check, ready to count.
#[derive(Debug, PartialEq, Eq)]
pub struct ValidBallot {
    /// `SHA-256(prepared)`; the server counts each one once.
    pub hash: [u8; 32],
    /// `None` is a blank vote.
    pub choice: Option<Vec<usize>>,
}

/// The parts of the current round a ballot is checked against.
pub struct RoundRules<'a> {
    pub id: RoundId,
    pub public_key: &'a RoundPublicKey,
    pub candidates: usize,
    pub max_choices: usize,
}

pub fn check(rules: &RoundRules, prepared: &[u8], sig: &[u8]) -> ApiResult<ValidBallot> {
    if prepared.len() <= RANDOMIZER_LEN || prepared.len() > MAX_PREPARED_LEN || sig.len() != SIGNATURE_LEN
    {
        return Err(ErrorCode::MalformedBallot.into());
    }
    let msg = rules.public_key.verify(prepared, sig).map_err(|e| match e {
        BlindError::InvalidSignature => ApiError::new(ErrorCode::InvalidSignature),
        _ => ApiError::new(ErrorCode::MalformedBallot),
    })?;

    let msg: Message = serde_json::from_slice(msg)
        .map_err(|e| invalid(format!("The ballot is not valid JSON: {e}")))?;
    if msg.v != BALLOT_VERSION {
        return Err(invalid(format!("Unsupported ballot version {}.", msg.v)));
    }
    if msg.round != rules.id {
        return Err(ErrorCode::WrongRound.into());
    }
    if b64_decode(&msg.nonce).map(|n| n.len()) != Some(32) {
        return Err(invalid("The ballot nonce must be 32 bytes."));
    }
    if let Some(choice) = &msg.choice {
        check_choice(choice, rules.candidates, rules.max_choices)?;
    }

    Ok(ValidBallot {
        hash: sha256(prepared),
        choice: msg.choice,
    })
}

/// A choice is a non-empty, strictly ascending list of candidate indices — so no duplicates —
/// all in range, and no longer than `max_choices`. A blank vote is `null`, not `[]`.
pub fn check_choice(choice: &[usize], candidates: usize, max_choices: usize) -> ApiResult<()> {
    if choice.is_empty() {
        return Err(invalid("A ballot must choose at least one candidate, or be blank (null)."));
    }
    if choice.len() > max_choices {
        return Err(invalid(format!("At most {max_choices} candidates may be chosen.")));
    }
    if !choice.windows(2).all(|w| w[0] < w[1]) {
        return Err(invalid("Candidates must be listed in ascending order, each once."));
    }
    if choice.iter().any(|&c| c >= candidates) {
        return Err(invalid("The ballot names a candidate that doesn't exist."));
    }
    Ok(())
}

fn invalid(message: impl Into<std::borrow::Cow<'static, str>>) -> ApiError {
    ApiError::with_message(ErrorCode::InvalidBallot, message)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rustsystem_core::{
        blind::{RoundKey, client},
        secret::new_token,
    };

    /// Signs `msg` the way a browser and trustauth would together.
    pub(crate) fn signed(key: &RoundKey, msg: &str) -> (Vec<u8>, Vec<u8>) {
        let blinded = client::blind(key.public_key(), msg.as_bytes()).unwrap();
        let blind_sig = key.blind_sign(&blinded.blinded).unwrap();
        client::finalize(key.public_key(), &blinded, &blind_sig).unwrap()
    }

    pub(crate) fn message(round: RoundId, choice: &str) -> String {
        format!(r#"{{"v":1,"round":"{round}","choice":{choice},"nonce":"{}"}}"#, new_token())
    }

    fn rules(key: &RoundKey, id: RoundId) -> RoundRules<'_> {
        RoundRules { id, public_key: key.public_key(), candidates: 3, max_choices: 2 }
    }

    fn code(result: ApiResult<ValidBallot>) -> ErrorCode {
        result.unwrap_err().code
    }

    #[test]
    fn valid_ballots_pass() {
        let (key, id) = (RoundKey::generate().unwrap(), Uuid::new_v4());
        for (choice, expected) in [("[0,2]", Some(vec![0, 2])), ("[1]", Some(vec![1])), ("null", None)] {
            let (p, s) = signed(&key, &message(id, choice));
            let ballot = check(&rules(&key, id), &p, &s).unwrap();
            assert_eq!(ballot.choice, expected);
            assert_eq!(ballot.hash, sha256(&p));
        }
    }

    #[test]
    fn choice_rules() {
        let (key, id) = (RoundKey::generate().unwrap(), Uuid::new_v4());
        for choice in ["[0,0]", "[2,0]", "[3]", "[0,1,2]", "[]"] {
            let (p, s) = signed(&key, &message(id, choice));
            assert_eq!(code(check(&rules(&key, id), &p, &s)), ErrorCode::InvalidBallot, "{choice}");
        }
    }

    #[test]
    fn wrong_round_is_rejected() {
        let (key, id) = (RoundKey::generate().unwrap(), Uuid::new_v4());
        let (p, s) = signed(&key, &message(Uuid::new_v4(), "[0]"));
        assert_eq!(code(check(&rules(&key, id), &p, &s)), ErrorCode::WrongRound);
    }

    #[test]
    fn signature_must_match() {
        let (key, id) = (RoundKey::generate().unwrap(), Uuid::new_v4());
        let other = RoundKey::generate().unwrap();
        let (p, s) = signed(&other, &message(id, "[0]"));
        assert_eq!(code(check(&rules(&key, id), &p, &s)), ErrorCode::InvalidSignature);

        let (mut p, s) = signed(&key, &message(id, "[0]"));
        let at = p.len() - 20;
        p[at] ^= 1;
        assert_eq!(code(check(&rules(&key, id), &p, &s)), ErrorCode::InvalidSignature);
    }

    #[test]
    fn message_shape_is_strict() {
        let (key, id) = (RoundKey::generate().unwrap(), Uuid::new_v4());
        let nonce = new_token();
        let bad = [
            format!(r#"{{"v":2,"round":"{id}","choice":null,"nonce":"{nonce}"}}"#),
            format!(r#"{{"v":1,"round":"{id}","choice":null,"nonce":"short"}}"#),
            format!(r#"{{"v":1,"round":"{id}","choice":null,"nonce":"{nonce}","voter":"me"}}"#),
            format!(r#"{{"v":1,"round":"{id}","choice":[0],"choice":[1],"nonce":"{nonce}"}}"#),
            "not json".to_owned(),
        ];
        for msg in bad {
            let (p, s) = signed(&key, &msg);
            assert_eq!(code(check(&rules(&key, id), &p, &s)), ErrorCode::InvalidBallot, "{msg}");
        }
    }

    #[test]
    fn sizes_are_checked_first() {
        let (key, id) = (RoundKey::generate().unwrap(), Uuid::new_v4());
        let r = rules(&key, id);
        assert_eq!(code(check(&r, &[0; 32], &[0; SIGNATURE_LEN])), ErrorCode::MalformedBallot);
        assert_eq!(code(check(&r, &[0; MAX_PREPARED_LEN + 1], &[0; SIGNATURE_LEN])), ErrorCode::MalformedBallot);
        assert_eq!(code(check(&r, &[0; 64], &[0; 10])), ErrorCode::MalformedBallot);
    }
}
