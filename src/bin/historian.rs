use std::io::Read;
use rusqlite::{Connection, params};
use rbc::api;
use rbc::game::{STARTING_FEN, BoardState, Move};

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

    conn.prepare("
        SELECT game_id, dict_id, data
        FROM game
        --WHERE game_id > 15900
        LIMIT 100000").unwrap()
    .query_map(params![], |row| {
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


        if game_id % 10 == 0 {
            println!("{}    {} {}", game_id, data.len(), h.len());
        }

        let h: api::GameHistoryResponse = serde_json::from_str(&h).unwrap();
        let h: api::GameHistory = h.game_history.into();
        // dbg!(game_id);
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
            // dbg!(m.taken_move.as_ref().unwrap_or(&"--".into()));
            let m = m.taken_move.as_ref().map(|s| Move::from_uci(s));
            state.make_move(m);
            state.en_passant_square = after.en_passant_square;  // TODO
            assert_eq!(state, after);
        }

        Ok(())
    }).unwrap()
    .collect::<Result<(), _>>().unwrap();
}
