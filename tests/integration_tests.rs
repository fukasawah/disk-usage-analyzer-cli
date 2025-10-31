// Integration tests entry point

mod fixtures;

mod integration {
    mod test_errors;
    mod test_perf_smoke;
    mod test_resilience;
    mod test_scan;
    mod test_snapshot_errors;
    mod test_snapshot_roundtrip;
    mod test_view_drill_down;
}

mod contract {
    mod test_json_shape;
    mod test_snapshot_json;
}

mod unit {
    mod aggregate_tests;
    mod depth_tests;
    mod normalize_path_tests;
    mod traverse_tests;
}
