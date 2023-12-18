use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Utc};

pub struct TarkovTime {
    pub right_time: String,
    pub left_time: String,
}

fn hrs(num: u64) -> u64 {
    60 * 60 * num
}

fn real_time_to_tarkov_time(time: u64, left: bool) -> u64 {
    let tarkov_ratio = 7;
    let one_day = hrs(24);
    let russia = hrs(3);
    let offset = if left { 0 } else { hrs(12) } + russia;

    (offset + (time * tarkov_ratio)) % one_day
}


pub fn calculate_tarkov_time() -> TarkovTime {
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let right_time = real_time_to_tarkov_time(now, false);
    let left_time = real_time_to_tarkov_time(now, true);

    let right_time = UNIX_EPOCH + Duration::from_secs(right_time);
    let left_time = UNIX_EPOCH + Duration::from_secs(left_time);


    return TarkovTime {
        left_time: DateTime::<Utc>::from(left_time).format("%H:%M:%S").to_string(),
        right_time: DateTime::<Utc>::from(right_time).format("%H:%M:%S").to_string(),
    };
}