#![allow(clippy::unreadable_literal)]

use std::io::Write;
use rand::prelude::*;
use rbc::game::{Square, Color, Move, Piece, BoardState};
use rbc::ai_interface::{Ai, Player};
use rbc::infoset::Infoset;
use rbc::distr;

struct GameState {
    board: BoardState,
    infoset_white: Infoset,
    player_white: Box<dyn Player>,
    infoset_black: Infoset,
    player_black: Box<dyn Player>,
    move_number: i32,
    last_capture: Option<(Square, Piece)>,
}

impl GameState {
    fn new(ai_white: &dyn Ai, ai_black: &dyn Ai, seed: u64) -> GameState {
        GameState {
            board: BoardState::initial(),
            infoset_white: Infoset::new(Color::White),
            player_white: ai_white.make_player(Color::White, seed + 1),
            infoset_black: Infoset::new(Color::Black),
            player_black: ai_black.make_player(Color::Black, seed + 2),
            move_number: 0,
            last_capture: None,
        }
    }

    fn is_over(&self) -> bool {
        self.board.winner().is_some()
    }

    fn phase1(&mut self, html: &mut dyn Write) -> Vec<(Square, f32)> {
        let (infoset, player) = match self.board.side_to_play() {
            Color::White => (&mut self.infoset_white, &mut self.player_white),
            Color::Black => (&mut self.infoset_black, &mut self.player_black),
        };
        if self.move_number > 0 {
            infoset.opponent_move(self.last_capture.map(|c| c.0));
            player.handle_opponent_move(self.last_capture, infoset, html);
        }
        player.choose_sense(infoset, html)
    }

    fn phase2(&mut self, sense: Square, html: &mut dyn Write) -> Vec<(Option<Move>, f32)> {
        let (infoset, player) = match self.board.side_to_play() {
            Color::White => (&mut self.infoset_white, &mut self.player_white),
            Color::Black => (&mut self.infoset_black, &mut self.player_black),
        };
        let sense_result = self.board.sense(sense);
        infoset.sense(sense, &sense_result);
        player.handle_sense(sense, &sense_result, infoset, html);
        player.choose_move(infoset, html)
    }

    fn phase3(&mut self, requested_move: Option<Move>, html: &mut dyn Write) {
        let (infoset, player) = match self.board.side_to_play() {
            Color::White => (&mut self.infoset_white, &mut self.player_white),
            Color::Black => (&mut self.infoset_black, &mut self.player_black),
        };
        {
            let mut fog_state = self.board.clone();
            fog_state.fog_of_war(self.board.side_to_play());
            assert!(fog_state.all_sensible_requested_moves().contains(&requested_move));
        }
        let board = &self.board;
        let taken_move = board.requested_to_taken(requested_move);
        let old_board = self.board.clone();
        self.last_capture = match self.board.make_move(taken_move) {
            Some(cs) => Some((cs, old_board.get_piece(cs).unwrap())),
            None => None,
        };
        let observed_capture = infoset.my_move(requested_move, taken_move, self.last_capture.map(|c| c.0));
        player.handle_move(requested_move, taken_move, observed_capture, infoset, html);

        self.move_number += 1;
    }
}

fn main() {
    rbc::logger::init_changeable_logger(
        rbc::logger::WriteLogger::new(
            std::fs::File::create("logs/self_play.info.txt").unwrap()));
    log::set_max_level(log::LevelFilter::Info);

    let ai1 = rbc::greedy::GreedyAi { experiment: true };
    let ai2 = rbc::greedy::GreedyAi { experiment: false };

    let mut rng = StdRng::seed_from_u64(424242);

    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1] == "bench" {
        let mut html_white = std::io::BufWriter::new(
            std::fs::File::create("logs/self_play_white.html").unwrap());
        writeln!(html_white, "{}", rbc::html::PREAMBLE).unwrap();
        let mut html_black = std::io::sink();

        let timer = std::time::Instant::now();
        let mut game = GameState::new(&ai1, &ai2, 424242);
        while !game.is_over() {
            dbg!(game.move_number);
            let html: &mut dyn Write = match game.board.side_to_play() {
                Color::White => &mut html_white,
                Color::Black => &mut html_black,
            };
            let sense_distr = game.phase1(html);
            let sense = *distr::draw(&sense_distr, &mut rng);
            let requested_distr = game.phase2(sense, html);
            let requested = *distr::draw(&requested_distr, &mut rng);
            game.phase3(requested, html);
        }
        println!("{:?} won", game.board.winner().unwrap());
        println!("{:#?}", game.board.render());
        println!("white summary:\n{}", game.player_white.get_summary());
        println!("black summary:\n{}", game.player_black.get_summary());
        println!("{}", rbc::stats::render());
        println!("it took {:.3}s", timer.elapsed().as_secs_f64());
        return;
    }

    let mut html = std::io::sink();

    use std::collections::HashMap;
    let mut outcome_cnt: HashMap<(Color, Color), i32> = HashMap::new();
    loop {
        let mut game1 = GameState::new(&ai1, &ai2, 424242);
        let mut game2 = GameState::new(&ai2, &ai1, 424242);
        while !game1.is_over() && !game2.is_over() {
            // println!("{} both", game1.move_number);
            let mut sense_distr1 = game1.phase1(&mut html);
            let mut sense_distr2 = game2.phase1(&mut html);
            distr::normalize(&mut sense_distr1);
            distr::normalize(&mut sense_distr2);
            let (&sense1, &sense2) = distr::draw_correlated(&sense_distr1, &sense_distr2, &mut rng);
            // println!("sense: {:?} {:?}", sense1, sense2);
            let mut requested_distr1 = game1.phase2(sense1, &mut html);
            let mut requested_distr2 = game2.phase2(sense2, &mut html);
            distr::normalize(&mut requested_distr1);
            distr::normalize(&mut requested_distr2);
            let (&requested1, &requested2) = distr::draw_correlated(&requested_distr1, &requested_distr2, &mut rng);
            // println!("move: {:?} {:?}", requested1, requested2);
            game1.phase3(requested1, &mut html);
            game2.phase3(requested2, &mut html);
        }
        // println!("---------");
        while !game1.is_over() {
            // println!("{} game1", game1.move_number);
            let sense_distr = game1.phase1(&mut html);
            let sense = *distr::draw(&sense_distr, &mut rng);
            let requested_distr = game1.phase2(sense, &mut html);
            let requested = *distr::draw(&requested_distr, &mut rng);
            game1.phase3(requested, &mut html);
        }
        while !game2.is_over() {
            // println!("{} game2", game2.move_number);
            let sense_distr = game2.phase1(&mut html);
            let sense = *distr::draw(&sense_distr, &mut rng);
            let requested_distr = game2.phase2(sense, &mut html);
            let requested = *distr::draw(&requested_distr, &mut rng);
            game2.phase3(requested, &mut html);
        }
        println!("moves: {} {}", game1.move_number, game2.move_number);
        let outcome = (game1.board.winner().unwrap(), game2.board.winner().unwrap());
        println!("outcome: {:?}", outcome);
        *outcome_cnt.entry(outcome).or_default() += 1;
        println!("{:?}", outcome_cnt);
    }
}
