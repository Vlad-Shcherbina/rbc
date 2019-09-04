use std::fmt::Write;
use std::io::Read;
use std::sync::Mutex;
use rusqlite::{Connection, params};
use rbc::api;
use rbc::game::{STARTING_FEN, Color, BoardState, Move, square_to_uci};

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
        writeln!(log.lock().unwrap(), "{:#?}", state.render()).unwrap();
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

    for (mut i, &color) in [Color::White, Color::Black].iter().enumerate() {
        writeln!(log.lock().unwrap(), "{:?} PoV:", color).unwrap();
        let mut state: BoardState = fen::BoardState::from_fen(STARTING_FEN).unwrap().into();
        state.fog_of_war(color);

        while i < h.moves.len() {
            if i > 0 {
                writeln!(log.lock().unwrap(),
                    "opp move: {}",
                    h.moves[i - 1].taken_move.as_ref().map_or("--", String::as_ref),
                ).unwrap();
                state.make_move_under_fog(h.moves[i - 1].capture_square);
            }

            let m = &h.moves[i];
            let mut before: BoardState = fen::BoardState::from_fen(&m.fen_before).unwrap().into();
            let mut after: BoardState = fen::BoardState::from_fen(&m.fen_after).unwrap().into();
            let actual_state = before.clone();
            writeln!(log.lock().unwrap(), "{:#?}", actual_state.render()).unwrap();
            before.fog_of_war(color);
            after.fog_of_war(color);
            assert_eq!(state, before);

            writeln!(log.lock().unwrap(),
                "my move (requested): {}",
                m.requested_move.as_ref().map_or("--", String::as_ref),
            ).unwrap();

            let requested = m.requested_move.as_ref().map(|s| Move::from_uci(s));
            if let Some(m) = &requested {
                let all_moves = state.all_sensible_requested_moves();
                assert!(all_moves.contains(m));
            }
            writeln!(log.lock().unwrap(),
                "my move (taken):     {}",
                m.taken_move.as_ref().map_or("--", String::as_ref),
            ).unwrap();

            let taken = m.taken_move.as_ref().map(|s| Move::from_uci(s));

            let predicted_taken =
                requested.map(|m| actual_state.requested_to_taken(m))
                .and_then(std::convert::identity);  // flatten
            assert_eq!(predicted_taken, taken);

            state.make_move(taken);
            state.fog_of_war(color);

            assert_eq!(state, after);

            i += 2;
        }
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
    pb.set_max_refresh_rate(Some(std::time::Duration::from_millis(500)));

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
                let log = log.into_inner().unwrap();
                let start = if log.len() < 1000 { 0 } else { log.len() - 1000 };
                println!("{}", &log[start..]);
                std::process::exit(1);
            }
        }

        Ok(())
    }).unwrap()
    .collect::<Result<(), _>>().unwrap();
    pb.finish();
    println!();
}
