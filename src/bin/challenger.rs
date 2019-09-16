use log::info;
use rand::Rng;
use rbc::game::{Color, Move};
use rbc::api;
use rbc::ai_interface::Ai;

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

        // std::thread::sleep(std::time::Duration::from_secs(1));
    }

    let winner = api::winner_color(game_id).expect("TODO");
    let win_reason = api::win_reason(game_id).expect("TODO");
    if color == winner {
        info!("I won game {} ({})", game_id, win_reason);
    } else {
        info!("I lost game {} ({})", game_id, win_reason);
    }

    Ok(())
}

fn main() {
    let logger = rbc::logger::init_changeable_logger(rbc::logger::SimpleLogger);
    log::set_max_level(log::LevelFilter::Info);

    // let ai = rbc::ai_interface::RandomAi {
    //     delay: 60,
    // };
    let ai = rbc::greedy::GreedyAi;

    loop {
        let mut opponents = api::list_users().unwrap();
        opponents.retain(|o| o != "DotModus_Chris");  // hangs
        let opponent = rand::thread_rng().gen_range(0, opponents.len());
        let opponent = &opponents[opponent];
        let color: Color = rand::thread_rng().gen_bool(0.5).into();
        let game_id = api::post_invitation(opponent, color).unwrap();

        std::fs::create_dir_all("logs").unwrap();
        let f = std::fs::File::create(format!("logs/game_{:05}.info", game_id)).unwrap();
        logger.with(rbc::logger::WriteLogger::new(f), || {
            info!("challenger playing against {}", opponent);
            play_game(color, game_id, &ai).unwrap();
        });
    }
}
