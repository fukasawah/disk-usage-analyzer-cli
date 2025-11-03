//! Progress reporting primitives for traversal strategies.

use crate::models::ProgressSnapshot;
use std::time::{Duration, Instant};

/// Default time-based interval used when no override is supplied.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(2);
/// Default byte delta that forces a progress update when the default interval is used.
pub const DEFAULT_BYTE_TRIGGER: u64 = 1_000_000;
/// Minimum duration enforced for interval throttling to avoid busy emissions.
const MIN_INTERVAL: Duration = Duration::from_millis(100);
/// Lower bound for byte-triggered emissions to avoid excessively chatty progress.
const MIN_BYTE_TRIGGER: u64 = 64 * 1024;

/// Time/byte-based throttler governing progress event emission.
#[derive(Debug)]
pub struct ProgressThrottler {
    interval: Duration,
    byte_trigger: u64,
    last_emit: Option<Instant>,
    last_emit_bytes: u64,
}

impl Default for ProgressThrottler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressThrottler {
    /// Construct a throttler using the default interval of two seconds.
    #[must_use]
    pub fn new() -> Self {
        Self::with_interval(DEFAULT_INTERVAL)
    }

    /// Construct a throttler with the supplied minimum interval.
    #[must_use]
    pub fn with_interval(interval: Duration) -> Self {
        Self::with_interval_and_trigger(interval, DEFAULT_BYTE_TRIGGER)
    }

    /// Construct a throttler with explicit parameters.
    #[must_use]
    pub fn with_interval_and_trigger(interval: Duration, byte_trigger: u64) -> Self {
        Self {
            interval: interval.max(MIN_INTERVAL),
            byte_trigger: byte_trigger.max(MIN_BYTE_TRIGGER),
            last_emit: None,
            last_emit_bytes: 0,
        }
    }

    /// Update the throttling interval while preserving prior emission state.
    pub fn set_interval(&mut self, interval: Duration, byte_trigger: u64) {
        self.interval = interval.max(MIN_INTERVAL);
        self.byte_trigger = byte_trigger.max(MIN_BYTE_TRIGGER);
    }

    /// Consider emitting a snapshot using the current traversal counters.
    pub fn consider(
        &mut self,
        now: Instant,
        processed_bytes: u64,
        processed_entries: u64,
        timestamp_ms: u64,
    ) -> Option<ProgressSnapshot> {
        if self.last_emit.is_none() {
            self.last_emit = Some(now);
            self.last_emit_bytes = processed_bytes;
            return None;
        }

        let last_emit = self.last_emit.unwrap();
        let elapsed = now.saturating_duration_since(last_emit);
        let bytes_delta = processed_bytes.saturating_sub(self.last_emit_bytes);

        let time_ready = elapsed >= self.interval;
        let byte_gate = std::cmp::max(self.interval / 2, MIN_INTERVAL);
        let bytes_ready = self.byte_trigger != u64::MAX
            && elapsed >= byte_gate
            && bytes_delta >= self.byte_trigger;

        if time_ready || bytes_ready {
            let throughput = compute_throughput(bytes_delta, elapsed);

            self.last_emit = Some(now);
            self.last_emit_bytes = processed_bytes;

            return Some(ProgressSnapshot {
                timestamp_ms,
                processed_entries,
                processed_bytes,
                estimated_completion_ratio: None,
                recent_throughput_bytes_per_sec: throughput,
            });
        }

        None
    }

    /// Emit a final snapshot regardless of thresholds.
    pub fn force_emit(
        &mut self,
        now: Instant,
        processed_bytes: u64,
        processed_entries: u64,
        timestamp_ms: u64,
    ) -> Option<ProgressSnapshot> {
        let throughput = self.estimate_throughput(now, processed_bytes);

        self.last_emit = Some(now);
        self.last_emit_bytes = processed_bytes;

        Some(ProgressSnapshot {
            timestamp_ms,
            processed_entries,
            processed_bytes,
            estimated_completion_ratio: Some(1.0_f32),
            recent_throughput_bytes_per_sec: throughput,
        })
    }

    fn estimate_throughput(&self, now: Instant, processed_bytes: u64) -> Option<u64> {
        let last_emit = self.last_emit?;
        let elapsed = now.saturating_duration_since(last_emit);
        let bytes_delta = processed_bytes.saturating_sub(self.last_emit_bytes);
        compute_throughput(bytes_delta, elapsed)
    }
}

fn compute_throughput(bytes_delta: u64, elapsed: Duration) -> Option<u64> {
    let nanos = elapsed.as_nanos();
    if nanos == 0 {
        return None;
    }

    let numerator = u128::from(bytes_delta) * 1_000_000_000u128;
    let rate = numerator / nanos;
    u64::try_from(rate.min(u128::from(u64::MAX))).ok()
}
