use log::info;
use rand::Rng;
use rbc::game::{STARTING_FEN, Color, Move, BoardState};
use rbc::api;

fn play_game(color: Color, game_id: i32) -> Result<(), api::Error> {
    let mut halfmove_number = match color {
        Color::White => 0,
        Color::Black => 1,
    };

    let mut state: BoardState = fen::BoardState::from_fen(STARTING_FEN).unwrap().into();
    state.fog_of_war(color);

    loop {
        let gs = api::game_status(game_id).expect("TODO");
        if gs.is_over {
            let winner = api::winner_color(game_id).expect("TODO");
            let win_reason = api::win_reason(game_id).expect("TODO");
            if color == winner {
                info!("I won game {} ({})", game_id, win_reason);
            } else {
                info!("I lost game {} ({})", game_id, win_reason);
            }
            break;
        }
        if gs.is_my_turn {
            // api::seconds_left(game_id).unwrap();
            let capture_square = api::opponent_move_results(game_id)?;

            if halfmove_number > 0 {
                state.make_move_under_fog(capture_square);
            } else {
                assert!(capture_square.is_none());
            }
            dbg!(state.render());

            api::sense(game_id, 0).expect("TODO");
            let mut from: i32;
            loop {
                from = rand::thread_rng().gen_range(0, 64);
                let p = state.pieces.0[from as usize];
                if p.is_some() && p.unwrap().color == color {
                    break;
                }
            }
            let mut to: i32;
            loop {
                to = rand::thread_rng().gen_range(0, 64);
                let p = state.pieces.0[to as usize];
                if p.is_some() && p.unwrap().color == color {
                    continue;
                }
                let dr = (from / 8 - to / 8).abs();
                let df = (from % 8 - to % 8).abs();
                if dr == 0 || df == 0 || dr == df || dr + df == 3 {
                    break;
                }
            }
            let my_move = Move {
                from,
                to,
                promotion: None,
            };
            let mr = api::make_move(game_id, my_move.to_uci()).expect("TODO");
            state.make_move(mr.taken.map(|m| Move::from_uci(&m)));
            state.fog_of_war(color);
            api::end_turn(game_id).expect("TODO");

            dbg!(state.render());

            halfmove_number += 2;
        }

        // std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}

fn main() {
    env_logger::init();
    let mut rng = rand::thread_rng();

    api::list_users().unwrap();

    let color: Color = rng.gen_bool(0.5).into();

    let game_id = api::post_invitation("random", color).unwrap();
    play_game(color, game_id).unwrap();
}