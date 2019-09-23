use std::io::Write;
use rand::prelude::*;
use log::info;
use crate::game::{Square, Color, Piece, Move, BoardState};
use crate::infoset::Infoset;

pub trait Ai {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player>;
}

pub trait Player {
    fn handle_opponent_move(&mut self,
        capture_square: Option<Square>,
        infoset: &Infoset,
        html: &mut dyn Write);
    fn choose_sense(&mut self, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Square, f32)>;
    fn handle_sense(&mut self,
        sense: Square, sense_result: &[(Square, Option<Piece>)],
        infoset: &Infoset,
        html: &mut dyn Write);
    fn choose_move(&mut self, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Option<Move>, f32)>;
    fn handle_move(&mut self,
        requested: Option<Move>, taken: Option<Move>, capture_square: Option<Square>,
        infoset: &Infoset,
        html: &mut dyn Write);
    fn get_summary(&self) -> String;
}

#[derive(Clone)]
pub struct RandomAi {
    pub delay: u64,
}

impl Ai for RandomAi {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player> {
        let mut state = BoardState::initial();
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
    fn handle_opponent_move(&mut self,
        capture_square: Option<Square>,
        _infoset: &Infoset,
        _html: &mut dyn Write,
    ) {
        assert!(self.color != self.state.side_to_play());
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
        self.state.make_move_under_fog(capture_square);
    }

    fn choose_sense(&mut self, _infoset: &Infoset, _html: &mut dyn Write) -> Vec<(Square, f32)> {
        assert_eq!(self.color, self.state.side_to_play());
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
        let mut result = Vec::new();
        for rank in 1..7 {
            for file in 1..7 {
                result.push((Square(rank * 8 + file), 1.0));
            }
        }
        result
    }

    fn handle_sense(&mut self,
        _sense: Square, _sense_result: &[(Square, Option<Piece>)],
        _infoset: &Infoset,
        _html: &mut dyn Write,
    ) {
        assert_eq!(self.color, self.state.side_to_play());
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
        info!("after sense: {:#?}", self.state.render());
    }

    fn choose_move(&mut self, _infoset: &Infoset, _html: &mut dyn Write) -> Vec<(Option<Move>, f32)> {
        assert_eq!(self.color, self.state.side_to_play());
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
        // TODO: with some probability, try arbitrary random moves,
        // not only sensible ones
        self.state.all_sensible_requested_moves()
            .into_iter()
            .map(|m| (Some(m), 1.0))
            .chain(Some((None, 1.0)))
            .collect()
    }

    fn handle_move(&mut self,
        _requested: Option<Move>, taken: Option<Move>, _capture_square: Option<Square>,
        _infoset: &Infoset,
        _html: &mut dyn Write,
    ) {
        assert_eq!(self.color, self.state.side_to_play());
        self.state.make_move(taken);
        self.state.fog_of_war(self.color);
        info!("after move: {:#?}", self.state.render());
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
    }

    fn get_summary(&self) -> String {
        String::new()
    }
}

fn random_move(rng: &mut impl RngCore, state: &BoardState) -> Move {
    let mut from;
    loop {
        from = Square(rng.gen_range(0, 64));
        let p = state.get_piece(from);
        if p.is_some() && p.unwrap().color == state.side_to_play() {
            break;
        }
    }
    let mut to;
    loop {
        to = Square(rng.gen_range(0, 64));
        let p = state.get_piece(to);
        if p.is_some() && p.unwrap().color == state.side_to_play() {
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
