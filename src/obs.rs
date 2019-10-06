use crate::game::{Square, Color, PieceKind, Piece, Move, BoardFlags, BoardState};

pub trait Obs {
    fn replace_piece(&mut self, sq: Square, old: Option<Piece>, new: Option<Piece>);
}

pub struct NullObs;

impl Obs for NullObs {
    fn replace_piece(&mut self, _sq: Square, _old: Option<Piece>, _new: Option<Piece>) {}
}

struct UndoState {
    edit_pos: usize,
    flags: crate::game::BoardFlags,
    en_passant_square: Option<Square>,
}

pub struct StateObs {
    undo_states: Vec<UndoState>,
    edits: Vec<(Square, Option<Piece>, Option<Piece>)>,
    pub material: i32,
    pub white: u64,
    pub black: u64,
    pub pawns: u64,
    pub knights: u64,
    pub bishops: u64,
    pub rooks: u64,
    pub queens: u64,
    pub kings: u64,
}

impl StateObs {
    pub fn new(b: &BoardState) -> StateObs {
        let mut obs = StateObs {
            undo_states: Vec::new(),
            edits: Vec::new(),
            material: 0,
            white: 0,
            black: 0,
            pawns: 0,
            knights: 0,
            bishops: 0,
            rooks: 0,
            queens: 0,
            kings: 0,
        };
        for sq in (0..64).map(Square) {
            let p = b.get_piece(sq);
            if p.is_some() {
                obs.replace_piece_raw(sq, None, p);
            }
        }
        obs
    }

    fn replace_piece_raw(&mut self, sq: Square, old: Option<Piece>, new: Option<Piece>) {
        let bit = 1u64 << sq.0;
        if let Some(p) = old {
            match p.color {
                Color::White => self.white ^= bit,
                Color::Black => self.black ^= bit,
            }
            match p.kind {
                PieceKind::Pawn => self.pawns ^= bit,
                PieceKind::Knight => self.knights ^= bit,
                PieceKind::Bishop => self.bishops ^= bit,
                PieceKind::Rook => self.rooks ^= bit,
                PieceKind::Queen => self.queens ^= bit,
                PieceKind::King => self.kings ^= bit,
            }
            match p.color {
                Color::White =>
                    self.material -= crate::eval::material_value(p.kind),
                Color::Black =>
                    self.material += crate::eval::material_value(p.kind),
            }
        }
        if let Some(p) = new {
            match p.color {
                Color::White => self.white ^= bit,
                Color::Black => self.black ^= bit,
            }
            match p.kind {
                PieceKind::Pawn => self.pawns ^= bit,
                PieceKind::Knight => self.knights ^= bit,
                PieceKind::Bishop => self.bishops ^= bit,
                PieceKind::Rook => self.rooks ^= bit,
                PieceKind::Queen => self.queens ^= bit,
                PieceKind::King => self.kings ^= bit,
            }
            match p.color {
                Color::White =>
                    self.material += crate::eval::material_value(p.kind),
                Color::Black =>
                    self.material -= crate::eval::material_value(p.kind),
            }
        }
    }

    pub fn push(&mut self, b: &BoardState) {
        self.undo_states.push(UndoState {
            edit_pos: self.edits.len(),
            flags: b.flags,
            en_passant_square: b.en_passant_square,
        });
    }

    pub fn pop(&mut self, b: &mut BoardState) {
        let us = self.undo_states.pop().unwrap();
        assert!(self.edits.len() >= us.edit_pos);
        while self.edits.len() > us.edit_pos {
            let (sq, old, new) = self.edits.pop().unwrap();
            self.replace_piece_raw(sq, new, old);
            let new2 = b.replace_piece(sq, old, &mut NullObs);
            assert_eq!(new, new2);
        }
        b.flags = us.flags;
        b.en_passant_square = us.en_passant_square;
    }
}

impl Obs for StateObs {
    fn replace_piece(&mut self, sq: Square, old: Option<Piece>, new: Option<Piece>) {
        self.edits.push((sq, old, new));
        self.replace_piece_raw(sq, old, new);
    }
}

#[cfg(test)]
#[test]
fn test_obs() {
    use crate::game::Move;
    let mut b = BoardState::initial();
    let mut obs = StateObs::new(&b);
    let m = Move::from_uci("e2e4");
    obs.push(&b);
    b.make_move(Some(m), &mut obs);
    obs.pop(&mut b);
    assert_eq!(b, BoardState::initial());
}

pub struct BigState {
    pub board: BoardState,
    pub obs: StateObs,
}

impl BigState {
    pub fn new(board: BoardState) -> BigState {
        BigState {
            obs: StateObs::new(&board),
            board,
        }
    }

    pub fn push(&mut self) {
        self.obs.push(&self.board);
    }

    pub fn pop(&mut self) {
        self.obs.pop(&mut self.board);
    }

    pub fn make_move(&mut self, m: Option<Move>) -> Option<Square> {
        self.board.make_move(m, &mut self.obs)
    }
}

impl BigState {
    pub fn find_king(&self, color: Color) -> Option<Square> {
        let k = self.obs.kings & match color {
            Color::White => self.obs.white,
            Color::Black => self.obs.black,
        };
        if k == 0 {
            None
        } else {
            Some(Square(k.trailing_zeros() as i8))
        }
    }

    pub fn cheapest_attack_to_for_testing(&self, to: Square, color: Color) -> Option<Move> {
        use crate::eval::material_value;
        let naive = self.cheapest_attack_to_naive(to, color);
        let fast = self.cheapest_attack_to(to, color);
        match (naive, fast) {
            (None, None) => {}
            (Some(m1), Some(m2)) => assert_eq!(
                material_value(m1.promotion.unwrap_or(self.board.get_piece(m1.from).unwrap().kind)),
                material_value(m2.promotion.unwrap_or(self.board.get_piece(m2.from).unwrap().kind)),
                "{:#?} naive:{:?} fast:{:?}", self.board.render(), naive, fast,
            ),
            _ => panic!("{:#?} naive:{:?} fast:{:?}", self.board.render(), naive, fast),
        }
        fast
    }

    #[inline(never)]
    pub fn cheapest_attack_to_naive(&self, to: Square, color: Color) -> Option<Move> {
        use crate::eval::material_value;
        self.board.all_attacks_to(to, color)
        .into_iter()
        .min_by_key(|am| {
            let kind = am.promotion.unwrap_or(self.board.get_piece(am.from).unwrap().kind);
            material_value(kind)
        })
    }

    #[inline(never)]
    pub fn cheapest_attack_to(&self, to: Square, color: Color) -> Option<Move> {
        match color {
            Color::White => if to.0 < 56 {
                let bit = 1u64 << to.0;
                let froms = self.obs.white & self.obs.pawns & (
                    (bit & 0xfefefefefefefefe) >> 9 |
                    (bit & 0x7f7f7f7f7f7f7f7f) >> 7);
                if froms != 0 {
                    let from = Square(froms.trailing_zeros() as i8);
                    return Some(Move { from, to, promotion: None });
                }
            }
            Color::Black => if to.0 >= 8 {
                let bit = 1u64 << to.0;
                let froms = self.obs.black & self.obs.pawns & (
                    (bit & 0xfefefefefefefefe) << 7 |
                    (bit & 0x7f7f7f7f7f7f7f7f) << 9);
                if froms != 0 {
                    let from = Square(froms.trailing_zeros() as i8);
                    return Some(Move { from, to, promotion: None });
                }
            }
        }
        let my_pieces = match color {
            Color::White => self.obs.white,
            Color::Black => self.obs.black,
        };
        use crate::bitboard::*;
        let knights = my_pieces & self.obs.knights & KNIGHT_ATTACKS[to.0 as usize];
        if knights != 0 {
            let from = Square(knights.trailing_zeros() as i8);
            return Some(Move { from, to, promotion: None });
        }

        let occ = self.obs.white | self.obs.black;

        let bishops = my_pieces & self.obs.bishops & BISHOP_ATTACKS[to.0 as usize];
        for from in iter_one_positions(bishops) {
            if occ & IN_BETWEEN[to.0 as usize * 64 + from as usize] == 0 {
                return Some(Move { from: Square(from as i8), to, promotion: None });
            }
        }

        let rooks = my_pieces & self.obs.rooks & ROOK_ATTACKS[to.0 as usize];
        for from in iter_one_positions(rooks) {
            if occ & IN_BETWEEN[to.0 as usize * 64 + from as usize] == 0 {
                return Some(Move { from: Square(from as i8), to, promotion: None });
            }
        }

        let queens = my_pieces & self.obs.queens & (BISHOP_ATTACKS[to.0 as usize] | ROOK_ATTACKS[to.0 as usize]);
        for from in iter_one_positions(queens) {
            if occ & IN_BETWEEN[to.0 as usize * 64 + from as usize] == 0 {
                return Some(Move { from: Square(from as i8), to, promotion: None });
            }
        }

        match color {
            Color::White => if to.0 >= 56 {
                let bit = 1u64 << to.0;
                let froms = self.obs.white & self.obs.pawns & (
                    (bit & 0xfefefefefefefefe) >> 9 |
                    (bit & 0x7f7f7f7f7f7f7f7f) >> 7);
                if froms != 0 {
                    let from = Square(froms.trailing_zeros() as i8);
                    return Some(Move { from, to, promotion: Some(PieceKind::Queen) });
                }
            }
            Color::Black => if to.0 < 8 {
                let bit = 1u64 << to.0;
                let froms = self.obs.black & self.obs.pawns & (
                    (bit & 0xfefefefefefefefe) << 7 |
                    (bit & 0x7f7f7f7f7f7f7f7f) << 9);
                if froms != 0 {
                    let from = Square(froms.trailing_zeros() as i8);
                    return Some(Move { from, to, promotion: Some(PieceKind::Queen) });
                }
            }
        }

        let kings = my_pieces & self.obs.kings & KING_ATTACKS[to.0 as usize];
        if kings != 0 {
            let from = Square(kings.trailing_zeros() as i8);
            return Some(Move { from, to, promotion: None });
        }

        None
    }

    pub fn can_attack_to_for_testing(&self, to: Square, color: Color) -> bool {
        let res = self.can_attack_to(to, color);
        assert_eq!(res, self.cheapest_attack_to(to, color).is_some());
        res
    }

    #[inline(never)]
    pub fn can_attack_to(&self, to: Square, color: Color) -> bool {
        match color {
            Color::White => {
                let bit = 1u64 << to.0;
                let froms = self.obs.white & self.obs.pawns & (
                    (bit & 0xfefefefefefefefe) >> 9 |
                    (bit & 0x7f7f7f7f7f7f7f7f) >> 7);
                if froms != 0 {
                    return true;
                }
            }
            Color::Black => {
                let bit = 1u64 << to.0;
                let froms = self.obs.black & self.obs.pawns & (
                    (bit & 0xfefefefefefefefe) << 7 |
                    (bit & 0x7f7f7f7f7f7f7f7f) << 9);
                if froms != 0 {
                    return true;
                }
            }
        }

        let my_pieces = match color {
            Color::White => self.obs.white,
            Color::Black => self.obs.black,
        };
        use crate::bitboard::*;
        let knights = my_pieces & self.obs.knights & KNIGHT_ATTACKS[to.0 as usize];
        if knights != 0 {
            return true;
        }

        let occ = self.obs.white | self.obs.black;

        let sliding_attackers = my_pieces & (
            (self.obs.bishops | self.obs.queens) & BISHOP_ATTACKS[to.0 as usize] |
            (self.obs.rooks | self.obs.queens) & ROOK_ATTACKS[to.0 as usize]);
        for from in iter_one_positions(sliding_attackers) {
            if occ & IN_BETWEEN[to.0 as usize * 64 + from as usize] == 0 {
                return true;
            }
        }

        let kings = my_pieces & self.obs.kings & KING_ATTACKS[to.0 as usize];
        if kings != 0 {
            return true;
        }

        false
    }

    #[inline(never)]
    pub fn all_moves(&self) -> Vec<Move> {
        let mut result = Vec::with_capacity(128);
        let color = self.board.side_to_play();
        let (my_pieces, opp_pieces) = match color {
            Color::White => (self.obs.white, self.obs.black),
            Color::Black => (self.obs.black, self.obs.white),
        };
        let occ = self.obs.white | self.obs.black;
        use crate::bitboard::*;
        match color {
            Color::White => {
                let froms = my_pieces & self.obs.pawns & (!occ >> 8);
                for from in iter_one_squares(froms & 0xff00ffff_ffffffff) {
                    let to = Square(from.0 + 8);
                    result.push(Move { from, to, promotion: None });
                }
                for from in iter_one_squares(froms & 0x00ff0000_00000000) {
                    let to = Square(from.0 + 8);
                    for &promotion in crate::moves::PROMOTION_TARGETS {
                        result.push(Move { from, to, promotion });
                    }
                }
                let froms = froms & 0x00000000_0000ff00 & (!occ >> 16);
                for from in iter_one_squares(froms) {
                    let to = Square(from.0 + 16);
                    result.push(Move { from, to, promotion: None });
                }

                let froms = my_pieces & self.obs.pawns & 0xfefefefe_fefefefe & (opp_pieces >> 7);
                for from in iter_one_squares(froms & 0xff00ffff_ffffffff) {
                    let to = Square(from.0 + 7);
                    result.push(Move { from, to, promotion: None });
                }
                for from in iter_one_squares(froms & 0x00ff0000_00000000) {
                    let to = Square(from.0 + 7);
                    for &promotion in crate::moves::PROMOTION_TARGETS {
                        result.push(Move { from, to, promotion });
                    }
                }
                let froms = my_pieces & self.obs.pawns & 0x7f7f7f7f_7f7f7f7f & (opp_pieces >> 9);
                for from in iter_one_squares(froms & 0xff00ffff_ffffffff) {
                    let to = Square(from.0 + 9);
                    result.push(Move { from, to, promotion: None });
                }
                for from in iter_one_squares(froms & 0x00ff0000_00000000) {
                    let to = Square(from.0 + 9);
                    for &promotion in crate::moves::PROMOTION_TARGETS {
                        result.push(Move { from, to, promotion });
                    }
                }

                if let Some(ep) = self.board.en_passant_square {
                    let bit = 1u64 << ep.0;
                    let froms = my_pieces & self.obs.pawns & (
                        (bit & 0xfefefefefefefefe) >> 9 |
                        (bit & 0x7f7f7f7f7f7f7f7f) >> 7);
                    for from in iter_one_squares(froms) {
                        result.push(Move { from, to: ep, promotion: None });
                    }
                }
            }
            Color::Black => {
                let froms = my_pieces & self.obs.pawns & (!occ << 8);
                for from in iter_one_squares(froms & 0xffffffff_ffff00ff) {
                    let to = Square(from.0 - 8);
                    result.push(Move { from, to, promotion: None });
                }
                for from in iter_one_squares(froms & 0x00000000_0000ff00) {
                    let to = Square(from.0 - 8);
                    for &promotion in crate::moves::PROMOTION_TARGETS {
                        result.push(Move { from, to, promotion });
                    }
                }
                let froms = froms & 0x00ff0000_00000000 & (!occ << 16);
                for from in iter_one_squares(froms) {
                    let to = Square(from.0 - 16);
                    result.push(Move { from, to, promotion: None });
                }

                let froms = my_pieces & self.obs.pawns & 0xfefefefe_fefefefe & (opp_pieces << 9);
                for from in iter_one_squares(froms & 0xffffffff_ffff00ff) {
                    let to = Square(from.0 - 9);
                    result.push(Move { from, to, promotion: None });
                }
                for from in iter_one_squares(froms & 0x00000000_0000ff00) {
                    let to = Square(from.0 - 9);
                    for &promotion in crate::moves::PROMOTION_TARGETS {
                        result.push(Move { from, to, promotion });
                    }
                }
                let froms = my_pieces & self.obs.pawns & 0x7f7f7f7f_7f7f7f7f & (opp_pieces << 7);
                for from in iter_one_squares(froms & 0xffffffff_ffff00ff) {
                    let to = Square(from.0 - 7);
                    result.push(Move { from, to, promotion: None });
                }
                for from in iter_one_squares(froms & 0x00000000_0000ff00) {
                    let to = Square(from.0 - 7);
                    for &promotion in crate::moves::PROMOTION_TARGETS {
                        result.push(Move { from, to, promotion });
                    }
                }

                if let Some(ep) = self.board.en_passant_square {
                    let bit = 1u64 << ep.0;
                    let froms = my_pieces & self.obs.pawns & (
                        (bit & 0xfefefefefefefefe) << 7 |
                        (bit & 0x7f7f7f7f7f7f7f7f) << 9);
                    for from in iter_one_squares(froms) {
                        result.push(Move { from, to: ep, promotion: None });
                    }
                }
            }
        }

        // https://www.chessprogramming.org/Blockers_and_Beyond
        for from in iter_one_squares(my_pieces & !self.obs.pawns) {
            let (mut ts, mut b) = match self.board.get_piece(from).unwrap().kind {
                PieceKind::Pawn => unreachable!(),
                PieceKind::Knight => (KNIGHT_ATTACKS[from.0 as usize], 0),
                PieceKind::King => (KING_ATTACKS[from.0 as usize], 0),
                PieceKind::Bishop => (
                    BISHOP_ATTACKS[from.0 as usize],
                    BISHOP_BLOCKERS_AND_BEYOND[from.0 as usize]),
                PieceKind::Rook => (
                    ROOK_ATTACKS[from.0 as usize],
                    ROOK_BLOCKERS_AND_BEYOND[from.0 as usize]),
                PieceKind::Queen => (
                    BISHOP_ATTACKS[from.0 as usize] | ROOK_ATTACKS[from.0 as usize],
                    QUEEN_BLOCKERS_AND_BEYOND[from.0 as usize]),
            };
            ts &= !my_pieces;
            b &= occ;
            while b != 0 {
                let sq = (b & b.wrapping_neg()).trailing_zeros();
                b &= b - 1;
                let behind = BEHIND[from.0 as usize * 64 + sq as usize];
                ts &= !behind;
                b &= !behind;
            }
            for to in iter_one_squares(ts) {
                result.push(Move { from, to, promotion: None });
            }
        }

        match color {
            Color::White => {
                if self.board.flags.contains(BoardFlags::WHITE_CAN_OO) &&
                   occ & 0b0110_0000 == 0{
                    result.push(Move { from: Square(4), to: Square(6), promotion: None });
                }
                if self.board.flags.contains(BoardFlags::WHITE_CAN_OOO) &&
                   occ & 0b0000_1110 == 0 {
                    result.push(Move { from: Square(4), to: Square(2), promotion: None });
                }
            }
            Color::Black => {
                if self.board.flags.contains(BoardFlags::BLACK_CAN_OO) &&
                   occ & (0b0110_0000 << 56) == 0{
                    result.push(Move { from: Square(56 + 4), to: Square(56 + 6), promotion: None });
                }
                if self.board.flags.contains(BoardFlags::BLACK_CAN_OOO) &&
                   occ & (0b0000_1110 << 56) == 0 {
                    result.push(Move { from: Square(56 + 4), to: Square(56 + 2), promotion: None });
                }
            }
        }

        result
    }

    #[inline(never)]
    pub fn mobility(&self, color: Color) -> i32 {
        let my_pieces = match color {
            Color::White => self.obs.white,
            Color::Black => self.obs.black,
        };
        let occ = self.obs.white | self.obs.black;
        use crate::bitboard::*;

        let mut result = 0;
        // https://www.chessprogramming.org/Blockers_and_Beyond
        for from in iter_one_squares(my_pieces & !self.obs.pawns) {
            let kind = self.board.get_piece(from).unwrap().kind;
            let (mut ts, mut b) = match kind {
                PieceKind::Pawn => unreachable!(),
                PieceKind::Knight => (KNIGHT_ATTACKS[from.0 as usize], 0),
                PieceKind::King => (KING_ATTACKS[from.0 as usize], 0),
                PieceKind::Bishop => (
                    BISHOP_ATTACKS[from.0 as usize],
                    BISHOP_BLOCKERS_AND_BEYOND[from.0 as usize]),
                PieceKind::Rook => (
                    ROOK_ATTACKS[from.0 as usize],
                    ROOK_BLOCKERS_AND_BEYOND[from.0 as usize]),
                PieceKind::Queen => (
                    BISHOP_ATTACKS[from.0 as usize] | ROOK_ATTACKS[from.0 as usize],
                    QUEEN_BLOCKERS_AND_BEYOND[from.0 as usize]),
            };
            b &= occ;
            while b != 0 {
                let sq = (b & b.wrapping_neg()).trailing_zeros();
                b &= b - 1;
                let behind = BEHIND[from.0 as usize * 64 + sq as usize];
                ts &= !behind;
                b &= !behind;
            }
            result += ts.count_ones() as i32 * crate::eval::mobility_value(kind);
        }
        result
    }
}
