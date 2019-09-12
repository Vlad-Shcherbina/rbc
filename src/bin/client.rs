use log::{info, error};

use rbc::api;

fn main() {
    env_logger::init();

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        info!("Ctrl-C, entering lame duck mode");
        r.store(false, Ordering::SeqCst);
    }).unwrap();

    api::list_users().expect("TODO");
    let me = api::announce_myself().expect("TODO");

    let mut game_ids: Vec<i32> = Vec::new();
    loop {
        api::announce_myself().expect("TODO");
        info!("active games: {:?}", game_ids);

        if game_ids.is_empty() && !running.load(Ordering::SeqCst) {
            info!("done");
            break;
        }

        game_ids.retain(|&game_id| {
            let gs = api::game_status(game_id).expect("TODO");
            if gs.is_over {
                let my_color = api::game_color(game_id).expect("TODO");
                let winner = api::winner_color(game_id).expect("TODO");
                let win_reason = api::win_reason(game_id).expect("TODO");
                if my_color == winner {
                    info!("I won game {} ({})", game_id, win_reason);
                } else {
                    info!("I lost game {} ({})", game_id, win_reason);
                }
                false
            } else {
                if gs.is_my_turn {
                    match api::seconds_left(game_id) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("{:?}", e);
                            return false;
                        }
                    }
                    match api::opponent_move_results(game_id) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("{:?}", e);
                            return false;
                        }
                    }
                    match api::sense(game_id, 0.into()) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("{:?}", e);
                            return false;
                        }
                    }
                    match api::make_move(game_id, "d2d4".to_owned()) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("{:?}", e);
                            return false;
                        }
                    }
                    match api::end_turn(game_id) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("{:?}", e);
                            return false;
                        }
                    }
                }
                true
            }
        });

        if running.load(Ordering::SeqCst) {
            for inv_id in api::list_invitations().expect("TODO") {
                if game_ids.len() < me.max_games as usize {
                    game_ids.push(api::accept_invitation(inv_id).expect("TODO"));
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
