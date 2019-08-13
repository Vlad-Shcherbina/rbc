use rbc::api;

fn main() {
    env_logger::init();

    let h = api::game_history(15144).expect("TODO");
    dbg!(h);
}
