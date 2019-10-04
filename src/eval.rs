use crate::game::{Color, PieceKind, Square, Move, BoardState};

pub fn material_value(k: PieceKind) -> i32 {
    match k {
        PieceKind::Pawn => 100,
        PieceKind::Knight => 350,
        PieceKind::Bishop => 350,
        PieceKind::Rook => 525,
        PieceKind::Queen => 1000,
        PieceKind::King => 100000,
    }
}

pub fn see(
    state: &mut crate::obs::BigState,
    sq: Square, color: Color,
) -> i32 {
    assert_eq!(state.board.side_to_play(), color);
    let cap = state.board.get_piece(sq).unwrap();
    assert_eq!(cap.color, color.opposite());
    let am = state.cheapest_attack_to(sq, color);
    if let Some(am) = am {
        state.push();
        state.make_move(Some(am));
        let result = 0.max(material_value(cap.kind) - see(state, sq, color.opposite()));
        state.pop();
        result
    } else {
        0
    }
}

#[cfg(test)]
#[test]
fn test_see() {
    let board: BoardState = fen::BoardState::from_fen(
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    let mut state = crate::obs::BigState::new(board);
    assert_eq!(see(&mut state, Square::from_san("e4"), Color::Black), 0);

    let board: BoardState = fen::BoardState::from_fen(
        "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    let mut state = crate::obs::BigState::new(board);
    assert_eq!(see(&mut state, Square::from_san("e4"), Color::Black), 100);

    let board: BoardState = fen::BoardState::from_fen(
        "rnbqkb1r/pppppppp/5n2/8/4P3/3B4/PPPP1PPP/RNBQK1NR b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    let mut state = crate::obs::BigState::new(board);
    assert_eq!(see(&mut state, Square::from_san("e4"), Color::Black), 0);

    let board: BoardState = fen::BoardState::from_fen(
        "rnb1kb1r/pppppppp/4qn2/8/4P3/3B4/PPPP1PPP/RNBQK1NR b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    let mut state = crate::obs::BigState::new(board);
    assert_eq!(see(&mut state, Square::from_san("e4"), Color::Black), 100);

    let board: BoardState = fen::BoardState::from_fen(
        "rnbqkbbr/pppppppp/8/6n1/4R3/3P4/PPP1PPPP/RNBQKBN1 b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    let mut state = crate::obs::BigState::new(board);
    assert_eq!(see(&mut state, Square::from_san("e4"), Color::Black),
                material_value(PieceKind::Rook) - material_value(PieceKind::Knight));

    let board: BoardState = fen::BoardState::from_fen(
        "r1bqkbbr/pppppppp/8/2n3n1/4R3/3PQ3/PPP1PPPP/RNB1KBN1 b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    let mut state = crate::obs::BigState::new(board);
    assert_eq!(see(&mut state, Square::from_san("e4"), Color::Black),
                material_value(PieceKind::Rook) - material_value(PieceKind::Knight));
}

fn mobility_value(kind: PieceKind) -> i32 {
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
fn standing_pat(state: &crate::obs::BigState, color: Color, all_moves: &[Move]) -> i32 {
    let board = &state.board;
    let mut static_val = 0;
    for &m in all_moves {
        static_val += mobility_value(board.get_piece(m.from).unwrap().kind);
    }
    let mut b2 = board.clone();
    b2.make_move(None, &mut crate::obs::NullObs);
    for m in b2.all_moves() {
        static_val -= mobility_value(board.get_piece(m.from).unwrap().kind);
    }
    static_val += standing_pat_material_only(state, color);
    static_val
}

#[inline(never)]
fn standing_pat_material_only(state: &crate::obs::BigState, color: Color) -> i32 {
    match color {
        Color::White => state.obs.material,
        Color::Black => -state.obs.material,
    }
}

pub struct Ctx {
    state: crate::obs::BigState,
    ply: usize,
    pub pvs: Vec<Vec<Move>>,
    pub print: bool,
    pub expensive_eval: bool,
}

impl Ctx {
    pub fn new(board: BoardState) -> Ctx {
        Ctx {
            state: crate::obs::BigState::new(board),
            ply: 0,
            pvs: Vec::new(),
            print: false,
            expensive_eval: false,
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

    let color = ctx.state.board.side_to_play();
    let king = match ctx.state.find_king(color) {
        None => return (-10000 + ctx.ply as i32).max(alpha).min(beta),
        Some(sq) => sq,
    };
    let opp_king = match ctx.state.find_king(color.opposite()) {
        None => return (10000 - ctx.ply as i32).max(alpha).min(beta),
        Some(sq) => sq,
    };
    if let Some(ka) = ctx.state.cheapest_attack_to(opp_king, color) {
        ctx.pvs[ctx.ply].push(ka);
        return (10000 - 1 - ctx.ply as i32).max(alpha).min(beta);
    }

    let mut all_moves = ctx.state.board.all_moves();
    tree_println!(ctx, "alpha={} beta={}", alpha, beta);
    if depth == 0 && ctx.state.cheapest_attack_to(king, color.opposite()).is_none() {
        let static_val = if ctx.expensive_eval {
            standing_pat(&ctx.state, color, &all_moves)
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

        let mut ranked_moves: Vec<(Move, i32)> = Vec::with_capacity(all_moves.len());
        for m in all_moves {
            let cap2 = match ctx.state.board.get_piece(m.to) {
                None => continue,
                Some(p) => p,
            };
            ctx.state.push();
            ctx.state.make_move(Some(m));
            let rank = material_value(cap2.kind) - see(&mut ctx.state, m.to, color.opposite());
            ctx.state.pop();
            if rank >= 0 {
                ranked_moves.push((m, rank));
            }
        }
        ranked_moves.sort_by_key(|&(_, rank)| -rank);
        all_moves = ranked_moves.into_iter().map(|(m, _)| m).collect();
    }
    for m in all_moves {
        ctx.state.push();
        ctx.state.make_move(Some(m));
        ctx.ply += 1;
        let t = -search((depth - 1).max(0), -beta, -alpha, ctx);
        ctx.ply -= 1;
        ctx.state.pop();
        if t > alpha {
            ctx.pvs[ctx.ply].clear();
            ctx.pvs[ctx.ply].push(m);

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
