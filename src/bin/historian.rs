use std::fmt::Write;
use std::io::Read;
use std::sync::Mutex;
use rusqlite::{Connection, params};
use rbc::api;
use rbc::game::{STARTING_FEN, Piece, BoardState, Move, square_to_uci};

fn check_game(h: api::GameHistory, log: &Mutex<String>) {
    for (i, m) in h.moves.iter().enumerate() {
        if i == 0 {
            assert_eq!(m.fen_before, STARTING_FEN);
        } else {
            assert_eq!(m.fen_before, h.moves[i - 1].fen_after);
        }
        let before: BoardState = fen::BoardState::from_fen(&m.fen_before).unwrap().into();
        let after: BoardState = fen::BoardState::from_fen(&m.fen_after).unwrap().into();
        if let Some(q) = &m.requested_move {
            Move::from_uci(q);
        }
        let mut state = before;

        for rank in (0..8).rev() {
            for file in 0..8 {
                write!(log.lock().unwrap(),
                    " {}", state.pieces.0[file + 8 * rank].map_or('.', Piece::to_char)
                ).unwrap();
            }
            writeln!(log.lock().unwrap()).unwrap();
        }

        if let Some(ep) = state.en_passant_square {
            writeln!(log.lock().unwrap(),
                "en passant square: {}", square_to_uci(ep)
            ).unwrap();
        }
        writeln!(log.lock().unwrap(),
            "{:?} {}",
            state.side_to_play,
            m.taken_move.as_ref().map_or("--", String::as_ref),
        ).unwrap();
        let m = m.taken_move.as_ref().map(|s| Move::from_uci(s));
        state.make_move(m);
        if state.en_passant_square.is_some() &&
           after.en_passant_square.is_none() {
            // their bug
            state.en_passant_square = None;
        }
        assert_eq!(state, after);
    }
}

fn main() {
    env_logger::init();

    let conn = Connection::open("game_log.db").unwrap();

    let dicts: Result<std::collections::HashMap<_, _>, _> =
        conn.prepare("SELECT id, data FROM dictionary").unwrap()
        .query_map(params![], |row| {
            let id: i64 = row.get(0)?;
            let data: Vec<u8> = row.get(1)?;
            Ok((id, data))
        }).unwrap().collect();
    let dicts = dicts.unwrap();

    let filter = "";
    // let filter = "WHERE game_id > 17000";

    let cnt: i64 =
        conn.prepare(&format!("SELECT COUNT(*) FROM game {}", filter)).unwrap()
        .query_row(params![], |row| row.get(0)).unwrap();
    let mut pb = pbr::ProgressBar::new(cnt as u64);

    conn.prepare(
        &format!("SELECT game_id, dict_id, data FROM game {}", filter)).unwrap()
    .query_map(params![], |row| {
        pb.inc();
        let game_id: i32 = row.get(0)?;
        let dict_id: i64 = row.get(1)?;
        let data: &[u8] = row.get_raw(2).as_blob().unwrap();
        if game_id == 15804 || game_id == 15931 {
            // TODO
            println!("skipping anomalies {}", game_id);
            return Ok(());
        }

        let mut dec = zstd::Decoder::with_dictionary(
            std::io::BufReader::new(data),
            &dicts[&dict_id]).unwrap();
        let mut h = String::new();
        dec.read_to_string(&mut h).unwrap();

        let h: api::GameHistoryResponse = serde_json::from_str(&h).unwrap();
        let h: api::GameHistory = h.game_history.into();

        let log = Mutex::new(String::new());
        match std::panic::catch_unwind(|| { check_game(h, &log); }) {
            Ok(()) => {},
            Err(_) => {
                dbg!(game_id);
                println!("{}", log.into_inner().unwrap());
                std::process::exit(1);
            }
        }

        Ok(())
    }).unwrap()
    .collect::<Result<(), _>>().unwrap();
    pb.finish();
    println!();
}
