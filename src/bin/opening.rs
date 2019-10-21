use rbc::cfr::{Encoding, Cfr};
use rbc::game::{Color, BoardState};

use rbc::rbc_xf::{State, RbcGame};

fn main() {
    let mut ctx = rbc::eval::Ctx::new(BoardState::initial());
    ctx.expensive_eval = true;
    let mut rbc_game = RbcGame::new(1 + 4, 3, &mut ctx, State::ChoosePositionBeforeMove(Color::Black), vec![
        BoardState::initial(),
        fen::BoardState::from_fen("rnbqkb1r/pppppppp/8/8/8/5n2/PPPPPPPP/RNBQKBNR w KQkq - 0 0").unwrap().into(),
        fen::BoardState::from_fen("rnbqkb1r/pppppppp/8/8/8/3n4/PPPPPPPP/RNBQKBNR w KQkq - 0 0").unwrap().into(),
    ]);
    let timer = std::time::Instant::now();
    let enc = Encoding::new(&mut rbc_game);
    println!("it took {:.3}s", timer.elapsed().as_secs_f64());
    // dbg!(&enc);
    dbg!(enc.nodes.len());
    dbg!(enc.infosets.len());
    dbg!(rbc_game.eval_cache.len());
    /*for inf in &enc.infosets {
        println!("{} {:?}", inf.player, inf.orig);
        println!("    {:?}", inf.actions);
    }*/
    // return;
    let mut cfr = Cfr::new(&enc);
    // dbg!(&cfr);
    let timer = std::time::Instant::now();
    for step in 0..30_000 {
        cfr.step(&enc);
        if (step + 1) % 10_000 == 0 {
            let mut strat: Vec<_> = cfr.get_strategy(&enc).into_iter().collect();
            strat.sort_by_key(|(infoset, _)| (infoset.len(), format!("{:?}", infoset)));
            for (infoset, mut ss) in strat {
                if infoset.len() >= 3 {
                    break;
                }
                println!("{:?}", infoset);
                println!("    // ev={}, visit_prob={}", ss.expected_value, ss.visit_prob);
                ss.actions.sort_by(|(_, p1), (_, p2)| p2.partial_cmp(p1).unwrap());
                for (a, p) in ss.actions {
                    if p < 1e-3 {
                        println!("   ...");
                        break;
                    }
                    println!("    {:>5.3} {:?}", p, a);
                }
            }
            println!("----------------");
        }
    }
    println!("it took {:.3}s", timer.elapsed().as_secs_f64());
    // dbg!(cfr.get_strategy(&enc));

    // let mut openins = openin
}
