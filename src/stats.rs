use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref STATS: Mutex<HashMap<(&'static str, Option<i32>), i64>> =
        Mutex::new(HashMap::new());
}

pub fn inc(name: &'static str, slot: Option<i32>, delta: i64) {
    *STATS.lock().unwrap().entry((name, slot)).or_default() += delta;
}

pub fn render() -> String {
    let mut result = String::new();
    let g = STATS.lock().unwrap();
    let mut kvs: Vec<_> = g.iter().collect();
    kvs.sort();
    for (k, v) in kvs {
        result.push_str(&format!("{:>9} {}", v, k.0));
        if let Some(slot) = k.1 {
            result.push_str(&format!("[{}]", slot));
        }
        result.push('\n');
    }
    result
}
