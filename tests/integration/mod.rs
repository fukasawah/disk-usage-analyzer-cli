//! Integration tests module

#[cfg(test)]
mod test_scan;

#[cfg(test)]
mod test_errors;

#[cfg(test)]
mod test_view_drill_down;

#[cfg(test)]
mod test_snapshot_roundtrip;

#[cfg(test)]
mod test_snapshot_errors;

#[cfg(test)]
mod test_perf_smoke;

#[cfg(test)]
mod test_resilience;
