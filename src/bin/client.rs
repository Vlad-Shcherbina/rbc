use std::io::Write;
use log::{info, error};
use rand::Rng;
use rbc::logger::{ThreadLocalLogger, WriteLogger};
use rbc::api;
use rbc::game::{Color, Move};
use rbc::ai_interface::Ai;
use rbc::infoset::Infoset;

pub fn play_game_no_panic(color: Color, game_id: i32, ai: &dyn Ai) -> (char, String) {
    let ai = std::panic::AssertUnwindSafe(ai);
    std::panic::catch_unwind(|| {
        play_game(color, game_id, *ai)
    }).unwrap_or(('E', format!("{}: panic  ", game_id)))
}

pub fn play_game(color: Color, game_id: i32, ai: &dyn Ai) -> (char, String) {
    let seed = rand::thread_rng().gen();
    info!("player seed: {}", seed);
    let mut player = ai.make_player(color, seed);

    let mut halfmove_number = match color {
        Color::White => 0,
        Color::Black => 1,
    };

    let timer = std::time::Instant::now();
    let mut last_time_left = 900.0;

    let mut html = std::fs::File::create(format!("logs/game_{:05}.html", game_id)).unwrap();
    writeln!(html, "{}", rbc::html::PREAMBLE).unwrap();

    let mut infoset = Infoset::new(color);

    loop {
        let gs = api::game_status(game_id).expect("TODO");
        if gs.is_over {
            break;
        }
        if gs.is_my_turn {
            writeln!(html, "<hr>").unwrap();
            match api::seconds_left(game_id) {
                Ok(t) => last_time_left = t,
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
                infoset.opponent_move(capture_square);
                player.handle_opponent_move(capture_square, &infoset, &mut html);
            } else {
                assert!(capture_square.is_none());
            }

            let sense = player.choose_sense(&infoset, &mut html);
            let sense_result = match api::sense(game_id, sense) {
                Ok(sr) => sr,
                Err(api::Error::HttpError(400)) => {
                    let gs = api::game_status(game_id).expect("TODO");
                    assert!(gs.is_over);
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };
            infoset.sense(sense, &sense_result);
            player.handle_sense(sense, &sense_result, &infoset, &mut html);

            let requested = player.choose_move(&infoset, &mut html);
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
            let requested = mr.requested.map(|m| Move::from_uci(&m));
            let taken = mr.taken.map(|m| Move::from_uci(&m));
            infoset.my_move(requested, taken, mr.capture_square);
            player.handle_move(
                requested,
                taken,
                mr.capture_square,
                &infoset,
                &mut html);

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
        None => 'D',
        Some(c) if color == c => 'W',
        _ => 'L',
    };
    let mut message = format!(
        "{}: {} {}; {} moves; {:.0}s/{:.0}s",
        game_id, opponent_name, h.win_reason.0,
        halfmove_number,
        900.0 - last_time_left, timer.elapsed().as_secs_f64());

    if outcome != 'W' && !(opponent_name == "APL_Oracle" && h.win_reason.0 == "KING_CAPTURE") {
        message.push_str("   !!!");
    }

    info!("summary:\n{}", player.get_summary());
    info!("{}", message);
    (outcome, message)
}

struct Slot {
    t: std::thread::JoinHandle<(char, String)>,
    is_challenger: bool,
}

fn print_slots(slots: &[Option<Slot>], idx: usize, c: char) {
    print!("{} ", chrono::offset::Utc::now().format("%m-%d %H:%M:%S"));
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

fn panic_hook(pi: &std::panic::PanicInfo) {
    let payload =
        if let Some(s) = pi.payload().downcast_ref::<&str>() {
            (*s).to_owned()
        } else if let Some(s) = pi.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            String::new()
        };
    let thread = std::thread::current();
    let thread = thread.name().unwrap_or("unnamed");
    let loc = match pi.location() {
        Some(loc) => format!("{}:{}:{}", loc.file(), loc.line(), loc.column()),
        None => "location unknown".to_owned()
    };
    let bt = backtrace::Backtrace::new();
    error!("thread '{}' panicked at {:?}, {}\n{:?}", thread, payload, loc, bt);
    let message = format!("thread '{}' panicked at {:?}, {}", thread, payload, loc);
    println!("{}", message);

    if let Ok(teleg_bot) = std::env::var("TELEG_BOT") {
        let resp = minreq::get(format!("{}{}", teleg_bot, message))
            .with_timeout(10)
            .send();
        if let Err(e) = resp {
            error!("{:?}", e);
        }
    }
}

fn main() {
    log::set_logger(&ThreadLocalLogger).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    ThreadLocalLogger::replace(Box::new(WriteLogger::new(
        std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("logs/client_main.info.txt").unwrap()
    )));

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("usage:");
        eprintln!("    challenger <max challenge threads>");
        std::process::exit(1);
    }
    let arg: i32 = args[1].parse().unwrap();
    let max_challenge_threads = arg.abs() as usize;
    let accept_invites = arg >= 0;

    let ai = rbc::greedy::GreedyAi { experiment: false };

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

    if std::env::var("TELEG_BOT").is_err() {
        println!("TELEG_BOT not set")
    }

    println!("accept invites: {}", accept_invites);
    println!("challenge threads: {}", max_challenge_threads);

    std::panic::set_hook(Box::new(panic_hook));

    let (tx, rx) = std::sync::mpsc::channel();

    let mut slots: Vec<Option<Slot>> = Vec::new();

    let spawn_thread = |slots: &mut Vec<Option<Slot>>, game_id, color, is_challenger: bool| {
        let slot_idx = slots.iter().position(Option::is_none).unwrap_or_else(|| {
            slots.push(None);
            slots.len() - 1
        });
        assert!(slots[slot_idx].is_none());

        let t = std::thread::Builder::new()
        .name(format!("game_{}", game_id))
        .spawn({
            let ai = ai.clone();
            let tx = tx.clone();
            move || {
                ThreadLocalLogger::replace(Box::new(WriteLogger::new(
                    std::fs::File::create(format!("logs/game_{:05}.info.txt", game_id)).unwrap()
                )));
                let (outcome, message) = play_game_no_panic(color, game_id, &ai);
                tx.send(slot_idx).unwrap();
                (outcome, message)
            }
        }).unwrap();
        slots[slot_idx] = Some(Slot { t, is_challenger });
        slot_idx
    };

    loop {
        if running.load(Ordering::SeqCst) {
            if accept_invites {
                api::announce_myself().expect("TODO");
                for inv_id in api::list_invitations().expect("TODO") {
                    let game_id = api::accept_invitation(inv_id).expect("TODO");
                    info!("{}: accepting invitation", game_id);
                    let color = api::game_color(game_id).expect("TODO");
                    let slot_idx = spawn_thread(&mut slots, game_id, color, false);
                    print_slots(&slots, slot_idx, '_');
                    println!("{}", game_id);
                }
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
                println!("{}: {}", game_id, opponent);
            }
        }

        if slots.iter().all(Option::is_none) {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        } else {
            if let Ok(slot_idx) = rx.recv_timeout(std::time::Duration::from_secs(5)) {
                let slot = slots[slot_idx].take().unwrap();
                let (outcome, message) = slot.t.join().unwrap();
                info!("{}", message);
                print_slots(&slots, slot_idx, outcome);
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
