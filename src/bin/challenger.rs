use log::info;
use rand::Rng;
use rbc::game::{Color, Move};
use rbc::api;
use rbc::ai_interface::Ai;
use rbc::logger::{ThreadLocalLogger, WriteLogger};

fn play_game(color: Color, game_id: i32, ai: &dyn Ai) -> Result<String, api::Error> {
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
    info!("{}", message);
    Ok(message)
}

fn main() {
    log::set_logger(&ThreadLocalLogger).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    ThreadLocalLogger::replace(Box::new(WriteLogger::new(
        std::fs::File::create("logs/challenger_main.info").unwrap()
    )));

    // let ai = rbc::ai_interface::RandomAi {
    //     delay: 60,
    // };
    let ai = rbc::greedy::GreedyAi;

    let (tx, rx) = std::sync::mpsc::channel();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage:");
        eprintln!("    challenger <max_threads>");
        std::process::exit(1);
    }
    let max_threads: usize = args[1].parse().unwrap();
    let mut thread_by_game_id = std::collections::HashMap::new();

    loop {
        while thread_by_game_id.len() < max_threads {
            let mut opponents = api::list_users().unwrap();
            opponents.retain(|o| o != "DotModus_Chris");  // hangs
            let opponent = rand::thread_rng().gen_range(0, opponents.len());
            let opponent = &opponents[opponent];
            let color: Color = rand::thread_rng().gen_bool(0.5).into();
            let game_id = api::post_invitation(opponent, color).unwrap();

            info!("challenger playing against {}", opponent);
            println!("{}: challenge {}", game_id, opponent);
            let t = std::thread::spawn({
                let ai = ai.clone();
                let tx = tx.clone();
                move || {
                    ThreadLocalLogger::replace(Box::new(WriteLogger::new(
                        std::fs::File::create(format!("logs/game_{:05}.info", game_id)).unwrap()
                    )));
                    let message = play_game(color, game_id, &ai).expect("TODO");
                    tx.send((game_id, message)).unwrap();
                }
            });
            thread_by_game_id.insert(game_id, t);
        }

        let (game_id, message) = rx.recv().unwrap();
        info!("{}", message);
        let t = thread_by_game_id.remove(&game_id).unwrap();
        t.join().unwrap();
        println!("{}", message);
    }
}
