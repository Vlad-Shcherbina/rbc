use log::info;
use rand::Rng;
use rbc::logger::{ThreadLocalLogger, WriteLogger};
use rbc::api;
use rbc::game::{Color, Move};
use rbc::ai_interface::Ai;

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

struct Slot {
    t: std::thread::JoinHandle<()>,
    is_challenger: bool,
}

fn print_slots(slots: &[Option<Slot>], idx: usize, c: char) {
    for (i, s) in slots.iter().enumerate() {
        if i == idx {
            print!("{} ", c)
        } else if s.is_none() {
            print!("  ");
        } else {
            print!("| ");
        };
    }
}

fn main() {
    log::set_logger(&ThreadLocalLogger).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    ThreadLocalLogger::replace(Box::new(WriteLogger::new(
        std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("logs/client_main.info").unwrap()
    )));

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage:");
        eprintln!("    challenger <max challenge threads>");
        std::process::exit(1);
    }
    let max_challenge_threads: usize = args[1].parse().unwrap();

    let ai = rbc::greedy::GreedyAi;

    use std::sync::atomic::{AtomicBool, Ordering};
    let running = std::sync::Arc::new(AtomicBool::new(true));
    ctrlc::set_handler({
        let running = running.clone();
        move || {
            // logging won't work here because it's a separate thread
            if !running.load(Ordering::SeqCst) {
                println!("Ctrl-C, exiting for real");
                std::process::exit(1);
            }
            println!("Ctrl-C, entering lame duck mode");
            running.store(false, Ordering::SeqCst);
        }
    }).unwrap();

    info!("**********************");
    println!("**********************");

    let (tx, rx) = std::sync::mpsc::channel();

    let me = api::announce_myself().expect("TODO");
    println!("{} max games", me.max_games);
    let mut slots: Vec<Option<Slot>> = Vec::new();

    let spawn_thread = |slots: &mut Vec<Option<Slot>>, game_id, color, is_challenger: bool| {
        let slot_idx = slots.iter().position(Option::is_none).unwrap_or_else(|| {
            slots.push(None);
            slots.len() - 1
        });
        assert!(slots[slot_idx].is_none());

        let t = std::thread::spawn({
            let ai = ai.clone();
            let tx = tx.clone();
            move || {
                ThreadLocalLogger::replace(Box::new(WriteLogger::new(
                    std::fs::File::create(format!("logs/game_{:05}.info", game_id)).unwrap()
                )));
                let message = play_game(color, game_id, &ai);
                tx.send((slot_idx, message)).unwrap();
            }
        });
        slots[slot_idx] = Some(Slot { t, is_challenger });
        slot_idx
    };

    loop {
        if running.load(Ordering::SeqCst) {
            let current_games = slots.iter().filter(|s| s.is_some()).count();
            if current_games < me.max_games as usize {
                api::announce_myself().expect("TODO");
            }
            for inv_id in api::list_invitations().expect("TODO") {
                let game_id = api::accept_invitation(inv_id).expect("TODO");
                info!("{}: accepting invitation", game_id);
                let color = api::game_color(game_id).expect("TODO");
                let slot_idx = spawn_thread(&mut slots, game_id, color, false);
                print_slots(&slots, slot_idx, '_');
                println!("{}: accepting invitation", game_id);
            }
            loop {
                let num_challengers = slots.iter()
                    .filter(|s| s.as_ref().map_or(false, |s| s.is_challenger))
                    .count();
                if num_challengers >= max_challenge_threads {
                    break;
                }
                let mut opponents = api::list_users().unwrap();
                opponents.retain(|o|
                    o != "DotModus_Chris" &&  // hangs
                    o != "genetic"
                );
                let opponent = rand::thread_rng().gen_range(0, opponents.len());
                let opponent = &opponents[opponent];
                let color: Color = rand::thread_rng().gen_bool(0.5).into();
                let game_id = api::post_invitation(opponent, color).unwrap();
                info!("challenger playing against {}", opponent);
                let slot_idx = spawn_thread(&mut slots, game_id, color, true);
                print_slots(&slots, slot_idx, '.');
                println!("{}: challenge {}", game_id, opponent);
            }
        }

        if slots.iter().all(Option::is_none) {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        } else {
            if let Ok((slot_idx, message)) = rx.recv_timeout(std::time::Duration::from_secs(5)) {
                info!("{}", message);
                let slot = slots[slot_idx].take().unwrap();
                slot.t.join().unwrap();
                print_slots(&slots, slot_idx, '*');
                println!("{}", message);

                while slots.len() > 5 && slots.last().unwrap().is_none() {
                    slots.pop();
                }
            }
        }
    }
    info!("finished");
    println!("finished");
}
