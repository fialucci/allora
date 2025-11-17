//! Internal channel logging utilities.
//! Centralizes tracing helpers used by queue (and future channel kinds).
//! Not part of public API; functions are `pub(crate)`.

use crate::Exchange;
use tracing::trace;

/// Log a send enqueue event.
pub(crate) fn log_send_enqueued(
    channel_id: &str,
    exchange: &Exchange,
    is_async: bool,
    corr_id: Option<&str>,
) {
    trace!(
        target: "allora::channel",
        channel_id = %channel_id,
        async = is_async,
        corr_id = ?corr_id,
        in_body = ?exchange.in_msg.body_text(),
        "send enqueued"
    );
}

/// Generic receive logger (low-level; prefer wrapper helpers).
pub(crate) fn log_receive(
    channel_id: &str,
    kind: &'static str,
    phase: &'static str,
    is_async: bool,
    exchange: Option<&Exchange>,
    queue_size: Option<usize>,
    corr_id: Option<&str>,
    attempts: Option<u32>,
    elapsed_ms: Option<u128>,
    timeout_ms: Option<u128>,
) {
    trace!(
        target: "allora::channel",
        channel_id = %channel_id,
        kind = %kind,
        phase = %phase,
        async = is_async,
        queue_size = ?queue_size,
        corr_id = ?corr_id,
        attempts = ?attempts,
        elapsed_ms = ?elapsed_ms,
        timeout_ms = ?timeout_ms,
        in_body = ?exchange.and_then(|e| e.in_msg.body_text()),
        out_body = ?exchange.and_then(|e| e.out_msg.as_ref().and_then(|m| m.body_text())),
        "receive dequeued"
    );
}

pub(crate) fn log_dequeued(
    channel_id: &str,
    kind: &'static str,
    is_async: bool,
    exchange: &Exchange,
    queue_size: Option<usize>,
    corr_id: Option<&str>,
) {
    log_receive(
        channel_id,
        kind,
        "dequeued",
        is_async,
        Some(exchange),
        queue_size,
        corr_id,
        None,
        None,
        None,
    );
}

pub(crate) fn log_empty(
    channel_id: &str,
    kind: &'static str,
    is_async: bool,
    queue_size: Option<usize>,
    corr_id: Option<&str>,
) {
    log_receive(
        channel_id, kind, "empty", is_async, None, queue_size, corr_id, None, None, None,
    );
}

pub(crate) fn log_phase(
    channel_id: &str,
    kind: &'static str,
    phase: &'static str,
    is_async: bool,
    attempts: Option<u32>,
    start: Option<&std::time::Instant>,
    timeout: Option<std::time::Duration>,
) {
    log_receive(
        channel_id,
        kind,
        phase,
        is_async,
        None,
        None,
        None,
        attempts,
        start.map(|s| s.elapsed().as_millis()),
        timeout.map(|t| t.as_millis()),
    );
}
