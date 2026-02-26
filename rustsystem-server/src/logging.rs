pub use rustsystem_core::logging::LogGuard;

/// Initialise logging for `rustsystem-server`.
/// Writes to `logs/server.log` (daily rolling) and per-meeting files at
/// `meetings/<muuid>/log`.  The returned [`LogGuard`] must be held for the
/// lifetime of the process.
pub fn init_logging() -> LogGuard {
    rustsystem_core::logging::init_logging("server.log")
}
