use crate::game::{Color, PieceKind, Square, Move, BoardState};

fn material_value(k: PieceKind) -> i32 {
    match k {
        PieceKind::Pawn => 100,
        PieceKind::Knight => 350,
        PieceKind::Bishop => 350,
        PieceKind::Rook => 525,
        PieceKind::Queen => 1000,
        PieceKind::King => 100000,
    }
}

pub fn see(board: &BoardState, sq: Square, color: Color) -> i32 {
    assert_eq!(board.side_to_play(), color);
    let cap = board.get_piece(sq).unwrap();
    assert_eq!(cap.color, color.opposite());
    let am = board.all_attacks_to(sq, color)
        .into_iter()
        .min_by_key(|am| {
            let kind = am.promotion.unwrap_or(board.get_piece(am.from).unwrap().kind);
            material_value(kind)
        });
    if let Some(am) = am {
        let mut board2 = board.clone();
        board2.make_move(Some(am));
        0.max(material_value(cap.kind) - see(&board2, sq, color.opposite()))
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
    assert_eq!(see(&board, Square::from_san("e4"), Color::Black), 0);

    let board: BoardState = fen::BoardState::from_fen(
        "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    assert_eq!(see(&board, Square::from_san("e4"), Color::Black), 100);

    let board: BoardState = fen::BoardState::from_fen(
        "rnbqkb1r/pppppppp/5n2/8/4P3/3B4/PPPP1PPP/RNBQK1NR b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    assert_eq!(see(&board, Square::from_san("e4"), Color::Black), 0);

    let board: BoardState = fen::BoardState::from_fen(
        "rnb1kb1r/pppppppp/4qn2/8/4P3/3B4/PPPP1PPP/RNBQK1NR b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    assert_eq!(see(&board, Square::from_san("e4"), Color::Black), 100);

    let board: BoardState = fen::BoardState::from_fen(
        "rnbqkbbr/pppppppp/8/6n1/4R3/3P4/PPP1PPPP/RNBQKBN1 b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    assert_eq!(see(&board, Square::from_san("e4"), Color::Black),
                material_value(PieceKind::Rook) - material_value(PieceKind::Knight));

    let board: BoardState = fen::BoardState::from_fen(
        "r1bqkbbr/pppppppp/8/2n3n1/4R3/3PQ3/PPP1PPPP/RNB1KBN1 b KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    assert_eq!(see(&board, Square::from_san("e4"), Color::Black),
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
fn standing_pat(board: &BoardState, color: Color, all_moves: &[Move]) -> i32 {
    let mut static_val = 0;
    for &m in all_moves {
        static_val += mobility_value(board.get_piece(m.from).unwrap().kind);
    }
    let mut b2 = board.clone();
    b2.make_move(None);
    for m in b2.all_moves() {
        static_val -= mobility_value(board.get_piece(m.from).unwrap().kind);
    }
    for sq in (0..64).map(Square) {
        if let Some(p) = board.get_piece(sq) {
            let v = material_value(p.kind);
            if p.color == color {
                static_val += v;
            } else {
                static_val -= v;
            }
        }
    }
    static_val
}

#[inline(never)]
fn standing_pat_material_only(board: &BoardState, color: Color) -> i32 {
    let mut static_val = 0;
    for sq in (0..64).map(Square) {
        if let Some(p) = board.get_piece(sq) {
            let v = material_value(p.kind);
            if p.color == color {
                static_val += v;
            } else {
                static_val -= v;
            }
        }
    }
    static_val
}

pub struct Ctx {
    pub ply: usize,
    pub pvs: Vec<Vec<Move>>,
    pub print: bool,
    pub expensive_eval: bool,
}

macro_rules! tree_println {
    ($ctx:expr, $($arg:tt)*) => ({
        if $ctx.print {
            print!("{}", "| ".repeat($ctx.ply));
            println!($($arg)*);
        }
    })
}

pub fn search(board: &BoardState, mut alpha: i32, beta: i32, ctx: &mut Ctx) -> i32 {
    assert!(alpha < beta);
    while ctx.pvs.len() <= ctx.ply {
        ctx.pvs.push(Vec::new());
    }
    ctx.pvs[ctx.ply].clear();

    let color = board.side_to_play();
    let king = board.find_king(color);
    if king.is_none() {
        return (-10000 + ctx.ply as i32).max(alpha).min(beta);
    }
    let king = king.unwrap();

    let opp_king = board.find_king(color.opposite());
    if opp_king.is_none() {
        return (10000 - ctx.ply as i32).max(alpha).min(beta);
    }
    let king_attacks = board.all_attacks_to(opp_king.unwrap(), color);
    if !king_attacks.is_empty() {
        ctx.pvs[ctx.ply].push(king_attacks[0]);
        return (10000 - 1 - ctx.ply as i32).max(alpha).min(beta);
    }

    let mut all_moves = board.all_moves();
    tree_println!(ctx, "alpha={} beta={}", alpha, beta);
    if board.all_attacks_to(king, color.opposite()).is_empty() {
        let static_val = if ctx.expensive_eval {
            standing_pat(board, color, &all_moves)
        } else {
            standing_pat_material_only(board, color)
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
            if board.get_piece(m.to).is_none() {
                continue;
            }
            let mut b2 = board.clone();
            let cap = b2.make_move(Some(m));
            let cap = board.get_piece(cap.unwrap()).unwrap();
            let rank = material_value(cap.kind) - see(&b2, m.to, color.opposite());
            if rank >= 0 {
                ranked_moves.push((m, rank));
            }
        }
        ranked_moves.sort_by_key(|&(_, rank)| -rank);
        all_moves = ranked_moves.into_iter().map(|(m, _)| m).collect();
    }
    for m in all_moves {
        let mut b2 = board.clone();
        b2.make_move(Some(m));
        ctx.ply += 1;
        let t = -search(&b2, -beta, -alpha, ctx);
        ctx.ply -= 1;
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
    let mut ctx = Ctx {
        ply: 0,
        pvs: Vec::new(),
        print: true,
        expensive_eval: false,
    };
    let q = search(&board, -10000, 10000, &mut ctx);
    assert_eq!(ctx.ply, 0);
    dbg!(q);
    dbg!(&ctx.pvs[0]);
}
