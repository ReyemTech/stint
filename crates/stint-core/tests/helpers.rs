use stint_core::{ids, time};

#[test]
fn new_local_uuid_is_a_valid_uuid_v4_string() {
    let id = ids::new_local_uuid();
    assert_eq!(id.len(), 36);
    assert_eq!(id.chars().nth(14), Some('4')); // version nibble
}

#[test]
fn now_utc_is_rfc3339_with_z_suffix() {
    let s = time::now_utc();
    assert!(s.ends_with('Z'), "expected Z suffix, got {s}");
    // parse round-trip
    let parsed = time::parse(&s).unwrap();
    assert_eq!(time::format(&parsed), s);
}
