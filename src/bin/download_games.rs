use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use rand::Rng;
use rayon::prelude::*;
use rusqlite::{Connection, params, OptionalExtension};
use rbc::api;

fn build_dict(game_ids: &[i32]) -> Vec<u8> {
    let pb = Arc::new(Mutex::new(pbr::ProgressBar::new(game_ids.len() as u64)));
    pb.lock().unwrap().message("samples for zstd dict  ");
    let samples = game_ids.par_iter().filter_map(|&game_id| {
        pb.lock().unwrap().inc();
        match api::game_history_raw(game_id) {
            Ok(h) => Some(h),
            Err(api::Error::HttpError(400)) |
            Err(api::Error::HttpError(404)) => None,
            e => panic!("{:?}", e),
        }
    });
    let samples: Vec<_> = samples.collect();
    pb.lock().unwrap().finish();
    println!();
    dbg!(samples.len());

    let dict = zstd::dict::from_samples(&samples, 1024 * 1024).unwrap();
    dbg!(dict.len());
    dict
}

fn main() {
    env_logger::init();
    rayon::ThreadPoolBuilder::new().num_threads(20).build_global().unwrap();

    let max_game_id = 18199;

    let mut conn = Connection::open("game_log.db").unwrap();
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
        winner_color TEXT,  -- 'w' or 'b'
        win_reason TEXT,
        num_moves INTEGER,
        dict_id INTEGER,
        data BLOB,

        time_created INTEGER,
        time_finished INTEGER,

        FOREIGN KEY (dict_id) REFERENCES dictionary
    );
    ").unwrap();

    let mut q = conn.prepare("SELECT id, data FROM dictionary LIMIT 1").unwrap();
    let r = q.query_row(params![], |row| {
        let id: i64 = row.get(0)?;
        let data: Vec<u8> = row.get(1)?;
        Ok((id, data))
    }).optional().unwrap();
    drop(q);
    let (dict_id, dict) = r.unwrap_or_else(|| {
        let mut rng = rand::thread_rng();
        let game_ids = (1..max_game_id).filter(|_| { rng.gen_bool(0.05)});
        let game_ids: Vec<_> = game_ids.collect();
        let dict = build_dict(&game_ids);

        let mut q = conn.prepare("INSERT INTO dictionary(data) VALUES (?) ").unwrap();
        let dict_id = q.insert(params![&dict]).unwrap();
        (dict_id, dict)
    });
    dbg!((dict_id, dict.len()));

    let game_ids = 12000..18000;
    let pb = Arc::new(Mutex::new(pbr::ProgressBar::new(game_ids.len() as u64)));
    pb.lock().unwrap().message("games to download  ");

    let rows = game_ids.into_par_iter().filter_map(|game_id| {
        pb.lock().unwrap().inc();
        match api::game_history_raw(game_id) {
            Ok(h) => {
                let mut enc = zstd::Encoder::with_dictionary(Vec::<u8>::new(), 21, &dict).unwrap();
                enc.write_all(h.as_bytes()).unwrap();
                let zd = enc.finish().unwrap();

                let mut dec = zstd::Decoder::with_dictionary(std::io::BufReader::new(&zd[..]), &dict).unwrap();
                let mut h_roundtrip = String::new();
                dec.read_to_string(&mut h_roundtrip).unwrap();
                assert_eq!(h, h_roundtrip);

                let h: api::GameHistoryResponse = serde_json::from_str(&h).unwrap_or_else(|e| {
                    dbg!(game_id);
                    dbg!(e);
                    panic!()
                });
                let h = api::GameHistory::from(h.game_history);
                Some((
                    game_id,
                    h.white_name,
                    h.black_name,
                    h.winner_color,
                    h.win_reason,
                    h.moves.len(),
                    zd,
                ))
            }
            Err(api::Error::HttpError(400)) |
            Err(api::Error::HttpError(404)) => None,
            e => { dbg!(&e); e.unwrap(); unreachable!(); }
        }
    });
    let rows: Vec<_> = rows.collect();
    pb.lock().unwrap().finish();
    println!();

    let mut pb = pbr::ProgressBar::new(rows.len() as u64);
    pb.message("games to save to db  ");
    let t = conn.transaction().unwrap();
    let mut q = t.prepare("
    INSERT OR IGNORE INTO game(
        game_id,
        white_name, black_name,
        winner_color, win_reason, num_moves,
        dict_id, data)
    VALUES (?,  ?, ?,  ?, ?, ?,  ?, ?)").unwrap();
    for row in rows {
        let (game_id, white_name, black_name, winner_color, win_reason, num_moves, data) = row;
        let winner_color = format!("{:?}", winner_color);
        let res = q.insert(params![
            game_id,
            white_name, black_name,
            winner_color, win_reason, num_moves as i32,
            dict_id, data,
        ]);
        match res {
            Ok(_) => {}
            Err(rusqlite::Error::StatementChangedRows(0)) => {},
            err => { err.unwrap(); }
        }
        pb.inc();
    }
    drop(q);
    t.commit().unwrap();
    pb.finish();
    println!();
}
