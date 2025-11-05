use askama::Template;
use askama_web::WebTemplate;
use axum::{
    Form, Router,
    extract::{FromRequestParts, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use uuid::Uuid;

use crate::{
    app_state::{AppState, Ballot, CastVoteError, Meeting, VotingInfo},
    router::MeetingPath,
};

#[derive(Template, WebTemplate)]
#[template(path = "voter.html")]
struct VoterTemplate {
    voting: Option<VotingInfo>,
}

async fn main_page(voter: MeetingVoter) -> impl IntoResponse {
    let voting = voter.meeting.current_voting();

    VoterTemplate { voting }
}

/// Kind of like [`MeetingAdmin`], but for voters
struct MeetingVoter {
    voter_id: Uuid,
    meeting: Meeting,
}

#[derive(serde::Deserialize)]
struct CastVoteForm {
    option: usize,
}

#[derive(Template, WebTemplate)]
#[template(path = "vote_response.html")]
struct VoteResponseTemplate {
    message: &'static str,
    // status: StatusCode,
}

async fn cast_vote(voter: MeetingVoter, form: Form<CastVoteForm>) -> impl IntoResponse {
    let (status, message) = match voter.meeting.cast_vote(
        &voter.voter_id,
        Ballot {
            option: form.option,
        },
    ) {
        Ok(()) => (StatusCode::OK, "Din röst har registrerats!"),
        Err(e) => match e {
            CastVoteError::VoterNotFound => (
                StatusCode::FORBIDDEN,
                "Du är inte behörig att rösta i detta möte.",
            ),
            CastVoteError::AlreadyVoted => (StatusCode::BAD_REQUEST, "Du har redan röstat."),
        },
    };

    (status, VoteResponseTemplate { message })
}

impl FromRequestParts<AppState> for MeetingVoter {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        #[derive(Debug, serde::Deserialize)]
        struct InviteQuery {
            invite: Option<Uuid>,
        }

        let meeting_id = Path::<MeetingPath>::from_request_parts(parts, state)
            .await
            .map_err(|rejection| rejection.into_response())?
            .meeting_id;

        let query = Query::<InviteQuery>::from_request_parts(parts, state)
            .await
            .map_err(|rejection| rejection.into_response())?;

        let cookie_jar = CookieJar::from_request_parts(parts, state).await.unwrap();

        let mut meeting = state.meetings.get(&meeting_id).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Möte med ID {} hittades inte", meeting_id),
            )
                .into_response()
        })?;

        if let Some(invite) = query.0.invite {
            // We don't want to keep a secret in the address bar
            //
            // TODO: preserve query params except invite
            let redirect_url = parts.uri.path();

            let voting_token = meeting
                .use_invitation(&invite)
                .ok_or_else(|| (StatusCode::FORBIDDEN, "oops").into_response())?;

            Err((
                cookie_jar.add(Cookie::new("voting_token", voting_token.to_string())),
                Redirect::to(redirect_url),
            )
                .into_response())
        } else {
            let token_cookie = cookie_jar
                .get("voting_token")
                .map(|cookie| cookie.value())
                .unwrap_or_default();

            if let Some(voting_token) = token_cookie.parse().ok()
                && meeting.contains_voter(&voting_token)
            {
                Ok(Self {
                    voter_id: voting_token,
                    meeting,
                })
            } else {
                // No voting token in cookie, or invalid token
                Err((StatusCode::FORBIDDEN, "Du saknar rösträtt :P").into_response())
            }
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new().route("/{meeting_id}", get(main_page).post(cast_vote))
}
