use log::{info, error};
use rusqlite::{Connection, params};
use rbc::history::GameHistory;
use rbc::game::{STARTING_FEN, Color, BoardState, Move, square_to_uci};

#[allow(clippy::cognitive_complexity)]
fn check_game(h: GameHistory, forgiving_en_passant: bool) {
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
        info!("{:#?}", state.render());
        if let Some(ep) = state.en_passant_square {
            info!("en passant square: {}", square_to_uci(ep));
        }
        info!("sense: {:?} -> {:?}", m.sense, m.sense_result);
        match m.sense {
            Some(s) => assert_eq!(m.sense_result, state.sense(s)),
            None => assert!(m.sense_result.is_empty()),
        }
        info!(
            "{:?} {}",
            state.side_to_play,
            m.taken_move.as_ref().map_or("--", String::as_ref),
        );
        let taken_move = m.taken_move.as_ref().map(|s| Move::from_uci(s));
        let capture_square = state.make_move(taken_move);
        assert_eq!(capture_square, m.capture_square);
        if forgiving_en_passant &&
           state.en_passant_square.is_some() &&
           after.en_passant_square.is_none() {
            // old games had en-passant-related bug
            state.en_passant_square = None;
        }
        assert_eq!(state, after);
    }

    for (mut i, &color) in [Color::White, Color::Black].iter().enumerate() {
        info!("{:?} PoV:", color);
        let mut state: BoardState = fen::BoardState::from_fen(STARTING_FEN).unwrap().into();
        state.fog_of_war(color);

        while i < h.moves.len() {
            if i > 0 {
                info!("opp move: {}",
                    h.moves[i - 1].taken_move.as_ref().map_or("--", String::as_ref));
                state.make_move_under_fog(h.moves[i - 1].capture_square);
            }

            let m = &h.moves[i];
            let mut before: BoardState = fen::BoardState::from_fen(&m.fen_before).unwrap().into();
            let mut after: BoardState = fen::BoardState::from_fen(&m.fen_after).unwrap().into();
            let actual_state = before.clone();
            info!("{:#?}", actual_state.render());
            before.fog_of_war(color);
            after.fog_of_war(color);
            assert_eq!(state, before);

            info!("my move (requested): {}",
                m.requested_move.as_ref().map_or("--", String::as_ref));

            let requested = m.requested_move.as_ref().map(|s| Move::from_uci(s));
            if let Some(m) = &requested {
                let all_moves = state.all_sensible_requested_moves();
                assert!(all_moves.contains(m));
            }
            info!("my move (taken):     {}",
                m.taken_move.as_ref().map_or("--", String::as_ref));

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
    let logger = rbc::logger::init_changeable_logger(rbc::logger::SimpleLogger);
    log::set_max_level(log::LevelFilter::Info);

    let conn = Connection::open("game_log.db").unwrap();

    let dicts = rbc::history_db::get_dicts(&conn);

    let filter = "";
    // let filter = "WHERE game_id > 17000";

    let cnt: i64 =
        conn.prepare(&format!("SELECT COUNT(*) FROM game {}", filter)).unwrap()
        .query_row(params![], |row| row.get(0)).unwrap();
    let mut pb = pbr::ProgressBar::new(cnt as u64);
    pb.set_max_refresh_rate(Some(std::time::Duration::from_millis(500)));

    conn.prepare(&format!("
        SELECT game_id, dict_id, data
        FROM game {} ORDER BY game_id DESC", filter)).unwrap()
    .query_map(params![], |row| rbc::history_db::game_query_map_fn(&dicts, row))
    .unwrap()
    .filter_map(Result::unwrap)
    .for_each(|(game_id, h)| {
        pb.inc();

        let forgiving_en_passant = game_id <= 18431;

        let (lg, res) = logger.capture_log(|| {
            std::panic::catch_unwind(|| { check_game(h, forgiving_en_passant); })
        });
        if res.is_err() {
            error!("game_id = {}", game_id);
            error!("-- 8< -- inner log --------");
            error!("...");
            let start = if lg.len() < 1000 { 0 } else { lg.len() - 1000 };
            error!("{}", &lg[start..]);
            error!("-------- inner log -- >8 --");
            std::process::exit(1);
        }
    });
    pb.finish();
    println!();
}
