//! Progress reporting primitives for traversal strategies.

use crate::models::ProgressSnapshot;
use std::time::{Duration, Instant};

const BYTE_TRIGGER: u64 = 1_000_000;
const MIN_INTERVAL: Duration = Duration::from_millis(100);

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
        Self::with_interval(Duration::from_secs(2))
    }

    /// Construct a throttler with the supplied minimum interval.
    #[must_use]
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            interval: interval.max(MIN_INTERVAL),
            byte_trigger: BYTE_TRIGGER,
            last_emit: None,
            last_emit_bytes: 0,
        }
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

        if elapsed >= self.interval || bytes_delta >= self.byte_trigger {
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
