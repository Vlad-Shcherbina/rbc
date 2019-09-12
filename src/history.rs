use crate::api::RawGameHistory;
use crate::game::{Square, Color, Piece};

#[derive(Debug)]
pub struct MoveHistory {
    pub sense: Option<Square>,
    pub sense_result: Vec<(Square, Option<Piece>)>,
    pub requested_move: Option<String>,
    pub taken_move: Option<String>,
    pub capture_square: Option<Square>,
    pub fen_before: String,
    pub fen_after: String,
}

#[derive(Debug)]
pub struct GameHistory {
    pub white_name: String,
    pub black_name: String,
    pub winner_color: Option<Color>,
    pub win_reason: String,
    pub moves: Vec<MoveHistory>,
}

impl From<RawGameHistory> for GameHistory {
    fn from(h: RawGameHistory) -> GameHistory {
        fn eq_or_one_less(x: usize, y: usize) -> bool {
            x == y || x + 1 == y
        }

        let white_moves = h.taken_moves["true"].len();
        // maybe they resigned or timed out after sensing
        assert!(eq_or_one_less(white_moves, h.senses["true"].len()));
        assert!(eq_or_one_less(white_moves, h.sense_results["true"].len()));
        assert_eq!(white_moves, h.requested_moves["true"].len());
        assert_eq!(white_moves, h.taken_moves["true"].len());
        assert_eq!(white_moves, h.capture_squares["true"].len());
        assert_eq!(white_moves, h.fens_before_move["true"].len());
        assert_eq!(white_moves, h.fens_after_move["true"].len());

        let black_moves = h.taken_moves["false"].len();
        // maybe they resigned or timed out after sensing
        assert!(eq_or_one_less(black_moves, h.senses["false"].len()));
        assert!(eq_or_one_less(black_moves, h.sense_results["false"].len()));
        assert_eq!(black_moves, h.requested_moves["false"].len());
        assert_eq!(black_moves, h.taken_moves["false"].len());
        assert_eq!(black_moves, h.capture_squares["false"].len());
        assert_eq!(black_moves, h.fens_before_move["false"].len());
        assert_eq!(black_moves, h.fens_after_move["false"].len());

        assert!(eq_or_one_less(black_moves, white_moves));
        let mut moves = Vec::new();
        for i in 0..white_moves + black_moves {
            let color = if i % 2 == 0 { "true" } else { "false" };
            moves.push(MoveHistory {
                sense: h.senses[color][i / 2],
                sense_result: h.sense_results[color][i / 2].clone(),
                requested_move: h.requested_moves[color][i / 2].clone().map(|m| m.0),
                taken_move: h.taken_moves[color][i / 2].clone().map(|m| m.0),
                capture_square: h.capture_squares[color][i / 2],
                fen_before: h.fens_before_move[color][i / 2].clone(),
                fen_after: h.fens_after_move[color][i / 2].clone(),
            });
        }
        GameHistory {
            white_name: h.white_name,
            black_name: h.black_name,
            winner_color: h.winner_color,
            win_reason: h.win_reason.0,
            moves,
        }
    }
}
