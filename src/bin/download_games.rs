use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use rand::Rng;
use rayon::prelude::*;
use rusqlite::{Connection, params};
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

    let mut conn = Connection::open("game_log.db").unwrap();
    rbc::history_db::init_tables(&conn);

    let mut dicts = rbc::history_db::get_dicts(&conn);
    if dicts.is_empty() {
        let mut rng = rand::thread_rng();
        let game_ids = (1..18199).filter(|_| { rng.gen_bool(0.05)});
        let game_ids: Vec<_> = game_ids.collect();
        let dict = build_dict(&game_ids);

        let mut q = conn.prepare("INSERT INTO dictionary(data) VALUES (?) ").unwrap();
        let dict_id = q.insert(params![&dict]).unwrap();
        dicts.insert(dict_id, dict);
    }
    let (&dict_id, dict) = dicts.iter().next().unwrap();

    let game_ids = 18400..18500;
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
                let h = h.game_history;
                let num_moves = h.taken_moves["true"].len() + h.taken_moves["false"].len();
                Some((
                    game_id,
                    h.white_name,
                    h.black_name,
                    h.winner_color,
                    h.win_reason,
                    num_moves,
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
        let winner_color = winner_color.map(|c| match c {
            rbc::game::Color::White => "White",
            rbc::game::Color::Black => "Black",
        });
        let res = q.insert(params![
            game_id,
            white_name, black_name,
            winner_color, win_reason.0, num_moves as i32,
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
