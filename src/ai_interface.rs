use std::io::Write;
use rand::prelude::*;
use log::info;
use crate::game::{Square, Color, Piece, Move, BoardState};
use crate::infoset::Infoset;

pub trait Ai {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player>;
}

pub trait Player {
    fn begin(&mut self, html: &mut dyn Write);
    fn handle_opponent_move(&mut self,
        capture: Option<(Square, Piece)>,
        infoset: &Infoset,
        html: &mut dyn Write);
    fn choose_sense(&mut self, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Square, f32)>;
    fn handle_sense(&mut self,
        sense: Square, sense_result: &[(Square, Option<Piece>)],
        infoset: &Infoset,
        html: &mut dyn Write);
    fn choose_move(&mut self, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Option<Move>, f32)>;
    fn handle_move(&mut self,
        requested: Option<Move>, taken: Option<Move>, capture: Option<(Square, Vec<Piece>)>,
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
    fn begin(&mut self, _html: &mut dyn Write) {}

    fn handle_opponent_move(&mut self,
        capture: Option<(Square, Piece)>,
        _infoset: &Infoset,
        _html: &mut dyn Write,
    ) {
        assert!(self.color != self.state.side_to_play());
        std::thread::sleep(std::time::Duration::from_secs(
            self.rng.gen_range(0, self.delay + 1)));
        self.state.make_move_under_fog(capture.map(|c| c.0));
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
        _requested: Option<Move>, taken: Option<Move>,
        _capture: Option<(Square, Vec<Piece>)>,
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
