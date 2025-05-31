use std::collections::{HashMap, HashSet};

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VotingId(u16);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token(Uuid);

pub struct Ballot {
    /// `None` if the user voted for no option (blank vote).
    pub option_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VotingSummary {
    title: String,
    options: Vec<String>,
    total_votes: usize,
    registered_voters: usize,
}

type Ballots = HashMap<Token, Option<Ballot>>;

#[derive(Debug, Clone)]
pub struct Tally {
    tally: Vec<usize>,
    blank_votes: usize,
}

impl Tally {
    fn from_ballots(ballots: &Ballots, n_options: usize) -> Self {
        let mut tally = vec![0; n_options];
        let mut blank_votes = 0;

        for ballot in ballots.values().flatten() {
            if let Some(index) = ballot.option_index {
                tally[index] += 1;
            } else {
                blank_votes += 1;
            }
        }

        Self { tally, blank_votes }
    }

    fn total_votes(&self) -> usize {
        self.tally.iter().sum::<usize>() + self.blank_votes
    }
}

enum State {
    Active(Ballots),
    Counted(Tally),
}

impl State {
    fn ballots_mut(&mut self) -> Option<&mut Ballots> {
        match self {
            State::Active(ballots) => Some(ballots),
            State::Counted(_) => None,
        }
    }

    fn total_votes(&self) -> usize {
        match self {
            State::Active(ballots) => ballots.values().filter(|b| b.is_some()).count(),
            State::Counted(tally) => tally.total_votes(),
        }
    }
}

pub struct Voting {
    title: String,
    options: Vec<String>,
    voters: HashSet<UserId>,
    state: State,
}

pub enum RegisterVoterError {
    AlreadyRegistered,
    Closed,
}

pub enum CastVoteError {
    AlreadyVoted,
    InvalidToken,
    IndexOutOfBounds,
    TooLate,
}

impl Voting {
    pub fn new(title: String, options: Vec<String>) -> Self {
        Self {
            title,
            options,
            voters: HashSet::new(),
            state: State::Active(HashMap::new()),
        }
    }

    pub fn register_voter(&mut self, user_id: UserId) -> Result<Token, RegisterVoterError> {
        let ballots = self.state.ballots_mut().ok_or(RegisterVoterError::Closed)?;

        if self.voters.insert(user_id) {
            let token = Token(Uuid::new_v4());
            assert!(
                ballots.insert(token, None).is_none(),
                "token should be unique"
            );
            Ok(token)
        } else {
            Err(RegisterVoterError::AlreadyRegistered)
        }
    }

    pub fn vote(&mut self, token: Token, ballot: Ballot) -> Result<(), CastVoteError> {
        if ballot.option_index.is_some_and(|i| i >= self.options.len()) {
            return Err(CastVoteError::IndexOutOfBounds);
        }

        let slot = self
            .state
            .ballots_mut()
            .ok_or(CastVoteError::TooLate)?
            .get_mut(&token)
            .ok_or(CastVoteError::InvalidToken)?;

        if slot.is_some() {
            Err(CastVoteError::AlreadyVoted)
        } else {
            *slot = Some(ballot);
            Ok(())
        }
    }

    pub fn summary(&self) -> VotingSummary {
        VotingSummary {
            title: self.title.clone(),
            options: self.options.clone(),
            total_votes: self.state.total_votes(),
            registered_voters: self.voters.len(),
        }
    }

    pub fn close(&mut self) {
        match &mut self.state {
            State::Active(ballots) => {
                let tally = Tally::from_ballots(ballots, self.options.len());
                self.state = State::Counted(tally);
            }
            State::Counted(_) => {
                // Already closed, do nothing
            }
        }
    }

    pub fn tally(&self) -> Option<&Tally> {
        match &self.state {
            State::Active(_) => None,
            State::Counted(tally) => Some(tally),
        }
    }
}
