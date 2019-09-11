use std::collections::{HashMap, HashSet};
use crate::game::{STARTING_FEN, Color, Piece, Move, BoardState};

pub struct Infoset {
    pub color: Color,
    pub fog_state: BoardState,
    pub possible_states: Vec<BoardState>,
}

#[inline(never)]
fn deduplicate(xs: &mut Vec<impl Eq + std::hash::Hash>) {
    let seen: HashSet<_> = xs.drain(..).collect();
    *xs = seen.into_iter().collect();
}

impl Infoset {
    #[inline(never)]
    pub fn new(color: Color) -> Infoset {
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
    pub fn opponent_move(&mut self, capture_square: Option<i32>) {
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
    pub fn sense(&mut self, sense: i32, sense_result: &[(i32, Option<Piece>)]) {
        assert_eq!(self.fog_state.side_to_play, self.color);
        for s in &self.possible_states {
            assert_eq!(s.side_to_play, self.color);
        }
        self.possible_states.retain(|state| { state.sense(sense) == sense_result });
    }

    #[inline(never)]
    pub fn sense_entropy(&self, sense: i32) -> f64 {
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
            let p = f64::from(v) / n;
            s -= p.log2() * p;
        }
        s
    }

    #[inline(never)]
    pub fn my_move(&mut self, requested_move: Option<Move>, taken_move: Option<Move>, capture_square: Option<i32>) {
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
    pub fn render(&self) -> Vec<String> {
        let mut piece_sets = vec![0u16; 64];
        for s in &self.possible_states {
            for (i, p) in piece_sets.iter_mut().enumerate() {
                *p |= 1u16 << Piece::to_int(s.get_piece(i as i32));
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
