use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_unix_time() -> u64 {
    let t = SystemTime::now();
    t.duration_since(UNIX_EPOCH).unwrap().as_secs() as u64
}

pub fn get_seconds_before_midnight(ts: u64) -> u64 {
    86400 - ts % 86400
}


#[test]
fn test_util() {
    assert_eq!(get_seconds_before_midnight(1651503600), 9 * 60 * 60);
}
