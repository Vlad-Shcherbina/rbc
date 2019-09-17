use log::{info, error};
use rusqlite::{Connection, params};
use rbc::history::GameHistory;
use rbc::game::{Square, Color, Move, BoardState};

fn replay(h: &GameHistory, color: Color) -> usize {
    // TODO: switch to GreedyAi
    let mut max_size = 0;

    let mut move_number = match color {
        Color::White => 0,
        Color::Black => 1,
    };

    let mut infoset = rbc::infoset::Infoset::new(color);

    while move_number < h.moves.len() {
        info!("move number {}", move_number);
        if move_number > 0 {
            info!("opp capture: {:?}", h.moves[move_number - 1].capture_square);
            infoset.opponent_move(h.moves[move_number - 1].capture_square);
            info!("{:#?}", infoset.render());
        }
        println!("{} possible states", infoset.possible_states.len());
        max_size = max_size.max(infoset.possible_states.len());
        let mut best_sense_rank = -1.0;
        let mut best_sense = Square(0);
        for rank in (1..7).rev() {
            let mut line = String::new();
            for file in 1..7 {
                let sq = Square(rank * 8 + file);
                let e = infoset.sense_entropy(sq);
                line.push_str(&format!("{:>7.2}", e));
                if e > best_sense_rank {
                    best_sense_rank = e;
                    best_sense = sq;
                }
            }
            info!("entropy: {}", line)
        }
        info!("best sense: {:?} {:.3}", best_sense, best_sense_rank);
        let actual_state: BoardState = fen::BoardState::from_fen(&h.moves[move_number].fen_before).unwrap().into();
        let sense_result = actual_state.sense(best_sense);
        info!("best sense result: {:?}", sense_result);
        infoset.sense(best_sense, &sense_result);
        info!("{:#?}", infoset.render());

        /*if let Some(sense) = h.moves[move_number].sense {
            info!("sense {:?} -> {:?}", sense, h.moves[move_number].sense_result);
            infoset.sense(sense, &h.moves[move_number].sense_result);
            info!("{:#?}", infoset.render());
        }*/

        let requested = h.moves[move_number].requested_move.as_ref().map(|s| Move::from_uci(s));
        let taken = h.moves[move_number].taken_move.as_ref().map(|s| Move::from_uci(s));
        let capture_square = h.moves[move_number].capture_square;

        info!("requested move: {:?}", requested);
        info!("taken move :    {:?}", taken);
        info!("capture square: {:?}", capture_square);
        infoset.my_move(requested, taken, capture_square);
        info!("{:#?}", infoset.render());

        move_number += 2;
    }
    max_size
}

fn main() {
    // env_logger::init();
    let logger = rbc::logger::init_changeable_logger(rbc::logger::SimpleLogger);
    log::set_max_level(log::LevelFilter::Info);

    let mut max_size = 0;

    let conn = Connection::open("game_log.db").unwrap();
    let dicts = rbc::history_db::get_dicts(&conn);
    let filter = "";
    conn.prepare(&format!("
        SELECT game_id, dict_id, data
        FROM game {} ORDER BY game_id DESC", filter)).unwrap()
    .query_map(params![], |row| rbc::history_db::game_query_map_fn(&dicts, row))
    .unwrap()
    .filter_map(Result::unwrap)
    .for_each(|(game_id, h)| {
        info!("{}", game_id);
        let (lg, res) = logger.capture_log(|| {
            std::panic::catch_unwind(|| {
                replay(&h, Color::White).max(
                replay(&h, Color::Black))
            })
        });
        match res {
            Ok(ms) => {
                if ms > max_size {
                    max_size = ms;
                    info!("max size: {}", max_size);
                }
            }
            Err(_) => {
                error!("game_id = {}", game_id);
                error!("-- 8< -- inner log --------");
                error!("...");
                let start = if lg.len() < 1000 { 0 } else { lg.len() - 1000 };
                error!("{}", &lg[start..]);
                error!("-------- inner log -- >8 --");
                std::process::exit(1);
            }
        }
    });
}
