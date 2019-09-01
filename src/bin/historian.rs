use rbc::api;
use rbc::game::{STARTING_FEN, Color, BoardState, Move};

fn main() {
    env_logger::init();

    let mut game_id = 18198;

    loop {
        match api::game_history(game_id) {
            Ok(h) => {
                let winner_color = match h.winner_color {
                    Some(Color::White) => "White",
                    Some(Color::Black) => "Black",
                    None => "-",
                };
                println!(
                    "{}: {:>25} - {:25} {:5} {:12}  {} moves",
                    game_id,
                    h.white_name, h.black_name, winner_color, h.win_reason, h.moves.len());
                for (i, m) in h.moves.iter().enumerate() {
                    if i == 0 {
                        assert_eq!(m.fen_before, STARTING_FEN);
                    } else {
                        assert_eq!(m.fen_before, h.moves[i - 1].fen_after);
                    }
                    let before: BoardState = fen::BoardState::from_fen(&m.fen_before).unwrap().into();
                    let after: BoardState = fen::BoardState::from_fen(&m.fen_after).unwrap().into();
                    if let Some(q) = &m.requested_move {
                        Move::from_uci(q);
                    }
                    let mut state = before;
                    dbg!(&m.taken_move);
                    let m = m.taken_move.as_ref().map(|s| Move::from_uci(s));
                    state.make_move(m);
                    state.en_passant_square = after.en_passant_square;  // TODO
                    assert_eq!(state, after);
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
