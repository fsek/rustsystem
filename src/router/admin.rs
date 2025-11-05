use askama::Template;
use askama_web::WebTemplate;
use axum::{
    Form, Router,
    extract::{FromRequestParts, Path, Query, State, WebSocketUpgrade},
    http::{StatusCode, Uri, uri},
    response::{IntoResponse, Redirect, Response},
    routing::{any, get, post},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use base64::{Engine, prelude::BASE64_STANDARD};
use qrcode::QrCode;
use rand::seq::SliceRandom;
use uuid::Uuid;

use crate::{
    AppState,
    app_state::{INVITE_TTL, Meeting, VotingInfo, VotingResults},
    router::MeetingPath,
};

// #[derive(Debug, serde::Deserialize)]
// struct CreateMeetingForm {}

async fn create_meeting(
    state: State<AppState>,
    cookie_jar: CookieJar,
    // form: Form<CreateMeetingForm>,
) -> impl IntoResponse {
    let (meeting_id, meeting) = state.meetings.create();

    (
        cookie_jar.add(Cookie::new(
            "admin_token",
            meeting.admin_token().to_string(),
        )),
        Redirect::to(&format!("/{meeting_id}/admin")),
    )
}

#[derive(Template, WebTemplate)]
#[template(path = "admin.html")]
struct AdminTemplate {
    meeting_id: Uuid,
    current_voting: Option<VotingInfo>,
    past_votings: Vec<VotingResults>,
}

async fn view_meeting(admin: MeetingAdmin) -> impl IntoResponse {
    AdminTemplate {
        meeting_id: admin.meeting_id,
        current_voting: admin.meeting.current_voting(),
        past_votings: admin.meeting.past_votings(),
    }
}

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum Checkbox {
    On,
    #[default]
    Off,
}

#[derive(serde::Deserialize)]
struct VotingForm {
    #[serde(default)]
    title: String,
    #[serde(default)]
    options: String,
    #[serde(default)]
    shuffle: Checkbox,
}

async fn form_action(admin: MeetingAdmin, Form(form): Form<VotingForm>) -> impl IntoResponse {
    if form.title.is_empty() {
        // interpret as stop voting
        admin.meeting.stop_voting();
    } else {
        let mut options = form
            .options
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        if matches!(form.shuffle, Checkbox::On) {
            options.shuffle(&mut rand::rng());
        }

        admin.meeting.start_voting(crate::app_state::VotingInfo {
            title: form.title,
            options,
        });
    }

    Redirect::to(&format!("/{}/admin", admin.meeting_id))
}

#[derive(Template, WebTemplate)]
#[template(path = "invite.html")]
struct InviteTemplate {
    qr_url: String,
    ws_url: String,
}

async fn show_invite_link(state: State<AppState>, mut admin: MeetingAdmin) -> impl IntoResponse {
    let invite_id = admin.meeting.create_invitation();

    let invite_url = uri::Builder::from(Uri::clone(&state.public_uri))
        .path_and_query(format!("/{}?invite={invite_id}", admin.meeting_id))
        .build()
        .unwrap()
        .to_string();

    let qr = QrCode::new(invite_url).unwrap();
    let svg = qr
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(200, 200)
        .max_dimensions(200, 200)
        .build();
    let svg_b64 = BASE64_STANDARD.encode(svg.as_bytes());

    let ws_url = state
        .ws_base_uri()
        .path_and_query(format!("/{}/ws", admin.meeting_id))
        .build()
        .unwrap()
        .to_string();

    (
        [("refresh", INVITE_TTL.as_secs() - 10)],
        InviteTemplate {
            qr_url: format!("data:image/svg+xml;base64,{svg_b64}"),
            ws_url,
        },
    )
}

async fn admin_ws(ws: WebSocketUpgrade, _admin: MeetingAdmin) -> impl IntoResponse {
    ws.on_upgrade(|_socket| async {})
}

struct MeetingAdmin {
    meeting_id: Uuid,
    meeting: Meeting,
}

impl FromRequestParts<AppState> for MeetingAdmin {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        #[derive(Debug, serde::Deserialize)]
        struct AdminTokenQuery {
            admin_token: Option<String>,
        }

        let meeting_id = Path::<MeetingPath>::from_request_parts(parts, state)
            .await
            .map_err(|rejection| rejection.into_response())?
            .meeting_id;

        let query = Query::<AdminTokenQuery>::from_request_parts(parts, state)
            .await
            .map_err(|rejection| rejection.into_response())?;

        let cookie_jar = CookieJar::from_request_parts(parts, state).await.unwrap();

        if let Some(query_token) = query.0.admin_token {
            // We don't want to keep a secret in the address bar, let's store it in
            // a cookie and redirect immediately
            //
            // TODO: preserve query params except admin_token
            let redirect_url = parts.uri.path();

            Err((
                cookie_jar.add(Cookie::new("admin_token", query_token)),
                Redirect::to(redirect_url),
            )
                .into_response())
        } else {
            let meeting = state.meetings.get(&meeting_id).ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    format!("Möte med ID {} hittades inte", meeting_id),
                )
                    .into_response()
            })?;

            let cookie_token = cookie_jar
                .get("admin_token")
                .and_then(|cookie| cookie.value().parse().ok());

            if Some(meeting.admin_token()) != cookie_token {
                return Err((
                    StatusCode::FORBIDDEN,
                    "Ogiltig eller saknad admin-token".to_string(),
                )
                    .into_response());
            }

            Ok(Self {
                meeting_id,
                meeting,
            })
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/new", post(create_meeting))
        .route("/{meeting_id}/admin", get(view_meeting).post(form_action))
        .route("/{meeting_id}/invite", get(show_invite_link))
        .route("/{meeting_id}/ws", any(admin_ws))
}
