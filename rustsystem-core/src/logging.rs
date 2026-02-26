use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;

use tracing::{Event, Subscriber};
use tracing::field::{Field, Visit};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

// ─── Field visitor ────────────────────────────────────────────────────────────

#[derive(Default)]
struct FieldVisitor {
    muuid: Option<String>,
    message: Option<String>,
    extras: Vec<(String, String)>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        // tracing uses `DisplayValue` for `%` sigils; its Debug output is the Display
        // representation of the inner value (no extra quotes).
        // For format-interpolated messages, tracing calls record_debug (not record_str),
        // so we must capture "message" here too.
        let s = format!("{value:?}");
        match field.name() {
            "muuid" => self.muuid = Some(s),
            "message" => self.message = Some(s),
            _ => self.extras.push((field.name().to_string(), s)),
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        // For static string messages, tracing calls record_str for the "message" field.
        match field.name() {
            "muuid" => self.muuid = Some(value.to_string()),
            "message" => self.message = Some(value.to_string()),
            _ => self.extras.push((field.name().to_string(), value.to_string())),
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.extras
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.extras
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.extras
            .push((field.name().to_string(), value.to_string()));
    }
}

// ─── Per-meeting log layer ────────────────────────────────────────────────────

/// A tracing `Layer` that writes log events containing a `muuid` field to a
/// per-meeting log file at `meetings/<muuid>/log`.  All other events are silently
/// ignored by this layer (they are still handled by the main formatter layer).
///
/// Both `rustsystem-server` and `rustsystem-trustauth` use this layer so that
/// a single `meetings/<muuid>/log` file contains the full picture of what
/// happened in a meeting across both services.
pub struct MeetingLogLayer;

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for MeetingLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let Some(muuid) = visitor.muuid else { return };

        let dir = format!("meetings/{muuid}");
        if fs::create_dir_all(&dir).is_err() {
            return;
        }

        let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{dir}/log"))
        else {
            return;
        };

        let now = chrono::Local::now();
        let level = event.metadata().level();
        let message = visitor.message.as_deref().unwrap_or("");

        let extras = if visitor.extras.is_empty() {
            String::new()
        } else {
            let kv: Vec<String> = visitor
                .extras
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            format!("  |  {}", kv.join("  "))
        };

        let _ = writeln!(
            file,
            "[{}] {:<5}  {}{}",
            now.format("%Y-%m-%d %H:%M:%S"),
            level.to_string(),
            message,
            extras
        );
    }
}

// ─── Guard newtype ────────────────────────────────────────────────────────────

/// Holds the background log-writer guard returned by `tracing-appender`.
/// **Must** be kept alive for the entire lifetime of the process; dropping it
/// flushes and shuts down the background thread.
#[allow(dead_code)] // held purely for its Drop side-effect (flushes log thread)
pub struct LogGuard(tracing_appender::non_blocking::WorkerGuard);

// ─── Initialiser ──────────────────────────────────────────────────────────────

/// Initialise the global tracing subscriber.
///
/// `log_file` is the filename written inside the `logs/` directory, e.g.
/// `"server.log"` or `"trustauth.log"`.
///
/// Layers configured:
/// - `stderr` — human-readable coloured output for the terminal.
/// - `logs/<log_file>` — machine-readable log rotated daily.
/// - [`MeetingLogLayer`] — per-meeting files at `meetings/<muuid>/log`.
///
/// Returns a [`LogGuard`] that **must** be held for the lifetime of the
/// process; dropping it flushes and shuts down the background log thread.
pub fn init_logging(log_file: &str) -> LogGuard {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let file_appender = tracing_appender::rolling::daily("logs", log_file);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into()))
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(MeetingLogLayer)
        .init();

    LogGuard(guard)
}
