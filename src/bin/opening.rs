use rbc::cfr::{NodeInfo, Game, Encoding, Cfr};
use rbc::game::BoardState;

use rbc::rbc_xf::RbcGame;

fn main() {
    let mut ctx = rbc::eval::Ctx::new(BoardState::initial());
    ctx.expensive_eval = true;
    let rbc_game = RbcGame { ctx: std::cell::RefCell::new(&mut ctx) };
    let timer = std::time::Instant::now();
    let enc = Encoding::new(&rbc_game);
    println!("it took {:.3}s", timer.elapsed().as_secs_f64());
    // dbg!(&enc);
    dbg!(enc.nodes.len());
    dbg!(enc.infosets.len());
    /*for inf in &enc.infosets {
        println!("{} {:?}", inf.player, inf.orig);
        println!("    {:?}", inf.actions);
    }*/
    return;
    let mut cfr = Cfr::new(&enc);
    // dbg!(&cfr);
    for step in 0..100_000 {
        cfr.step(&enc);
        if step % 10_000 == 0 {
            let mut strat: Vec<_> = cfr.get_strategy(&enc).into_iter().collect();
            strat.sort_by_key(|(infoset, _)| infoset.len());
            for (infoset, mut actions) in strat {
                println!("{:?}", infoset);
                actions.sort_by(|(_, p1), (_, p2)| p2.partial_cmp(p1).unwrap());
                for (a, p) in actions {
                    if p < 1e-3 {
                        println!("   ...");
                        break;
                    }
                    println!("   {:>5.3} {:?}", p, a);
                }
            }
            println!("----------------");
        }
    }
    // dbg!(cfr.get_strategy(&enc));

    // let mut openins = openin
}
