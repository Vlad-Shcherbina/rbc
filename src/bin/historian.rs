use rbc::api;
use rbc::game::{STARTING_FEN, BoardState, Move};

fn main() {
    env_logger::init();

    let mut game_id = 18198;

    loop {
        match api::game_history(game_id) {
            Ok(h) => {
                println!(
                    "{}: {:>25} - {:25} {:?} {:12}  {} moves",
                    game_id,
                    h.white_name, h.black_name, h.winner_color, h.win_reason, h.moves.len());
                for (i, m) in h.moves.iter().enumerate() {
                    if i == 0 {
                        assert_eq!(m.fen_before, STARTING_FEN);
                    } else {
                        assert_eq!(m.fen_before, h.moves[i - 1].fen_after);
                    }
                    let _before: BoardState = fen::BoardState::from_fen(&m.fen_before).unwrap().into();
                    let _after: BoardState = fen::BoardState::from_fen(&m.fen_after).unwrap().into();
                    if let Some(q) = &m.requested_move {
                        Move::from_uci(q);
                    }
                    if let Some(q) = &m.taken_move {
                        Move::from_uci(q);
                    }
                }
            }
            Err(api::Error::HttpError(404)) => {
                println!("{}: game does not exist", game_id);
            }
            Err(api::Error::HttpError(400)) => {
                println!("{}: game is not over", game_id);
            }
            Err(e) => panic!("{:?}", e),
        }
        game_id -= 1;
    }
}
