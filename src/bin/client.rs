use log::info;
use rbc::logger::{ThreadLocalLogger, WriteLogger};
use rbc::api;

fn main() {
    log::set_logger(&ThreadLocalLogger).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    ThreadLocalLogger::replace(Box::new(WriteLogger::new(
        std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("logs/client_main.info").unwrap()
    )));

    let ai = rbc::greedy::GreedyAi;

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

    let (tx, rx) = std::sync::mpsc::channel();

    let me = api::announce_myself().expect("TODO");
    let max_threads = me.max_games as usize;
    println!("{} max threads", max_threads);
    let mut thread_by_game_id = std::collections::HashMap::new();

    loop {
        if running.load(Ordering::SeqCst) {
            if thread_by_game_id.len() < max_threads {
                api::announce_myself().expect("TODO");
            }
            for inv_id in api::list_invitations().expect("TODO") {
                let game_id = api::accept_invitation(inv_id).expect("TODO");
                println!("{}: accepting invitation", game_id);
                info!("{}: accepting invitation", game_id);
                let color = api::game_color(game_id).expect("TODO");

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
        }

        if !thread_by_game_id.is_empty() {
            while let Ok((game_id, message)) = rx.recv_timeout(std::time::Duration::from_secs(5)) {
                info!("{}", message);
                let t = thread_by_game_id.remove(&game_id).unwrap();
                t.join().unwrap();
                println!("{}", message);
            }
        }
        if !running.load(Ordering::SeqCst) && thread_by_game_id.is_empty() {
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    info!("finished");
    println!("finished");
}
