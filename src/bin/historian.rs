use rbc::api;
use rbc::game::{STARTING_FEN, Move};

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
                if let Some(m) = h.moves.first() {
                    assert_eq!(m.fen_before, STARTING_FEN);
                }
                for m in &h.moves {
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
