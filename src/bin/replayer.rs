use std::collections::{HashMap, HashSet};
use log::{info, error};
use rusqlite::{Connection, params};
use rbc::history::GameHistory;
use rbc::game::{STARTING_FEN, Color, Piece, Move, BoardState, square_to_uci};

struct Infoset {
    color: Color,
    fog_state: BoardState,
    possible_states: Vec<BoardState>,
}

#[inline(never)]
fn deduplicate(xs: &mut Vec<impl Eq + std::hash::Hash>) {
    let seen: HashSet<_> = xs.drain(..).collect();
    *xs = seen.into_iter().collect();
}

impl Infoset {
    #[inline(never)]
    fn new(color: Color) -> Infoset {
        let start_state: BoardState = fen::BoardState::from_fen(STARTING_FEN).unwrap().into();
        let mut fog_state = start_state.clone();
        fog_state.fog_of_war(color);
        Infoset {
            color,
            fog_state,
            possible_states: vec![start_state]
        }
    }

    #[inline(never)]
    fn opponent_move(&mut self, capture_square: Option<i32>) {
        assert!(self.fog_state.side_to_play != self.color);
        for s in &self.possible_states {
            assert!(s.side_to_play != self.color);
        }

        let mut new_possible_states = Vec::new();
        for state in &self.possible_states {
            let mut all_moves = vec![None];
            for m in state.all_moves() {
                all_moves.push(Some(m));
            }
            for m in all_moves {
                let mut new_state = state.clone();
                let c = new_state.make_move(m);
                if c == capture_square {
                    new_possible_states.push(new_state);
                }
            }
        }
        self.possible_states = new_possible_states;
        deduplicate(&mut self.possible_states);
        self.fog_state.make_move_under_fog(capture_square);
    }

    #[inline(never)]
    fn sense(&mut self, sense: i32, sense_result: &[(i32, Option<Piece>)]) {
        assert_eq!(self.fog_state.side_to_play, self.color);
        for s in &self.possible_states {
            assert_eq!(s.side_to_play, self.color);
        }
        self.possible_states.retain(|state| { state.sense(sense) == sense_result });
    }

    #[inline(never)]
    fn sense_entropy(&self, sense: i32) -> f64 {
        let rank = sense / 8;
        let file = sense % 8;
        assert!(1 <= rank && rank < 7);
        assert!(1 <= file && file < 7);
        let mut cnt = HashMap::<u32, i32>::new();
        for s in &self.possible_states {
            let mut fingerprint = 0u32;
            for r in rank-1..=rank+1 {
                for f in file-1..=file+1 {
                    let sq = r * 8 + f;
                    fingerprint *= 7;
                    fingerprint += s.get_piece(sq).map_or(0, |p| p.kind.to_int());
                }
            }
            *cnt.entry(fingerprint).or_default() += 1;
        }
        let mut s = 0.0;
        let n = self.possible_states.len() as f64;
        for &v in cnt.values() {
            let p = v as f64 / n;
            s -= p.log2() * p;
        }
        s
    }

    #[inline(never)]
    fn my_move(&mut self, requested_move: Option<Move>, taken_move: Option<Move>, capture_square: Option<i32>) {
        assert_eq!(self.fog_state.side_to_play, self.color);
        for s in &self.possible_states {
            assert_eq!(s.side_to_play, self.color);
        }

        let mut new_possible_states = Vec::new();
        for mut state in self.possible_states.drain(..) {
            let t = requested_move.map(|m| state.requested_to_taken(m))
                .and_then(std::convert::identity);  // flatten
            if t == taken_move {
                let c = state.make_move(t);
                if c == capture_square {
                    new_possible_states.push(state);
                }
            }
        }
        self.possible_states = new_possible_states;
        deduplicate(&mut self.possible_states);  // TODO: is it necessary?
        self.fog_state.make_move(taken_move);
    }

    #[inline(never)]
    fn render(&self) -> Vec<String> {
        let mut piece_sets = vec![0u16; 64];
        for s in &self.possible_states {
            for i in 0..64 {
                piece_sets[i] |= 1u16 << Piece::to_int(s.get_piece(i as i32));
            }
        }
        let mut result = Vec::new();
        for rank in (0..8).rev() {
            let mut line = format!("{}  ", rank + 1);
            for file in 0..8 {
                let mut ps: String = (0..13)
                    .filter(|i| piece_sets[rank * 8 + file] & (1u16 << i) != 0)
                    .map(|i| Piece::from_int(i).map_or('.', Piece::to_char))
                    .collect();
                if ps.len() == 7 {
                    ps = "???".to_string();
                }
                let d = 6 - ps.len();
                for _ in 0..d/2 {
                    ps.insert(0, ' ');
                }
                for _ in 0..(d + 1)/2 {
                    ps.push(' ');
                }
                line.push(' ');
                line.push_str(&ps);
            }
            result.push(line);
            if rank > 0 {
                result.push(String::new());
            }
        }
        result.push(format!("{} possibilities", self.possible_states.len()));
        result.append(&mut self.fog_state.render());
        result
    }
}

fn replay(h: &GameHistory, color: Color) -> usize {
    let mut max_size = 0;

    let mut move_number = match color {
        Color::White => 0,
        Color::Black => 1,
    };

    let mut infoset = Infoset::new(color);

    while move_number < h.moves.len() {
        info!("move number {}", move_number);
        if move_number > 0 {
            info!("opp capture: {:?}", h.moves[move_number - 1].capture_square);
            infoset.opponent_move(h.moves[move_number - 1].capture_square);
            info!("{:#?}", infoset.render());
        }
        max_size = max_size.max(infoset.possible_states.len());
        let mut best_sense_rank = -1.0;
        let mut best_sense = -1;
        for rank in (1..7).rev() {
            let mut line = String::new();
            for file in 1..7 {
                let sq = rank * 8 + file;
                let e = infoset.sense_entropy(sq);
                line.push_str(&format!("{:>7.2}", e));
                if e > best_sense_rank {
                    best_sense_rank = e;
                    best_sense = sq;
                }
            }
            info!("entropy: {}", line)
        }
        info!("best sense: {} {:.3}", square_to_uci(best_sense), best_sense_rank);
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
