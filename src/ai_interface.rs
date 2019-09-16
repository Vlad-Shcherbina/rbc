use rand::prelude::*;
use log::info;
use crate::game::{STARTING_FEN, Square, Color, Piece, Move, BoardState};

pub trait Ai {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player>;
}

pub trait Player {
    fn handle_opponent_move(&mut self, capture_square: Option<Square>);
    fn choose_sense(&mut self) -> Square;
    fn handle_sense(&mut self, sense: Square, sense_result: &[(Square, Option<Piece>)]);
    fn choose_move(&mut self) -> Option<Move>;
    fn handle_move(&mut self, requested: Option<Move>, taken: Option<Move>, capture_square: Option<Square>);
}

pub struct RandomAi {
    pub delay: u64,
}

impl Ai for RandomAi {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player> {
        let mut state: BoardState = fen::BoardState::from_fen(STARTING_FEN).unwrap().into();
        state.fog_of_war(color);
        Box::new(RandomPlayer {
            rng: StdRng::seed_from_u64(seed),
            delay: self.delay,
            color,
            state,
        })
    }
}

struct RandomPlayer {
    rng: StdRng,
    delay: u64,
    color: Color,
    state: BoardState,
}

impl Player for RandomPlayer {
    fn handle_opponent_move(&mut self, capture_square: Option<Square>) {
        assert!(self.color != self.state.side_to_play);
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
        self.state.make_move_under_fog(capture_square);
    }

    fn choose_sense(&mut self) -> Square {
        assert_eq!(self.color, self.state.side_to_play);
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
        Square(self.rng.gen_range(0, 64))
    }

    fn handle_sense(&mut self, _sense: Square, _sense_result: &[(Square, Option<Piece>)]) {
        assert_eq!(self.color, self.state.side_to_play);
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
        info!("after sense: {:#?}", self.state.render());
    }

    fn choose_move(&mut self) -> Option<Move> {
        assert_eq!(self.color, self.state.side_to_play);
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
        if self.rng.gen_bool(0.5) {
            Some(random_move(&mut self.rng, &self.state))
        } else {
            let all_moves = self.state.all_sensible_requested_moves();
            Some(all_moves[self.rng.gen_range(0, all_moves.len())])
        }
    }

    fn handle_move(&mut self, _requested: Option<Move>, taken: Option<Move>, _capture_square: Option<Square>) {
        assert_eq!(self.color, self.state.side_to_play);
        self.state.make_move(taken);
        self.state.fog_of_war(self.color);
        info!("after move: {:#?}", self.state.render());
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
    }
}

fn random_move(rng: &mut impl RngCore, state: &BoardState) -> Move {
    let mut from;
    loop {
        from = Square(rng.gen_range(0, 64));
        let p = state.get_piece(from);
        if p.is_some() && p.unwrap().color == state.side_to_play {
            break;
        }
    }
    let mut to;
    loop {
        to = Square(rng.gen_range(0, 64));
        let p = state.get_piece(to);
        if p.is_some() && p.unwrap().color == state.side_to_play {
            continue;
        }
        let dr = (from.0 / 8 - to.0 / 8).abs();
        let df = (from.0 % 8 - to.0 % 8).abs();
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
