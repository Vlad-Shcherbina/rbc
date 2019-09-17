use log::info;
use rand::Rng;
use crate::game::{Color, Move};
use crate::api;
use crate::ai_interface::Ai;

pub fn play_game(color: Color, game_id: i32, ai: &dyn Ai) -> String {
    let seed = rand::thread_rng().gen();
    info!("player seed: {}", seed);
    let mut player = ai.make_player(color, seed);

    let mut halfmove_number = match color {
        Color::White => 0,
        Color::Black => 1,
    };

    loop {
        let gs = api::game_status(game_id).expect("TODO");
        if gs.is_over {
            break;
        }
        if gs.is_my_turn {
            match api::seconds_left(game_id) {
                Ok(_) => {},
                Err(api::Error::HttpError(400)) => {
                    let gs = api::game_status(game_id).expect("TODO");
                    assert!(gs.is_over);
                    break;
                }
                Err(e) => panic!("{:?}", e),
            }
            let capture_square = match api::opponent_move_results(game_id) {
                Ok(cs) => cs,
                Err(api::Error::HttpError(400)) => {
                    let gs = api::game_status(game_id).expect("TODO");
                    assert!(gs.is_over);
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };

            if halfmove_number > 0 {
                player.handle_opponent_move(capture_square);
            } else {
                assert!(capture_square.is_none());
            }

            let sense = player.choose_sense();
            let sense_result = match api::sense(game_id, sense) {
                Ok(sr) => sr,
                Err(api::Error::HttpError(400)) => {
                    let gs = api::game_status(game_id).expect("TODO");
                    assert!(gs.is_over);
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };
            player.handle_sense(sense, &sense_result);

            let requested = player.choose_move();
            let req_str = requested.map_or("a1a1".to_owned(), |r| r.to_uci());
            let mr = match api::make_move(game_id, req_str) {
                Ok(mr) => mr,
                Err(api::Error::HttpError(400)) => {
                    let gs = api::game_status(game_id).expect("TODO");
                    assert!(gs.is_over);
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };
            player.handle_move(
                mr.requested.map(|m| Move::from_uci(&m)),
                mr.taken.map(|m| Move::from_uci(&m)),
                mr.capture_square);

            match api::end_turn(game_id) {
                Ok(()) => {},
                Err(api::Error::HttpError(400)) => {
                    let gs = api::game_status(game_id).expect("TODO");
                    assert!(gs.is_over);
                    break;
                }
                Err(e) => panic!("{:?}", e),
            }
            halfmove_number += 2;
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    let h = api::game_history_raw(game_id).expect("TODO");
    let h: api::GameHistoryResponse = serde_json::from_str(&h).expect("TODO");
    let h = h.game_history;
    let opponent_name = match color {
        Color::White => h.black_name,
        Color::Black => h.white_name,
    };

    let outcome = match h.winner_color {
        None => "draw",
        Some(c) if color == c => "won",
        _ => "lost",
    };
    let message = format!("{}: {} against {} ({})", game_id, outcome, opponent_name, h.win_reason.0);
    info!("summary:\n{}", player.get_summary());
    info!("{}", message);
    message
}
