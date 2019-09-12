use log::info;
use rand::prelude::*;
use crate::game::{Square, Color, PieceKind, Piece, Move, BoardState};
use crate::ai_interface::{Ai, Player};
use crate::infoset::Infoset;

pub struct GreedyAi;

impl Ai for GreedyAi {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player> {
        let infoset = Infoset::new(color);
        Box::new(GreedyPlayer {
            rng: StdRng::seed_from_u64(seed),
            color,
            infoset,
        })
    }
}

struct GreedyPlayer {
    rng: StdRng,
    color: Color,
    infoset: Infoset,
}

impl Player for GreedyPlayer {
    fn handle_opponent_move(&mut self, capture_square: Option<Square>) {
        assert!(self.color != self.infoset.fog_state.side_to_play);
        info!("opp capture: {:?}", capture_square);
        self.infoset.opponent_move(capture_square);
        info!("{:#?}", self.infoset.render());
    }

    fn choose_sense(&mut self) -> Square {
        assert_eq!(self.color, self.infoset.fog_state.side_to_play);
        let mut best_sense_rank = -1.0;
        let mut best_sense = Square(0);
        for rank in (1..7).rev() {
            let mut line = String::new();
            for file in 1..7 {
                let sq = Square(rank * 8 + file);
                let e = self.infoset.sense_entropy(sq);
                line.push_str(&format!("{:>7.2}", e));
                if e > best_sense_rank {
                    best_sense_rank = e;
                    best_sense = sq;
                }
            }
            info!("entropy: {}", line)
        }
        info!("best sense: {:?} {:.3}", best_sense, best_sense_rank);
        best_sense
    }

    fn handle_sense(&mut self, sense: Square, sense_result: &[(Square, Option<Piece>)]) {
        assert_eq!(self.color, self.infoset.fog_state.side_to_play);
        info!("sense {:?} -> {:?}", sense, sense_result);
        self.infoset.sense(sense, sense_result);
        dbg!(self.infoset.render());
    }

    fn choose_move(&mut self) -> Option<Move> {
        assert_eq!(self.color, self.infoset.fog_state.side_to_play);
        let mut best_score = -10000;
        let mut best_move = None;
        for requested in self.infoset.fog_state.all_sensible_requested_moves() {
            let mut score = 0;
            for s in &self.infoset.possible_states {
                let taken = s.requested_to_taken(requested);
                let mut s2 = s.clone();
                let cs = s2.make_move(taken);
                if let Some(cs) = cs {
                    score += 20 * material_value(s.get_piece(cs).unwrap().kind);
                }
                s2.make_move(None);
                assert_eq!(s2.side_to_play, self.color);
                score += s2.all_moves().len() as i64;
                score -= 2 * material_value(s.get_piece(requested.from).unwrap().kind);
            }
            if score > best_score {
                best_score = score;
                best_move = Some(requested);
            }
        }
        best_move
    }

    fn handle_move(&mut self, requested: Option<Move>, taken: Option<Move>, capture_square: Option<Square>) {
        assert_eq!(self.color, self.infoset.fog_state.side_to_play);
        info!("requested move: {:?}", requested);
        info!("taken move :    {:?}", taken);
        info!("capture square: {:?}", capture_square);
        self.infoset.my_move(requested, taken, capture_square);
        info!("{:#?}", self.infoset.render());
    }
}

fn material_value(k: PieceKind) -> i64 {
    match k {
        PieceKind::Pawn => 1,
        PieceKind::Knight => 3,
        PieceKind::Bishop => 3,
        PieceKind::Rook => 6,
        PieceKind::Queen => 9,
        PieceKind::King => 20,
    }
}
