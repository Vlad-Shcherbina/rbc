use rbc::game::BoardState;

fn main() {
    let board: BoardState = fen::BoardState::from_fen(
        // "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 0 0"
        // "r1bqkbnr/ppp2ppp/2np4/4p3/2B1P3/2N2N2/PPPP1PPP/R1BQK2R b KQkq - 0 0"  // opening
        "r2qk2r/p1p2pbp/2npbp1n/4p3/2B1P3/2N1BNQ1/PPP2PPP/R4RK1 b kq - 0 0"  // middle
    ).unwrap().into();
    dbg!(board.render());
    let mut total_nodes = 0;
    let mut prev_nodes = 1;
    let mut pv = Vec::new();
    let timer = std::time::Instant::now();
    let mut ctx = rbc::eval::Ctx::new(board.clone());
    ctx.expensive_eval = true;
    for depth in 0..7 {
        ctx.reset(board.clone());
        ctx.suggested_pv = pv;
        let timer = std::time::Instant::now();
        let val = rbc::eval::search(depth, -10000, 10000, &mut ctx);
        println!("{:>2} {:>6.2}s {:>9} {:>5.1} {:>4} {:?}",
            depth, timer.elapsed().as_secs_f64(),
            ctx.stats.nodes, ctx.stats.nodes as f64 / prev_nodes as f64,
            val, ctx.pvs[0]);
        total_nodes += ctx.stats.nodes;
        prev_nodes = ctx.stats.nodes;

        // dbg!(ctx.full_branch);
        // dbg!(ctx.q_branch);
        pv = ctx.pvs[0].clone();
    }
    dbg!(&ctx.stats);
    println!("{:.0} ns per node", 1e9 * timer.elapsed().as_secs_f64() / total_nodes as f64);
}
