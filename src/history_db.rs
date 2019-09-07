use std::io::Read;
use std::collections::HashMap;
use rusqlite::{Connection, params};
use crate::history::GameHistory;

pub fn init_tables(conn: &Connection) {
    conn.execute_batch("
    CREATE TABLE IF NOT EXISTS
    dictionary (
        id INTEGER PRIMARY KEY,
        data BLOB NOT NULL
    );
    CREATE TABLE IF NOT EXISTS
    game (
        game_id INTEGER PRIMARY KEY,
        white_name TEXT,
        black_name TEXT,
        winner_color TEXT,  -- 'White', 'Black', NULL for draw
        win_reason TEXT,
        num_moves INTEGER,
        dict_id INTEGER,
        data BLOB,

        time_created INTEGER,
        time_finished INTEGER,

        FOREIGN KEY (dict_id) REFERENCES dictionary
    );
    ").unwrap();
}

pub fn get_dicts(conn: &Connection) -> HashMap<i64, Vec<u8>> {
    let dicts: Result<HashMap<_, _>, _> =
        conn.prepare("SELECT id, data FROM dictionary").unwrap()
        .query_map(params![], |row| {
            let id: i64 = row.get(0)?;
            let data: Vec<u8> = row.get(1)?;
            Ok((id, data))
        }).unwrap().collect();
    dicts.unwrap()
}

pub fn game_query_map_fn<S: std::hash::BuildHasher>(
    dicts: &HashMap<i64, Vec<u8>, S>,
    row: &rusqlite::Row,
) -> rusqlite::Result<Option<(i32, GameHistory)>> {
    let game_id: i32 = row.get(0)?;
    let dict_id: i64 = row.get(1)?;
    let data: &[u8] = row.get_raw(2).as_blob().unwrap();
    if [15804, 15931, 15330, 15823, 15829].contains(&game_id) {
        // TODO
        log::warn!("skipping anomaly {}", game_id);
        return Ok(None);
    }
    let mut dec = zstd::Decoder::with_dictionary(
        std::io::BufReader::new(data),
        &dicts[&dict_id]).unwrap();
    let mut h = String::new();
    dec.read_to_string(&mut h).unwrap();

    let h: crate::api::GameHistoryResponse = serde_json::from_str(&h).unwrap();
    let h: GameHistory = h.game_history.into();

    Ok(Some((game_id, h)))
}
