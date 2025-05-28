use axum::{
    Router,
    response::{Html, IntoResponse},
    routing::get,
};
use axum_server::tls_rustls::RustlsConfig;
use std::{fs, net::SocketAddr, str::from_utf8};
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .nest("/remote", rustsystem_remote::router())
        .nest_service(
            "/www",
            ServeDir::new("../rustsystem-client/wrapper").append_index_html_on_directories(false),
        )
        .nest_service("/pkg", ServeDir::new("../rustsystem-client/pkg"));

    let config = RustlsConfig::from_pem_file("certs/server.crt", "certs/server.key")
        .await
        .unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], 8443));
    println!("listening on {}", addr);
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../../rustsystem-client/wrapper/index.html"))
}
