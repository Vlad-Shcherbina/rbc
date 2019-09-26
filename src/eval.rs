use crate::game::{Color, PieceKind, Square, BoardState};

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

pub fn quiescence(board: &BoardState, depth: i32, mut alpha: i32, beta: i32) -> i32 {
    assert!(alpha <= beta);
    let color = board.side_to_play();
    let king = board.find_king(color);
    if king.is_none() {
        return alpha;
    }
    let king = king.unwrap();

    let opp_king = board.find_king(color.opposite());
    if opp_king.is_none() {
        return beta;
    }
    if !board.all_attacks_to(opp_king.unwrap(), color).is_empty() {
        return beta;
    }

    let all_moves = board.all_moves();
    let in_check = if board.all_attacks_to(king, color.opposite()).is_empty() {
        let mut static_val = 0;
        for &m in &all_moves {
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
        if static_val >= beta {
            return beta;
        }
        alpha = alpha.max(static_val);
        false
    } else {
        true
    };
    for m in all_moves {
        if !in_check && board.get_piece(m.to).is_none() {
            continue;
        }
        let mut b2 = board.clone();
        let cap = b2.make_move(Some(m));
        assert!(in_check || cap.is_some());
        if !in_check {
            let cap = board.get_piece(cap.unwrap()).unwrap();
            if material_value(cap.kind) < see(&b2, m.to, color.opposite()) {
                continue;
            }
        }
        let t = -quiescence(&b2, depth + 1, -beta, -alpha);
        if t >= beta {
            return beta;
        }
        alpha = alpha.max(t);
    }
    alpha
}

#[cfg(test)]
#[test]
fn test_quiescence() {
    let board: BoardState = fen::BoardState::from_fen(
        "rnbqkbnr/pppp1ppp/8/4p2Q/4P3/8/PPPP1PPP/RNB1KBNR w KQkq - 0 0").unwrap().into();
    dbg!(board.render());
    let q = quiescence(&board, 0, -3000, 3000);
    dbg!(q);
}
