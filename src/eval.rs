use crate::game::{Color, PieceKind, Square, Move, BoardState};

pub fn material_value(k: PieceKind) -> i32 {
    match k {
        PieceKind::Pawn => 100,
        PieceKind::Knight => 350,
        PieceKind::Bishop => 350,
        PieceKind::Rook => 525,
        PieceKind::Queen => 1000,
        PieceKind::King => 10000,
    }
}

pub fn see(
    state: &mut crate::fast::State,
    undo_log: &mut Vec<crate::fast::UndoEntry>,
    sq: Square, color: Color,
) -> i32 {
    assert_eq!(state.side_to_play(), color);
    let cap = state.get_piece(sq).unwrap();
    assert_eq!(cap.color, color.opposite());
    let am = state.cheapest_attack_to(sq, color);
    if let Some(am) = am {
        state.make_move(am, undo_log);
        let result = 0.max(material_value(cap.kind) - see(state, undo_log, sq, color.opposite()));
        state.unmake_move(am, undo_log);
        result
    } else {
        0
    }
}

#[cfg(test)]
#[test]
fn test_see() {
    for &(b, expected) in &[
        ("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 0", 0),
        ("rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 0", 100),
        ("rnbqkb1r/pppppppp/5n2/8/4P3/3B4/PPPP1PPP/RNBQK1NR b KQkq - 0 0", 0),
        ("rnb1kb1r/pppppppp/4qn2/8/4P3/3B4/PPPP1PPP/RNBQK1NR b KQkq - 0 0", 100),
        ("rnbqkbbr/pppppppp/8/6n1/4R3/3P4/PPP1PPPP/RNBQKBN1 b KQkq - 0 0",
         material_value(PieceKind::Rook) - material_value(PieceKind::Knight)),
        ("r1bqkbbr/pppppppp/8/2n3n1/4R3/3PQ3/PPP1PPPP/RNB1KBN1 b KQkq - 0 0",
         material_value(PieceKind::Rook) - material_value(PieceKind::Knight)),
    ] {
        let board: BoardState = fen::BoardState::from_fen(b).unwrap().into();
        dbg!(board.render());
        let mut s: crate::fast::State = (&board).into();
        let mut undo_log = Vec::new();
        assert_eq!(see(&mut s, &mut undo_log, Square::from_san("e4"), Color::Black), expected);
    }
}

pub fn mobility_value(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Pawn => 0,
        PieceKind::Knight |
        PieceKind::Bishop => 3,
        PieceKind::Rook => 2,
        PieceKind::Queen => 1,
        PieceKind::King => 1,
    }
}

// https://www.chessprogramming.org/Quiescence_Search#Standing_Pat
#[inline(never)]
fn standing_pat(state: &crate::fast::State, color: Color) -> i32 {
    standing_pat_material_only(state, color) + state.mobility(color) - state.mobility(color.opposite())
}

#[inline(never)]
fn standing_pat_material_only(state: &crate::fast::State, color: Color) -> i32 {
    state.material(color) - state.material(color.opposite())
}

#[cfg(test)]
#[test]
fn test_standing_pat() {
    let board: BoardState = fen::BoardState::from_fen(
        "r1bqk2r/p1pp1ppp/2p5/4N3/4n3/2P5/PPP2PPP/R1BQK2R w KQkq - 0 0"
    ).unwrap().into();
    dbg!(board.render());
    let s: crate::fast::State = (&board).into();
    dbg!(standing_pat(&s, Color::White));
    dbg!(standing_pat(&s, Color::Black));
    dbg!(s.material(Color::White));
    dbg!(s.material(Color::Black));
    dbg!(s.mobility(Color::White));
    dbg!(s.mobility(Color::Black));
}

pub struct Ctx {
    state: crate::fast::State,
    undo_log: Vec<crate::fast::UndoEntry>,
    ply: usize,
    leftmost: bool,
    pub pvs: Vec<Vec<Move>>,
    pub suggested_pv: Vec<Move>,
    pub print: bool,
    pub expensive_eval: bool,
    pub nodes: i64,
    pub full_branch: i64,
    pub q_branch: i64,
}

impl Ctx {
    pub fn new(board: BoardState) -> Ctx {
        Ctx {
            state: (&board).into(),
            undo_log: Vec::new(),
            ply: 0,
            leftmost: true,
            pvs: Vec::new(),
            suggested_pv: Vec::new(),
            print: false,
            expensive_eval: false,
            nodes: 0,
            full_branch: 0,
            q_branch: 0,
        }
    }
}

macro_rules! tree_println {
    ($ctx:expr, $($arg:tt)*) => ({
        if $ctx.print {
            print!("{}", "| ".repeat($ctx.ply));
            println!($($arg)*);
        }
    })
}

pub fn search(depth: i32, mut alpha: i32, beta: i32, ctx: &mut Ctx) -> i32 {
    assert!(alpha < beta);
    while ctx.pvs.len() <= ctx.ply {
        ctx.pvs.push(Vec::new());
    }
    ctx.pvs[ctx.ply].clear();
    ctx.nodes += 1;

    let color = ctx.state.side_to_play();
    let king = match ctx.state.find_king(color) {
        None => return (-10000 + ctx.ply as i32).max(alpha).min(beta),
        Some(sq) => sq,
    };
    let opp_king = match ctx.state.find_king(color.opposite()) {
        None => return (10000 - ctx.ply as i32).max(alpha).min(beta),
        Some(sq) => sq,
    };
    if ctx.state.can_attack_to(opp_king, color) {
        ctx.pvs[ctx.ply].push(
            ctx.state.cheapest_attack_to(opp_king, color).unwrap()
            .to_simple_move().unwrap());
        return (10000 - 1 - ctx.ply as i32).max(alpha).min(beta);
    }

    let mut all_moves = Vec::with_capacity(128);
    tree_println!(ctx, "alpha={} beta={}", alpha, beta);
    if depth == 0 && !ctx.state.can_attack_to(king, color.opposite()) {
        ctx.state.all_attacks(&mut all_moves);
        ctx.q_branch += 1;
        let static_val = if ctx.expensive_eval {
            standing_pat(&mut ctx.state, color)
        } else {
            standing_pat_material_only(&ctx.state, color)
        };
        if static_val >= beta {
            tree_println!(ctx, "standing pat cutoff {}", static_val);
            return beta;
        }
        if static_val > alpha {
            tree_println!(ctx, "standing pat imp {}", static_val);
            alpha = static_val;
        } else {
            tree_println!(ctx, "standing pat no imp {}", static_val);
        }

        let mut ranked_moves: Vec<(crate::fast::Move, i32)> = Vec::with_capacity(all_moves.len());
        for &m in &all_moves {
            let cap2 = ctx.state.get_piece(m.to_sq()).unwrap();
            ctx.state.make_move(m, &mut ctx.undo_log);
            let rank = material_value(cap2.kind) - see(&mut ctx.state, &mut ctx.undo_log, m.to_sq(), color.opposite());
            ctx.state.unmake_move(m, &mut ctx.undo_log);
            if rank >= 0 {
                ranked_moves.push((m, rank));
            }
        }
        ranked_moves.sort_by_key(|&(_, rank)| -rank);
        all_moves.clear();
        for (m, _) in ranked_moves {
            all_moves.push(m);
        }
    } else {
        ctx.state.all_moves(&mut all_moves);
        ctx.full_branch += 1;
    }
    if ctx.leftmost && ctx.ply < ctx.suggested_pv.len() {
        if let Some(i) = all_moves.iter().position(|&m| m.to_simple_move().unwrap() == ctx.suggested_pv[ctx.ply]) {
            all_moves.swap(0, i);
        }
    }
    for m in all_moves {
        ctx.state.make_move(m, &mut ctx.undo_log);
        ctx.ply += 1;
        let t = -search((depth - 1).max(0), -beta, -alpha, ctx);
        ctx.ply -= 1;
        ctx.state.unmake_move(m, &mut ctx.undo_log);
        ctx.leftmost = false;
        if t > alpha {
            ctx.pvs[ctx.ply].clear();
            ctx.pvs[ctx.ply].push(m.to_simple_move().unwrap());

            // pvs[ply].extend_from_slice(&pvs[ply + 1])
            // but borrowck-friendly
            let (l, r) = ctx.pvs.split_at_mut(ctx.ply + 1);
            l.last_mut().unwrap().extend_from_slice(r.first().unwrap());
        }
        if t >= beta {
            tree_println!(ctx, "ev {:?} cutoff {}", m, t);
            return beta;
        }
        if t > alpha {
            tree_println!(ctx, "ev {:?} imp {}", m, t);
            alpha = t;
        } else {
            tree_println!(ctx, "ev {:?} no imp {}", m, t);
        }
    }
    alpha
}

#[cfg(test)]
#[test]
fn test_quiescence() {
    let board: BoardState = fen::BoardState::from_fen(
        "rnbqkbnr/pppp1ppp/8/4p2Q/4P3/8/PPPP1PPP/RNB1KBNR w KQkq - 0 0").unwrap().into();
    // let board: BoardState = fen::BoardState::from_fen(
        // "rnbqk2r/pppppppp/5n2/3b4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    let mut ctx = Ctx::new(board);
    ctx.print = true;
    let q = search(0, -10000, 10000, &mut ctx);
    assert_eq!(ctx.ply, 0);
    dbg!(q);
    dbg!(&ctx.pvs[0]);
}
