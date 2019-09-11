use log::info;
use rand::Rng;
use rbc::game::{Color, Move};
use rbc::api;
use rbc::ai_interface::{Ai, RandomAi};

fn play_game(color: Color, game_id: i32, ai: &dyn Ai) -> Result<(), api::Error> {
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
                player.handle_opponent_move(capture_square);
            } else {
                assert!(capture_square.is_none());
            }

            let sense = player.choose_sense();
            let sense_result = api::sense(game_id, sense).expect("TODO");
            player.handle_sense(sense, &sense_result);

            let requested = player.choose_move();
            let mr = api::make_move(game_id, requested.expect("TODO").to_uci()).expect("TODO");
            player.handle_move(
                mr.requested.map(|m| Move::from_uci(&m)),
                mr.taken.map(|m| Move::from_uci(&m)),
                mr.capture_square);

            api::end_turn(game_id).expect("TODO");
            halfmove_number += 2;
        }

        // std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}

fn main() {
    env_logger::init();
    let ai = RandomAi;

    loop {
        let mut opponents = api::list_users().unwrap();
        opponents.retain(|o| o != "DotModus_Chris");  // hangs
        let opponent = rand::thread_rng().gen_range(0, opponents.len());
        let opponent = &opponents[opponent];
        let color: Color = rand::thread_rng().gen_bool(0.5).into();
        let game_id = api::post_invitation(opponent, color).unwrap();
        play_game(color, game_id, &ai).unwrap();
    }
}
