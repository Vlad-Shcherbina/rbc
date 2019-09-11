use rand::prelude::*;
use crate::game::{STARTING_FEN, Color, Piece, Move, BoardState};

pub trait Ai {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player>;
}

pub trait Player {
    fn handle_opponent_move(&mut self, capture_square: Option<i32>);
    fn choose_sense(&mut self) -> i32;
    fn handle_sense(&mut self, sense: i32, sense_result: &[(i32, Option<Piece>)]);
    fn choose_move(&mut self) -> Option<Move>;
    fn handle_move(&mut self, requested: Option<Move>, taken: Option<Move>, capture_square: Option<i32>);
}

pub struct RandomAi;

impl Ai for RandomAi {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player> {
        let mut state: BoardState = fen::BoardState::from_fen(STARTING_FEN).unwrap().into();
        state.fog_of_war(color);
        Box::new(RandomPlayer {
            rng: StdRng::seed_from_u64(seed),
            color,
            state,
        })
    }
}

struct RandomPlayer {
    rng: StdRng,
    color: Color,
    state: BoardState,
}

impl Player for RandomPlayer {
    fn handle_opponent_move(&mut self, capture_square: Option<i32>) {
        assert!(self.color != self.state.side_to_play);
        self.state.make_move_under_fog(capture_square);
    }

    fn choose_sense(&mut self) -> i32 {
        assert_eq!(self.color, self.state.side_to_play);
        self.rng.gen_range(0, 64)
    }

    fn handle_sense(&mut self, _sense: i32, _sense_result: &[(i32, Option<Piece>)]) {
        assert_eq!(self.color, self.state.side_to_play);
        dbg!(self.state.render());
    }

    fn choose_move(&mut self) -> Option<Move> {
        assert_eq!(self.color, self.state.side_to_play);
        if self.rng.gen_bool(0.5) {
            Some(random_move(&mut self.rng, &self.state))
        } else {
            let all_moves = self.state.all_sensible_requested_moves();
            Some(all_moves[self.rng.gen_range(0, all_moves.len())])
        }
    }

    fn handle_move(&mut self, _requested: Option<Move>, taken: Option<Move>, _capture_square: Option<i32>) {
        assert_eq!(self.color, self.state.side_to_play);
        self.state.make_move(taken);
        self.state.fog_of_war(self.color);
        dbg!(self.state.render());
    }
}

fn random_move(rng: &mut impl RngCore, state: &BoardState) -> Move {
    let mut from: i32;
    loop {
        from = rng.gen_range(0, 64);
        let p = state.get_piece(from);
        if p.is_some() && p.unwrap().color == state.side_to_play {
            break;
        }
    }
    let mut to: i32;
    loop {
        to = rng.gen_range(0, 64);
        let p = state.get_piece(to);
        if p.is_some() && p.unwrap().color == state.side_to_play {
            continue;
        }
        let dr = (from / 8 - to / 8).abs();
        let df = (from % 8 - to % 8).abs();
        if dr == 0 || df == 0 || dr == df || dr + df == 3 {
            break;
        }
    }
    Move {
        from,
        to,
        promotion: None,
    }
}
