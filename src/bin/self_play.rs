use std::io::Write;
use rand::prelude::*;
use rbc::game::{Square, Color, Move, BoardState};
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
    last_capture_square: Option<Square>,
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
            last_capture_square: None,
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
            infoset.opponent_move(self.last_capture_square);
            player.handle_opponent_move(self.last_capture_square, infoset, html);
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
        if let Some(rm) = &requested_move {
            let mut fog_state = self.board.clone();
            fog_state.fog_of_war(self.board.side_to_play());
            assert!(fog_state.all_sensible_requested_moves().contains(rm));
        }
        let board = &self.board;
        let taken_move = requested_move.and_then(|m| board.requested_to_taken(m));
        self.last_capture_square = self.board.make_move(taken_move);
        infoset.my_move(requested_move, taken_move, self.last_capture_square);
        player.handle_move(requested_move, taken_move, self.last_capture_square, infoset, html);

        self.move_number += 1;
    }
}

fn main() {
    dbg!(std::mem::size_of::<BoardState>());
    // let logger = rbc::logger::init_changeable_logger(rbc::logger::SimpleLogger);
    // log::set_max_level(log::LevelFilter::Info);

    let timer = std::time::Instant::now();

    let ai1 = rbc::greedy::GreedyAi { experiment: false };
    let ai2 = rbc::greedy::GreedyAi { experiment: false };

    let mut game = GameState::new(&ai1, &ai2, 424242);

    let mut rng = StdRng::seed_from_u64(424242);
    let mut html = std::io::sink();

    while !game.is_over() {
        dbg!(game.move_number);
        let sense_distr = game.phase1(&mut html);
        let sense = *distr::draw(&sense_distr, &mut rng);
        let requested_distr = game.phase2(sense, &mut html);
        let requested = *distr::draw(&requested_distr, &mut rng);
        game.phase3(requested, &mut html);
    }
    println!("{:?} won", game.board.winner().unwrap());
    println!("{:#?}", game.board.render());
    println!("white summary:\n{}", game.player_white.get_summary());
    println!("black summary:\n{}", game.player_black.get_summary());

    println!("it took {:.3}s", timer.elapsed().as_secs_f64());
}
