use rbc::api;

fn main() {
    env_logger::init();

    let mut game_id = 18198;

    loop {
        match api::game_history(game_id) {
            Ok(h) => {
                println!(
                    "{}: {:>25} - {:25} {:?} {:12}  {} moves",
                    game_id,
                    h.white_name, h.black_name, h.winner_color, h.win_reason, h.moves.len());
            }
            Err(api::Error::HttpError(404)) => {
                println!("{}: game does not exist", game_id);
            }
            Err(api::Error::HttpError(400)) => {
                println!("{}: game is not over", game_id);
            }
            Err(e) => panic!("{:?}", e),
        }
        game_id -= 1;
    }
}
