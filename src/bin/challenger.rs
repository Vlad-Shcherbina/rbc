use log::info;
use rand::Rng;
use rbc::game::Color;
use rbc::api;
use rbc::logger::{ThreadLocalLogger, WriteLogger};

fn main() {
    log::set_logger(&ThreadLocalLogger).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    ThreadLocalLogger::replace(Box::new(WriteLogger::new(
        std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("logs/challenger_main.info").unwrap()
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

    use std::sync::atomic::{AtomicBool, Ordering};
    let running = std::sync::Arc::new(AtomicBool::new(true));
    ctrlc::set_handler({
        let running = running.clone();
        move || {
            // logging won't work here because it's a separate thread
            if !running.load(Ordering::SeqCst) {
                println!("exiting for real");
                std::process::exit(1);
            }
            println!("Ctrl-C, entering lame duck mode");
            running.store(false, Ordering::SeqCst);
        }
    }).unwrap();

    info!("**********************");
    println!("**********************");

    loop {
        while running.load(Ordering::SeqCst) && thread_by_game_id.len() < max_threads {
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
            println!("{}: challenge {}", game_id, opponent);
            let t = std::thread::spawn({
                let ai = ai.clone();
                let tx = tx.clone();
                move || {
                    ThreadLocalLogger::replace(Box::new(WriteLogger::new(
                        std::fs::File::create(format!("logs/game_{:05}.info", game_id)).unwrap()
                    )));
                    let message = rbc::interact::play_game(color, game_id, &ai);
                    tx.send((game_id, message)).unwrap();
                }
            });
            thread_by_game_id.insert(game_id, t);
        }

        if !running.load(Ordering::SeqCst) && thread_by_game_id.is_empty() {
            break;
        }

        assert!(!thread_by_game_id.is_empty());
        let (game_id, message) = rx.recv().unwrap();
        info!("{}", message);
        let t = thread_by_game_id.remove(&game_id).unwrap();
        t.join().unwrap();
        println!("{}", message);
        if !running.load(Ordering::SeqCst) {
            info!("in lame duck mode, not challenging anymore");
        }
    }
    info!("finished");
    println!("finished");
}
