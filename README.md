## RwLock structure — rustsystem-server

### Overview

All meeting state lives in `ActiveMeetings`:

```
Arc<AsyncRwLock<HashMap<MUuid, Arc<Meeting>>>>
```

The outer `AsyncRwLock` wraps the map of all meetings. Inside each entry is an `Arc<Meeting>` whose fields carry their own individual locks.

### Outer map lock

The outer map is held in **write mode** only when the map itself changes:

| Operation       | Outer lock                                          |
| --------------- | --------------------------------------------------- |
| Create meeting  | Write                                               |
| Close meeting   | Write                                               |
| Everything else | Read (just long enough to clone the `Arc<Meeting>`) |

The `AppState::get_meeting()` helper encapsulates the common case: it acquires a read lock, clones the `Arc<Meeting>`, and releases the lock before returning. Callers then work with the `Arc` without holding the outer lock at all.

### Per-field locks inside `Meeting`

```rust
pub struct Meeting {
    pub title: String,           // immutable after construction — no lock
    pub start_time: SystemTime,  // immutable after construction — no lock
    pub locked: AtomicBool,      // simple flag — atomic, no lock
    pub voters:     AsyncRwLock<HashMap<Uuid, Voter>>,
    pub vote_auth:  AsyncRwLock<VoteAuthority>,
    pub invite_auth: AsyncRwLock<InviteAuthority>,
    pub admin_auth:  AsyncRwLock<AdminAuthority>,
}
```

Each authority is locked independently. Operations that only need `vote_auth` do not block operations that only need `voters`, and vice versa.

### Lock ordering

When an operation must acquire more than one field lock, it always does so in this order to prevent deadlock:

1. `voters`
2. `vote_auth`
3. `invite_auth`
4. `admin_auth`

Operations that currently acquire multiple locks:

| Endpoint      | Locks acquired (in order)                                                                         |
| ------------- | ------------------------------------------------------------------------------------------------- |
| `start-vote`  | `vote_auth.write` → `voters.write` (safe: nothing holds `voters.write` and waits for `vote_auth`) |
| `login`       | `voters.write` → `invite_auth.write` → `admin_auth.write`                                         |
| `new-voter`   | `voters.write` → `admin_auth.write`                                                               |
| `reset-login` | `voters.write` → `admin_auth.write`                                                               |

`start-vote` acquires `vote_auth.write` before `voters.write` to make the "check inactive, then start" sequence atomic. This does not violate the ordering because no other operation holds `voters.write` and then waits for `vote_auth.write`.

## RwLock structure — rustsystem-trustauth

### Overview

All round state lives in `ActiveRounds`:

```
Arc<AsyncRwLock<HashMap<Uuid, Arc<RoundState>>>>
```

Same two-level pattern as the server: the outer `AsyncRwLock` wraps the map of all rounds; inside each entry is an `Arc<RoundState>` whose mutable field carries its own lock.

### Outer map lock

| Operation       | Outer lock                                           |
| --------------- | ---------------------------------------------------- |
| Start round     | Write                                                |
| Everything else | Read (just long enough to clone the `Arc<RoundState>`) |

`AppState::get_round()` acquires a read lock, clones the `Arc<RoundState>`, and releases the lock before returning.

### Per-field locks inside `RoundState`

```rust
pub struct RoundState {
    pub keys: AuthenticationKeys,  // immutable after construction — no lock
    pub header: Vec<u8>,           // immutable after construction — no lock
    pub registered_voters: AsyncRwLock<HashMap<Uuid, VoterRegistration>>,
}
```

`keys` and `header` are set once by `start-round` and never modified. Only `registered_voters` needs a lock.

| Endpoint       | `registered_voters` lock |
| -------------- | ------------------------ |
| `register`     | Write                    |
| `is-registered`| Read                     |
| `vote-data`    | Read                     |
