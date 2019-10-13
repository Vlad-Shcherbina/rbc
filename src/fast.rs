use log::info;
use crate::game::{Square, Color, PieceKind, Piece, BoardFlags, BoardState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(u32);

impl Move {
    fn new(
        from: u32, to: u32,
        from_kind: u32, to_kind: u32,
        cap: u32,
        ep_file: u32,
    ) -> Move {
        debug_assert!(from < 64);
        debug_assert!(to < 64);
        debug_assert!(from_kind < 6);
        debug_assert!(to_kind < 6);
        debug_assert!(ep_file <= 8);
        debug_assert!(cap <= 6);
        Move(
            from |
            to << 6 |
            from_kind << 12 |
            to_kind << 15 |
            cap << 18 |
            ep_file << 21 |
            0)
    }
    fn from(self) -> u32 {
        self.0 & 63
    }
    fn to(self) -> u32 {
        (self.0 >> 6) & 63
    }
    pub fn to_sq(self) -> Square {
        Square(self.to() as i8)
    }
    fn from_kind(self) -> u32 {
        (self.0 >> 12) & 7
    }
    fn to_kind(self) -> u32 {
        (self.0 >> 15) & 7
    }
    fn cap(self) -> u32 {
        (self.0 >> 18) & 7
    }
    fn ep_file(self) -> u32 {
        (self.0 >> 21) & 15
    }

    pub fn null() -> Move {
        Move::new(1, 1, 0, 0, 0, 8)
    }

    fn from_simple_move(m: Option<crate::game::Move>, s: &State) -> Move {
        let m = match m {
            Some(m) => m,
            None => return Move::null(),
        };
        let from_kind = s.get_opt_kind(m.from.0);
        assert!(from_kind > 0);
        let from_kind = from_kind - 1;
        let to_kind = m.promotion.map_or(from_kind, |k| k as u32);
        let cap = s.get_opt_kind(m.to.0);
        let mut ep_file = 8;
        if from_kind == PieceKind::Pawn as u32 {
            let c = 1 - s.side_to_play_();
            let epf = m.from.0 & 7;
            if (m.to.0 - m.from.0).abs() == 16 {
                let mask = ((0b101 << (epf + 3 * 8 - 1)) & 0x000000_ff000000)
                    << ((1 - c as i8) * 8);
                if s.by_color[c as usize] & s.by_kind[0] & mask != 0 {
                    ep_file = epf as u32;
                }
            }
        }
        Move::new(
            m.from.0 as u32,
            m.to.0 as u32,
            from_kind,
            to_kind,
            cap,
            ep_file,
        )
    }

    pub fn to_simple_move(self) -> Option<crate::game::Move> {
        if self == Move::null() {
            return None;
        }
        let promotion = if self.from_kind() == self.to_kind() {
            None
        } else {
            Some(PieceKind::from_int(self.to_kind()))
        };
        Some(crate::game::Move {
            from: Square(self.from() as i8),
            to: Square(self.to() as i8),
            promotion,
        })
    }
}

impl std::fmt::Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Move")
            .field("from", &self.from())
            .field("to", &self.to())
            .field("from_kind", &self.from_kind())
            .field("to_kind", &self.to_kind())
            .field("cap", &self.cap())
            .field("ep_file", &self.ep_file())
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct State {
    by_color: [u64; 2],
    by_kind: [u64; 6],
    flags: u8, // bits 0..3: KQkq, bit 4: black to play
    ep_file: u8,  // or 8 if no ep
    hash: u64,
}

impl From<&BoardState> for State {
    fn from(b: &BoardState) -> State {
        let mut result = State::empty();
        for sq in (0..64).map(Square) {
            if let Some(p) = b.get_piece(sq) {
                let bit = 1 << sq.0;
                result.by_color[p.color as usize] |= bit;
                result.by_kind[p.kind as usize] |= bit;
            }
        }
        result.ep_file = 8;
        if let Some(ep) = b.en_passant_square {
            let c = b.side_to_play();
            let mask = ((0b101 << ((ep.0 & 7) + 3 * 8 - 1)) & 0x000000_ff000000)
                << ((1 - c as i8) * 8);
            if result.by_color[c as usize] & result.by_kind[0] & mask != 0 {
                result.ep_file = (ep.0 & 7) as u8;
            }
        }
        if !b.flags.contains(BoardFlags::WHITE_TO_PLAY) {
            result.flags |= 16;
        }
        if b.flags.contains(BoardFlags::WHITE_CAN_OO) {
            result.flags |= 1;
        }
        if b.flags.contains(BoardFlags::WHITE_CAN_OOO) {
            result.flags |= 2;
        }
        if b.flags.contains(BoardFlags::BLACK_CAN_OO) {
            result.flags |= 4;
        }
        if b.flags.contains(BoardFlags::BLACK_CAN_OOO) {
            result.flags |= 8;
        }
        result.hash = result.recompute_hash();
        debug_assert!(result.check());
        result
    }
}

impl From<&State> for BoardState {
    fn from(s: &State) -> BoardState {
        let mut result = BoardState::empty();
        for sq in 0..64 {
            let bit = 1 << sq;
            if (s.by_color[0] | s.by_color[1]) & bit == 0 {
                continue;
            }
            let color = if s.by_color[0] & bit != 0 { Color::White } else { Color::Black };
            let kind = PieceKind::from_int(s.get_kind(sq));
            result.replace_piece(Square(sq), Some(Piece { color, kind }));
        }
        if s.flags & 16 == 0 { result.flags |= BoardFlags::WHITE_TO_PLAY; }
        if s.flags & 1 != 0 { result.flags |= BoardFlags::WHITE_CAN_OO; }
        if s.flags & 2 != 0 { result.flags |= BoardFlags::WHITE_CAN_OOO; }
        if s.flags & 4 != 0 { result.flags |= BoardFlags::BLACK_CAN_OO; }
        if s.flags & 8 != 0 { result.flags |= BoardFlags::BLACK_CAN_OOO; }
        if s.ep_file != 8 {
            if s.flags & 16 == 0 {
                result.en_passant_square = Some(Square(5 * 8 + s.ep_file as i8));
            } else {
                result.en_passant_square = Some(Square(2 * 8 + s.ep_file as i8));
            }
        }
        result
    }
}

impl State {
    pub fn empty() -> State {
        State {
            by_color: [0, 0],
            by_kind: [0, 0, 0, 0, 0, 0],
            flags: 0,
            ep_file: 0,
            hash: 0,
        }
    }

    fn side_to_play_(&self) -> u8 {
        self.flags >> 4
    }

    pub fn side_to_play(&self) -> Color {
        match self.side_to_play_() {
            0 => Color::White,
            1 => Color::Black,
            _ => unreachable!(),
        }
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub fn get_piece(&self, sq: Square) -> Option<Piece> {
        let bit = 1 << sq.0;
        let color = if self.by_color[0] & bit != 0 {
            Color::White
        } else if self.by_color[1] & bit != 0 {
            Color::Black
        } else {
            return None;
        };
        let kind = self.get_opt_kind(sq.0);
        assert!(kind > 0);
        let kind = PieceKind::from_int(kind - 1);
        Some(Piece { color, kind })
    }

    pub fn find_king(&self, color: Color) -> Option<Square> {
        let ks = self.by_color[color as usize] & self.by_kind[PieceKind::King as usize];
        if ks == 0 {
            None
        } else {
            Some(Square(ks.trailing_zeros() as i8))
        }
    }

    fn get_kind(&self, sq: i8) -> u32 {
        debug_assert!(((self.by_color[0] | self.by_color[1]) >> sq) & 1 != 0);
        let kind =
            (((self.by_kind[1] | self.by_kind[3] | self.by_kind[5]) >> sq) & 1) +
            (((self.by_kind[2] | self.by_kind[3]) >> sq) & 1) * 2 +
            (((self.by_kind[4] | self.by_kind[5]) >> sq) & 1) * 4;
        debug_assert!(kind < 6);
        kind as u32
    }

    // 0 or kind + 1
    fn get_opt_kind(&self, sq: i8) -> u32 {
        let kind =
            (((self.by_kind[0] | self.by_kind[2] | self.by_kind[4]) >> sq) & 1) +
            (((self.by_kind[1] | self.by_kind[2] | self.by_kind[5]) >> sq) & 1) * 2 +
            (((self.by_kind[3] | self.by_kind[4] | self.by_kind[5]) >> sq) & 1) * 4;
        debug_assert!(kind <= 6);
        kind as u32
    }

    fn recompute_hash(&self) -> u64 {
        let pre: &Precomputed = &PRECOMPUTED;
        let mut result = 0;
        for sq in 0..64 {
            let bit = 1 << sq;
            if (self.by_color[0] | self.by_color[1]) & bit == 0 {
                continue;
            }
            let color = (self.by_color[1] & bit) >> sq;
            let kind = self.get_kind(sq);
            result ^= pre.zobrist[(kind as usize * 2 + color as usize) * 64 + sq as usize];
        }
        result ^= self.side_to_play_() as u64 * pre.zobrist_black_to_play;
        result ^= pre.zobrist_castling[(self.flags & 15) as usize];
        result ^= pre.zobrist_ep[self.ep_file as usize];
        result
    }

    pub fn check(&self) -> bool {
        assert!(self.by_color[0] & self.by_color[1] == 0);
        let mut occ = 0;
        for i in 0..6 {
            occ |= self.by_kind[i];
            for j in 0..i {
                assert!(self.by_kind[i] & self.by_kind[j] == 0);
            }
        }
        assert_eq!(occ, self.by_color[0] | self.by_color[1]);
        assert_eq!(self.hash, self.recompute_hash());
        true
    }

    #[inline(never)]
    pub fn make_move(&mut self, m: Move, undo_log: &mut Vec<UndoEntry>) {
        undo_log.push(UndoEntry {
            hash: self.hash,
            flags: self.flags,
            ep_file: self.ep_file,
        });
        let pre: &Precomputed = &PRECOMPUTED;
        self.hash ^= pre.zobrist_ep[self.ep_file as usize];
        self.hash ^= pre.zobrist_castling[(self.flags & 15) as usize];
        let from_bit = 1 << m.from();
        let to_bit = 1 << m.to();
        let c = self.side_to_play_();
        self.by_color[c as usize] ^= from_bit ^ to_bit;
        self.by_kind[m.from_kind() as usize] ^= from_bit;
        self.by_kind[m.to_kind() as usize] ^= to_bit;
        self.hash ^= pre.zobrist[(m.from_kind() as usize * 2 + c as usize) * 64 + m.from() as usize];
        self.hash ^= pre.zobrist[(m.to_kind() as usize * 2 + c as usize) * 64 + m.to() as usize];

        let cap = m.cap();
        if cap != 0 {
            self.by_color[1 - c as usize] ^= to_bit;
            self.by_kind[cap as usize - 1] ^= to_bit;
            self.hash ^= pre.zobrist[((cap as usize - 1) * 2 + 1 - c as usize) * 64 + m.to() as usize];
        } else {
            if m.from_kind() == 5 {
                let from_to = m.0 & 0b111111_111111;
                if from_to == 0b000110_000100 {  // White OO
                    debug_assert_eq!(c, 0);
                    self.by_color[0] ^= 1 << 7 | 1 << 5;
                    let r = PieceKind::Rook as usize;
                    self.by_kind[r] ^= 1 << 7 | 1 << 5;
                    self.hash ^= pre.zobrist[(r * 2 + 0) * 64 + 7] ^
                                 pre.zobrist[(r * 2 + 0) * 64 + 5];
                } else if from_to == 0b111110_111100 {  // Black OO
                    debug_assert_eq!(c, 1);
                    self.by_color[1] ^= 1 << 56 + 7 | 1 << 56 + 5;
                    let r = PieceKind::Rook as usize;
                    self.by_kind[r] ^= 1 << 56 + 7 | 1 << 56 + 5;
                    self.hash ^= pre.zobrist[(r * 2 + 1) * 64 + 56 + 7] ^
                                 pre.zobrist[(r * 2 + 1) * 64 + 56 + 5];
                } else if from_to == 0b000010_000100 {  // White OOO
                    debug_assert_eq!(c, 0);
                    self.by_color[0] ^= 1 << 0 | 1 << 3;
                    let r = PieceKind::Rook as usize;
                    self.by_kind[r] ^= 1 << 0 | 1 << 3;
                    self.hash ^= pre.zobrist[(r * 2 + 0) * 64 + 0] ^
                                 pre.zobrist[(r * 2 + 0) * 64 + 3];
                } else if from_to == 0b111010_111100 {  // Black OOO
                    debug_assert_eq!(c, 1);
                    self.by_color[1] ^= 1 << 56 + 0 | 1 << 56 + 3;
                    let r = PieceKind::Rook as usize;
                    self.by_kind[r] ^= 1 << 56 + 0 | 1 << 56 + 3;
                    self.hash ^= pre.zobrist[(r * 2 + 1) * 64 + 56 + 0] ^
                                 pre.zobrist[(r * 2 + 1) * 64 + 56 + 3];
                }
            } else if self.ep_file < 8 {
                let t = ((m.0 >> 6) & 0b111_111111) + ((c as u32) << 9);
                if t == 0b0_000_101000 + self.ep_file as u32 {
                    // white ep capture
                    self.by_color[1] ^= 1 << 4 * 8 + self.ep_file;
                    self.by_kind[0] ^= 1 << 4 * 8 + self.ep_file;
                    self.hash ^= pre.zobrist[1 * 64 + 4 * 8 + self.ep_file as usize];
                } else if t == 0b1_000_010000 + self.ep_file as u32 {
                    // black ep capture
                    self.by_color[0] ^= 1 << 3 * 8 + self.ep_file;
                    self.by_kind[0] ^= 1 << 3 * 8 + self.ep_file;
                    self.hash ^= pre.zobrist[0 * 64 + 3 * 8 + self.ep_file as usize];
                }
            }
        }

        const CASTLING_FILTER: [u8; 64] = [
            !2, !0, !0, !0, !3, !0, !0, !1,
            !0, !0, !0, !0, !0, !0, !0, !0,
            !0, !0, !0, !0, !0, !0, !0, !0,
            !0, !0, !0, !0, !0, !0, !0, !0,
            !0, !0, !0, !0, !0, !0, !0, !0,
            !0, !0, !0, !0, !0, !0, !0, !0,
            !0, !0, !0, !0, !0, !0, !0, !0,
            !8, !0, !0, !0, !12, !0, !0, !4,
        ];
        self.flags &= CASTLING_FILTER[m.from() as usize] & CASTLING_FILTER[m.to() as usize];

        self.ep_file = m.ep_file() as u8;
        self.hash ^= pre.zobrist_ep[self.ep_file as usize];
        self.flags ^= 16;
        self.hash ^= pre.zobrist_black_to_play;
        self.hash ^= pre.zobrist_castling[(self.flags & 15) as usize];

        debug_assert_eq!(self.hash, self.recompute_hash());
    }

    #[inline(never)]
    pub fn unmake_move(&mut self, m: Move, undo_log: &mut Vec<UndoEntry>) {
        let u = undo_log.pop().unwrap();
        self.hash = u.hash;
        self.flags = u.flags;
        self.ep_file = u.ep_file;

        let from_bit = 1 << m.from();
        let to_bit = 1 << m.to();
        let c = self.side_to_play_();
        self.by_color[c as usize] ^= from_bit ^ to_bit;
        self.by_kind[m.from_kind() as usize] ^= from_bit;
        self.by_kind[m.to_kind() as usize] ^= to_bit;

        let cap = m.cap();
        if cap != 0 {
            self.by_color[1 - c as usize] ^= to_bit;
            self.by_kind[cap as usize - 1] ^= to_bit;
        } else {
            if m.from_kind() == 5 {
                let from_to = m.0 & 0b111111_111111;
                if from_to == 0b000110_000100 {  // White OO
                    debug_assert_eq!(c, 0);
                    self.by_color[0] ^= 1 << 7 | 1 << 5;
                    self.by_kind[PieceKind::Rook as usize] ^= 1 << 7 | 1 << 5;
                } else if from_to == 0b111110_111100 {  // Black OO
                    debug_assert_eq!(c, 1);
                    self.by_color[1] ^= 1 << 56 + 7 | 1 << 56 + 5;
                    self.by_kind[PieceKind::Rook as usize] ^= 1 << 56 + 7 | 1 << 56 + 5;
                } else if from_to == 0b000010_000100 {  // White OOO
                    debug_assert_eq!(c, 0);
                    self.by_color[0] ^= 1 << 0 | 1 << 3;
                    self.by_kind[PieceKind::Rook as usize] ^= 1 << 0 | 1 << 3;
                } else if from_to == 0b111010_111100 {  // Black OOO
                    debug_assert_eq!(c, 1);
                    self.by_color[1] ^= 1 << 56 + 0 | 1 << 56 + 3;
                    self.by_kind[PieceKind::Rook as usize] ^= 1 << 56 + 0 | 1 << 56 + 3;
                }
            } else if self.ep_file < 8 {
                if ((m.0 >> 6) & 0b111_111111) + ((c as u32) << 9) == 0b0_000_101000 + self.ep_file as u32 {
                    // white ep capture
                    self.by_color[1] ^= 1 << 4 * 8 + self.ep_file;
                    self.by_kind[0] ^= 1 << 4 * 8 + self.ep_file;
                } else if ((m.0 >> 6) & 0b111_111111) + ((c as u32) << 9) == 0b1_000_010000 + self.ep_file as u32 {
                    // black ep capture
                    self.by_color[0] ^= 1 << 3 * 8 + self.ep_file;
                    self.by_kind[0] ^= 1 << 3 * 8 + self.ep_file;
                }
            }
        }
    }

    #[inline(never)]
    pub fn all_moves(&self, result: &mut Vec<Move>) {
        let occ = self.by_color[0] | self.by_color[1];
        if self.side_to_play_() == 0 {
            // pushes
            let my_pawns = self.by_color[0] & self.by_kind[0];
            let froms = my_pawns & (!occ >> 8);
            for from in iter_one_positions(froms & 0xff00ffff_ffffffff) {
                result.push(Move::new(from, from + 8, 0, 0, 0, 8));
            }
            for from in iter_one_positions(froms & 0x00ff0000_00000000) {
                for promo in 1..5 {
                    result.push(Move::new(from, from + 8, 0, promo, 0, 8));
                }
            }

            // double pushes
            let froms = froms & (!occ >> 16) & 0x000000_0000ff00;
            let opp_pawns = self.by_color[1] & self.by_kind[0];
            let ep_threat =
                (opp_pawns & 0xfefefefe_fefefefe) >> 9 | (opp_pawns & 0x7f7f7f7f_7f7f7f7f) >> 7;
            let ep_threat = ep_threat >> 8;
            for from in iter_one_positions(froms & !ep_threat) {
                result.push(Move::new(from, from + 16, 0, 0, 0, 8));
            }
            for from in iter_one_positions(froms & ep_threat) {
                result.push(Move::new(from, from + 16, 0, 0, 0, from & 7));
            }

            let targets = self.by_color[1] | (1 << self.ep_file & 0xff) << 5 * 8;

            // captures right
            let froms = my_pawns & (targets & 0xfefefefe_fefefefe) >> 9;
            for from in iter_one_positions(froms & 0xff00ffff_ffffffff) {
                let cap = self.get_opt_kind(from as i8 + 9);
                result.push(Move::new(from, from + 9, 0, 0, cap, 8));
            }
            for from in iter_one_positions(froms & 0x00ff0000_00000000) {
                let cap = self.get_opt_kind(from as i8 + 9);
                debug_assert!(cap != 0);
                for promo in 1..5 {
                    result.push(Move::new(from, from + 9, 0, promo, cap, 8));
                }
            }

            // captures left
            let froms = my_pawns & (targets & 0x7f7f7f7f_7f7f7f7f) >> 7;
            for from in iter_one_positions(froms & 0xff00ffff_ffffffff) {
                let cap = self.get_opt_kind(from as i8 + 7);
                result.push(Move::new(from, from + 7, 0, 0, cap, 8));
            }
            for from in iter_one_positions(froms & 0x00ff0000_00000000) {
                let cap = self.get_opt_kind(from as i8 + 7);
                debug_assert!(cap != 0);
                for promo in 1..5 {
                    result.push(Move::new(from, from + 7, 0, promo, cap, 8));
                }
            }

            // castlings
            if self.flags & 1 != 0 && occ & 0b0110_0000 == 0 {
                result.push(Move::new(4, 6, 5, 5, 0, 8));
            }
            if self.flags & 2 != 0 && occ & 0b0000_1110 == 0 {
                result.push(Move::new(4, 2, 5, 5, 0, 8));
            }
        } else {
            // pushes
            let my_pawns = self.by_color[1] & self.by_kind[0];
            let froms = my_pawns & (!occ << 8);
            for from in iter_one_positions(froms & 0xffffffff_ffff00ff) {
                result.push(Move::new(from, from - 8, 0, 0, 0, 8));
            }
            for from in iter_one_positions(froms & 0x00000000_0000ff00) {
                for promo in 1..5 {
                    result.push(Move::new(from, from - 8, 0, promo, 0, 8));
                }
            }

            // double pushes
            let froms = froms & (!occ << 16) & 0x00ff0000_00000000;
            let opp_pawns = self.by_color[0] & self.by_kind[0];
            let ep_threat =
                (opp_pawns & 0xfefefefe_fefefefe) << 7 | (opp_pawns & 0x7f7f7f7f_7f7f7f7f) << 9;
            let ep_threat = ep_threat << 8;
            for from in iter_one_positions(froms & !ep_threat) {
                result.push(Move::new(from, from - 16, 0, 0, 0, 8));
            }
            for from in iter_one_positions(froms & ep_threat) {
                result.push(Move::new(from, from - 16, 0, 0, 0, from & 7));
            }

            let targets = self.by_color[0] | (1 << self.ep_file & 0xff) << 2 * 8;

            // captures right
            let froms = my_pawns & (targets & 0xfefefefe_fefefefe) << 7;
            for from in iter_one_positions(froms & 0xffffffff_ffff00ff) {
                let cap = self.get_opt_kind(from as i8 - 7);
                result.push(Move::new(from, from - 7, 0, 0, cap, 8));
            }
            for from in iter_one_positions(froms & 0x00000000_0000ff00) {
                let cap = self.get_opt_kind(from as i8 - 7);
                debug_assert!(cap != 0);
                for promo in 1..5 {
                    result.push(Move::new(from, from - 7, 0, promo, cap, 8));
                }
            }

            // captures left
            let froms = my_pawns & (targets & 0x7f7f7f7f_7f7f7f7f) << 9;
            for from in iter_one_positions(froms & 0xffffffff_ffff00ff) {
                let cap = self.get_opt_kind(from as i8 - 9);
                result.push(Move::new(from, from - 9, 0, 0, cap, 8));
            }
            for from in iter_one_positions(froms & 0x00000000_0000ff00) {
                let cap = self.get_opt_kind(from as i8 - 9);
                debug_assert!(cap != 0);
                for promo in 1..5 {
                    result.push(Move::new(from, from - 9, 0, promo, cap, 8));
                }
            }

            // castlings
            if self.flags & 4 != 0 && occ & 0b0110_0000 << 56 == 0 {
                result.push(Move::new(4 + 56, 6 + 56, 5, 5, 0, 8));
            }
            if self.flags & 8 != 0 && occ & 0b0000_1110 << 56 == 0 {
                result.push(Move::new(4 + 56, 2 + 56, 5, 5, 0, 8));
            }
        }

        let pre: &Precomputed = &PRECOMPUTED;
        let mine = self.by_color[self.side_to_play_() as usize];
        for from in iter_one_positions(mine & self.by_kind[PieceKind::Knight as usize]) {
            for to in iter_one_positions(pre.knight_attacks[from as usize] & !mine) {
                result.push(Move::new(
                    from, to,
                    PieceKind::Knight as u32, PieceKind::Knight as u32,
                    self.get_opt_kind(to as i8), 8));
            }
        }
        for from in iter_one_positions(mine & self.by_kind[PieceKind::King as usize]) {
            for to in iter_one_positions(pre.king_attacks[from as usize] & !mine) {
                result.push(Move::new(
                    from, to,
                    PieceKind::King as u32, PieceKind::King as u32,
                    self.get_opt_kind(to as i8), 8));
            }
        }

        for kind in 2..5 {
            for from in iter_one_positions(mine & self.by_kind[kind]) {
                let SlidingEntry {
                    attack: mut ts,
                    mask: mut b
                } = pre.sliding[(kind - 2) * 64 + from as usize];
                ts &= !mine;
                b &= occ;
                while b != 0 {
                    let sq = (b & b.wrapping_neg()).trailing_zeros();
                    b &= b - 1;
                    let behind = pre.behind[from as usize * 64 + sq as usize];
                    ts &= !behind;
                    b &= !behind;
                }
                for to in iter_one_positions(ts) {
                    result.push(Move::new(
                        from, to,
                        kind as u32, kind as u32,
                        self.get_opt_kind(to as i8), 8));
                }
            }
        }
    }

    #[inline(never)]
    pub fn all_attacks(&self, result: &mut Vec<Move>) {
        let targets = self.by_color[1 - self.side_to_play_() as usize];
        if self.side_to_play_() == 0 {
            let my_pawns = self.by_color[0] & self.by_kind[0];

            // captures right
            let froms = my_pawns & (targets & 0xfefefefe_fefefefe) >> 9;
            for from in iter_one_positions(froms & 0xff00ffff_ffffffff) {
                let cap = self.get_opt_kind(from as i8 + 9);
                debug_assert!(cap != 0);
                result.push(Move::new(from, from + 9, 0, 0, cap, 8));
            }
            for from in iter_one_positions(froms & 0x00ff0000_00000000) {
                let cap = self.get_opt_kind(from as i8 + 9);
                debug_assert!(cap != 0);
                result.push(Move::new(from, from + 9, 0, PieceKind::Queen as u32, cap, 8));
            }

            // captures left
            let froms = my_pawns & (targets & 0x7f7f7f7f_7f7f7f7f) >> 7;
            for from in iter_one_positions(froms & 0xff00ffff_ffffffff) {
                let cap = self.get_opt_kind(from as i8 + 7);
                debug_assert!(cap != 0);
                result.push(Move::new(from, from + 7, 0, 0, cap, 8));
            }
            for from in iter_one_positions(froms & 0x00ff0000_00000000) {
                let cap = self.get_opt_kind(from as i8 + 7);
                debug_assert!(cap != 0);
                result.push(Move::new(from, from + 7, 0, PieceKind::Queen as u32, cap, 8));
            }
        } else {
            let my_pawns = self.by_color[1] & self.by_kind[0];

            // captures right
            let froms = my_pawns & (targets & 0xfefefefe_fefefefe) << 7;
            for from in iter_one_positions(froms & 0xffffffff_ffff00ff) {
                let cap = self.get_opt_kind(from as i8 - 7);
                debug_assert!(cap != 0);
                result.push(Move::new(from, from - 7, 0, 0, cap, 8));
            }
            for from in iter_one_positions(froms & 0x00000000_0000ff00) {
                let cap = self.get_opt_kind(from as i8 - 7);
                debug_assert!(cap != 0);
                result.push(Move::new(from, from - 7, 0, PieceKind::Queen as u32, cap, 8));
            }

            // captures left
            let froms = my_pawns & (targets & 0x7f7f7f7f_7f7f7f7f) << 9;
            for from in iter_one_positions(froms & 0xffffffff_ffff00ff) {
                let cap = self.get_opt_kind(from as i8 - 9);
                debug_assert!(cap != 0);
                result.push(Move::new(from, from - 9, 0, 0, cap, 8));
            }
            for from in iter_one_positions(froms & 0x00000000_0000ff00) {
                let cap = self.get_opt_kind(from as i8 - 9);
                debug_assert!(cap != 0);
                result.push(Move::new(from, from - 9, 0, PieceKind::Queen as u32, cap, 8));
            }
        }

        let pre: &Precomputed = &PRECOMPUTED;
        let mine = self.by_color[self.side_to_play_() as usize];
        for from in iter_one_positions(mine & self.by_kind[PieceKind::Knight as usize]) {
            for to in iter_one_positions(pre.knight_attacks[from as usize] & targets) {
                let cap = self.get_opt_kind(to as i8);
                debug_assert!(cap != 0);
                result.push(Move::new(
                    from, to,
                    PieceKind::Knight as u32, PieceKind::Knight as u32,
                    cap, 8));
            }
        }
        for from in iter_one_positions(mine & self.by_kind[PieceKind::King as usize]) {
            for to in iter_one_positions(pre.king_attacks[from as usize] & targets) {
                let cap = self.get_opt_kind(to as i8);
                debug_assert!(cap != 0);
                result.push(Move::new(
                    from, to,
                    PieceKind::King as u32, PieceKind::King as u32,
                    cap, 8));
            }
        }

        let occ = self.by_color[0] | self.by_color[1];
        for kind in 2..5 {
            for from in iter_one_positions(mine & self.by_kind[kind]) {
                let SlidingEntry {
                    attack: mut ts,
                    mask: mut b
                } = pre.sliding[(kind - 2) * 64 + from as usize];
                ts &= targets;
                b &= occ;
                while b != 0 && ts != 0 {
                    let sq = (b & b.wrapping_neg()).trailing_zeros();
                    b &= b - 1;
                    let behind = pre.behind[from as usize * 64 + sq as usize];
                    ts &= !behind;
                    b &= !behind;
                }
                for to in iter_one_positions(ts) {
                    result.push(Move::new(
                        from, to,
                        kind as u32, kind as u32,
                        self.get_opt_kind(to as i8), 8));
                }
            }
        }
    }

    #[inline(never)]
    pub fn material(&self, color: Color) -> i32 {
        let mine = self.by_color[color as usize];
        (
            (mine & self.by_kind[PieceKind::Pawn as usize]).count_ones() * 100 +
            (mine & self.by_kind[PieceKind::Knight as usize]).count_ones() * 350 +
            (mine & self.by_kind[PieceKind::Bishop as usize]).count_ones() * 350 +
            (mine & self.by_kind[PieceKind::Rook as usize]).count_ones() * 525 +
            (mine & self.by_kind[PieceKind::Queen as usize]).count_ones() * 1000 +
            (mine & self.by_kind[PieceKind::King as usize]).count_ones() * 10000
        ) as i32
    }

    #[inline(never)]
    pub fn mobility(&self, color: Color) -> i32 {
        let color = color as usize;
        let pre: &Precomputed = &PRECOMPUTED;
        let mine = self.by_color[color];
        let occ = self.by_color[0] | self.by_color[1];
        let mut result = 0;

        for from in iter_one_positions(mine & self.by_kind[PieceKind::Knight as usize]) {
            let tos = pre.knight_attacks[from as usize];
            result += 3 * tos.count_ones();
        }
        for from in iter_one_positions(mine & self.by_kind[PieceKind::King as usize]) {
            let tos = pre.king_attacks[from as usize];
            result += tos.count_ones();
        }

        for kind in 2..5 {
            for from in iter_one_positions(mine & self.by_kind[kind]) {
                let SlidingEntry {
                    attack: mut ts,
                    mask: mut b
                } = pre.sliding[(kind - 2) * 64 + from as usize];
                b &= occ;
                while b != 0 {
                    let sq = (b & b.wrapping_neg()).trailing_zeros();
                    b &= b - 1;
                    let behind = pre.behind[from as usize * 64 + sq as usize];
                    ts &= !behind;
                    b &= !behind;
                }
                result += ts.count_ones() * match kind {
                    2 => 3,
                    3 => 2,
                    4 => 1,
                    _ => unreachable!(),
                };
            }
        }

        result as i32
    }

    #[inline(never)]
    pub fn can_attack_to(&self, to: Square, color: crate::game::Color) -> bool {
        let color = color as usize;
        let bit = 1u64 << to.0;
        if color == 0 {
            let froms = self.by_color[0] & self.by_kind[0] & (
                (bit & 0xfefefefefefefefe) >> 9 |
                (bit & 0x7f7f7f7f7f7f7f7f) >> 7);
            if froms != 0 {
                return true;
            }
        } else {
            let bit = 1u64 << to.0;
            let froms = self.by_color[1] & self.by_kind[0] & (
                (bit & 0xfefefefefefefefe) << 7 |
                (bit & 0x7f7f7f7f7f7f7f7f) << 9);
            if froms != 0 {
                return true;
            }
        }
        let pre: &Precomputed = &PRECOMPUTED;
        let mine = self.by_color[color];
        let knights = mine & self.by_kind[PieceKind::Knight as usize] & pre.knight_attacks[to.0 as usize];
        if knights != 0 {
            return true;
        }

        let occ = self.by_color[0] | self.by_color[1];
        let sliding_attackers = mine & (
            (self.by_kind[PieceKind::Bishop as usize] |
             self.by_kind[PieceKind::Queen as usize]) & pre.sliding[0 * 64 + to.0 as usize].attack |
            (self.by_kind[PieceKind::Rook as usize] |
             self.by_kind[PieceKind::Queen as usize]) & pre.sliding[1 * 64 + to.0 as usize].attack);
        for from in iter_one_positions(sliding_attackers) {
            if occ & pre.in_between[to.0 as usize * 64 + from as usize] == 0 {
                return true;
            }
        }

        let kings = mine & self.by_kind[PieceKind::King as usize] & pre.king_attacks[to.0 as usize];
        if kings != 0 {
            return true;
        }
        false
    }

    #[inline(never)]
    pub fn cheapest_attack_to(&self, to: Square, color: crate::game::Color) -> Option<Move> {
        let color = color as usize;
        if color == 0 {
            if to.0 < 56 {
                let bit = 1u64 << to.0;
                let froms = self.by_color[0] & self.by_kind[0] & (
                    (bit & 0xfefefefefefefefe) >> 9 |
                    (bit & 0x7f7f7f7f7f7f7f7f) >> 7);
                if froms != 0 {
                    let from = froms.trailing_zeros();
                    let cap = self.get_opt_kind(to.0);
                    debug_assert!(cap != 0);
                    return Some(Move::new(
                        from, to.0 as u32,
                        0, 0,
                        cap, 8));
                }
            }
        } else {
            if to.0 >= 8 {
                let bit = 1u64 << to.0;
                let froms = self.by_color[1] & self.by_kind[0] & (
                    (bit & 0xfefefefefefefefe) << 7 |
                    (bit & 0x7f7f7f7f7f7f7f7f) << 9);
                if froms != 0 {
                    let from = froms.trailing_zeros();
                    let cap = self.get_opt_kind(to.0);
                    debug_assert!(cap != 0);
                    return Some(Move::new(
                        from, to.0 as u32,
                        0, 0,
                        cap, 8));
                }
            }
        }
        let pre: &Precomputed = &PRECOMPUTED;
        let mine = self.by_color[color];

        let knights = mine & self.by_kind[PieceKind::Knight as usize] & pre.knight_attacks[to.0 as usize];
        if knights != 0 {
            let from = knights.trailing_zeros();
            let cap = self.get_opt_kind(to.0);
            debug_assert!(cap != 0);
            return Some(Move::new(
                from, to.0 as u32,
                PieceKind::Knight as u32, PieceKind::Knight as u32,
                cap, 8));
        }

        let occ = self.by_color[0] | self.by_color[1];

        let bishops = mine & self.by_kind[PieceKind::Bishop as usize] & pre.sliding[0 * 64 + to.0 as usize].attack;
        for from in iter_one_positions(bishops) {
            if occ & pre.in_between[to.0 as usize * 64 + from as usize] == 0 {
                let cap = self.get_opt_kind(to.0);
                debug_assert!(cap != 0);
                return Some(Move::new(
                    from, to.0 as u32,
                    PieceKind::Bishop as u32, PieceKind::Bishop as u32,
                    self.get_opt_kind(to.0), 8));
            }
        }

        let rooks = mine & self.by_kind[PieceKind::Rook as usize] & pre.sliding[1 * 64 + to.0 as usize].attack;
        for from in iter_one_positions(rooks) {
            if occ & pre.in_between[to.0 as usize * 64 + from as usize] == 0 {
                let cap = self.get_opt_kind(to.0);
                debug_assert!(cap != 0);
                return Some(Move::new(
                    from, to.0 as u32,
                    PieceKind::Rook as u32, PieceKind::Rook as u32,
                    self.get_opt_kind(to.0), 8));
            }
        }

        let queens = mine & self.by_kind[PieceKind::Queen as usize] & pre.sliding[2 * 64 + to.0 as usize].attack;
        for from in iter_one_positions(queens) {
            if occ & pre.in_between[to.0 as usize * 64 + from as usize] == 0 {
                let cap = self.get_opt_kind(to.0);
                debug_assert!(cap != 0);
                return Some(Move::new(
                    from, to.0 as u32,
                    PieceKind::Queen as u32, PieceKind::Queen as u32,
                    self.get_opt_kind(to.0), 8));
            }
        }

        if color == 0 {
            if to.0 >= 56 {
                let bit = 1u64 << to.0;
                let froms = self.by_color[0] & self.by_kind[0] & (
                    (bit & 0xfefefefefefefefe) >> 9 |
                    (bit & 0x7f7f7f7f7f7f7f7f) >> 7);
                if froms != 0 {
                    let from = froms.trailing_zeros();
                    let cap = self.get_opt_kind(to.0);
                    debug_assert!(cap != 0);
                    return Some(Move::new(
                        from, to.0 as u32,
                        0, PieceKind::Queen as u32,
                        cap, 8));
                }
            }
        } else {
            if to.0 < 8 {
                let bit = 1u64 << to.0;
                let froms = self.by_color[1] & self.by_kind[0] & (
                    (bit & 0xfefefefefefefefe) << 7 |
                    (bit & 0x7f7f7f7f7f7f7f7f) << 9);
                if froms != 0 {
                    let from = froms.trailing_zeros();
                    let cap = self.get_opt_kind(to.0);
                    debug_assert!(cap != 0);
                    return Some(Move::new(
                        from, to.0 as u32,
                        0, PieceKind::Queen as u32,
                        cap, 8));
                }
            }
        }

        let kings = mine & self.by_kind[PieceKind::King as usize] & pre.king_attacks[to.0 as usize];
        if kings != 0 {
            let from = kings.trailing_zeros();
            let cap = self.get_opt_kind(to.0);
            debug_assert!(cap != 0);
            return Some(Move::new(
                from, to.0 as u32,
                PieceKind::King as u32, PieceKind::King as u32,
                cap, 8));
        }
        None
    }
}

pub struct UndoEntry {
    hash: u64,
    flags: u8,
    ep_file: u8,
}

#[derive(Clone, Copy)]
struct SlidingEntry {
    attack: u64,
    mask: u64,
}

fn blockers_and_beyond(from: i8, deltas: &[(i8, i8)]) -> u64 {
    let rank = from / 8;
    let file = from % 8;
    let mut result = 0;
    for &(dr, df) in deltas {
        let mut r = rank + 2 * dr;
        let mut f = file + 2 * df;
        while 0 <= r && r < 8 && 0 <= f && f < 8 {
            result |= 1 << ((r - dr) * 8 + (f - df));
            r += dr;
            f += df;
        }
    }
    result
}

fn compute_behind(from: i8, to: i8) -> Option<u64> {
    if from == to {
        return None;
    }
    let dr = to / 8 - from / 8;
    let df = to % 8 - from % 8;
    if dr != 0 && df != 0 && dr.abs() != df.abs() {
        return None;
    }
    let dr = dr.signum();
    let df = df.signum();
    let mut result = 0;
    let mut r = to / 8 + dr;
    let mut f = to % 8 + df;
    while 0 <= r && r < 8 && 0 <= f && f < 8 {
        result |= 1 << (r * 8 + f);
        r += dr;
        f += df;
    }
    Some(result)
}

fn compute_in_between(from: i8, to: i8) -> Option<u64> {
    if from == to {
        return None;
    }
    let mut rank = from / 8;
    let mut file = from % 8;
    let dr = to / 8 - rank;
    let df = to % 8 - file;
    if dr != 0 && df != 0 && dr.abs() != df.abs() {
        return None;
    }
    let dr = dr.signum();
    let df = df.signum();
    let mut result = 0;
    loop {
        rank += dr;
        file += df;
        let pos = rank * 8 + file;
        if pos == to {
            break;
        }
        result |= 1 << pos;
    }
    Some(result)
}

struct Precomputed {
    zobrist: [u64; 6 * 2 * 64],
    zobrist_castling: [u64; 16],
    zobrist_black_to_play: u64,
    zobrist_ep: [u64; 9],

    knight_attacks: [u64; 64],
    king_attacks: [u64; 64],
    sliding: [SlidingEntry; 3 * 64],
    behind: [u64; 64 * 64],
    in_between: [u64; 64 * 64],
}

impl Precomputed {
    #[inline(never)]
    fn init() -> Precomputed {
        use rand::prelude::*;
        let mut rng = StdRng::seed_from_u64(43);
        let mut zobrist = [0u64; 6 * 2 * 64];
        for x in &mut zobrist[..] {
            *x = rng.gen();
        }
        let mut zobrist_castling = [0u64; 16];
        for x in &mut zobrist_castling[..] {
            *x = rng.gen();
        }
        let mut zobrist_ep = [0u64; 9];
        for x in &mut zobrist_ep[..] {
            *x = rng.gen();
        }

        let mut knight_attacks = [0; 64];
        let mut king_attacks = [0; 64];
        let mut sliding = [SlidingEntry { attack: 0, mask: 0 }; 3 * 64];
        for from in 0..64 {
            sliding[0 * 64 + from].mask = blockers_and_beyond(from as i8, &crate::moves::BISHOP_DELTAS);
            sliding[1 * 64 + from].mask = blockers_and_beyond(from as i8, &crate::moves::ROOK_DELTAS);
            sliding[2 * 64 + from].mask = blockers_and_beyond(from as i8, &crate::moves::QUEEN_DELTAS);
            for to in 0..64 {
                if from == to {
                    continue;
                }
                let dr = (from as i8 / 8 - to as i8 / 8).abs();
                let df = (from as i8 % 8 - to as i8 % 8).abs();
                if dr.min(df) == 1 && dr.max(df) == 2 {
                    knight_attacks[from] |= 1 << to;
                }
                if dr.max(df) == 1 {
                    king_attacks[from] |= 1 << to;
                }
                if dr == df {
                    sliding[0 * 64 + from].attack |= 1 << to;
                }
                if dr == 0 || df == 0 {
                    sliding[1 * 64 + from].attack |= 1 << to;
                }
                if dr == df || dr == 0 || df == 0 {
                    sliding[2 * 64 + from].attack |= 1 << to;
                }
            }
        }
        let mut behind = [0; 64 * 64];
        let mut in_between = [0; 64 * 64];
        for from in 0..64 {
            for to in 0..64 {
                behind[from * 64 + to] = compute_behind(from as i8, to as i8).unwrap_or(0);
                in_between[from * 64 + to] = compute_in_between(from as i8, to as i8).unwrap_or(0);
            }
        }

        Precomputed {
            zobrist,
            zobrist_castling,
            zobrist_black_to_play: rng.gen(),
            zobrist_ep,
            knight_attacks,
            king_attacks,
            sliding,
            behind,
            in_between,
        }
    }
}

lazy_static::lazy_static! {
    static ref PRECOMPUTED: Precomputed = Precomputed::init();
}

pub fn verify(mut b: BoardState) {
    let mut s: State = (&b).into();
    assert!(s.check());
    b.clear_irrelevant_en_passant_square();
    let b2: BoardState = (&s).into();
    assert_eq!(b, b2);

    let mut undo_log = Vec::new();
    let s0 = s.clone();

    let all_gmoves: Vec<_> =
        b.all_moves().into_iter().map(Option::Some)
        .chain(Some(None))
        .collect();
    for &gm in &all_gmoves {
        assert_eq!(gm, Move::from_simple_move(gm, &s).to_simple_move());
    }

    let expected_all_moves: Vec<Move> = all_gmoves.iter()
        .map(|&gm| Move::from_simple_move(gm, &s))
        .collect();
    let mut all_moves = Vec::new();
    all_moves.push(Move::null());
    s.all_moves(&mut all_moves);
    for &m in &all_moves {
        assert!(expected_all_moves.contains(&m), "{:?} ({:?})", m, m.to_simple_move());
    }
    for &m in &expected_all_moves {
        assert!(all_moves.contains(&m), "{:?} ({:?})", m, m.to_simple_move());
    }

    let all_attacks: Vec<_> = all_moves.iter().cloned()
        .filter(|m| m.cap() != 0 &&
                    (m.to_kind() == m.from_kind() || m.to_kind() == PieceKind::Queen as u32))
        .collect();

    let mut all_attacks2 = Vec::new();
    s.all_attacks(&mut all_attacks2);
    for &m in &all_attacks {
        assert!(all_attacks2.contains(&m), "{:?} ({:?})", m, m.to_simple_move());
    }
    for &m in &all_attacks2 {
        assert!(all_attacks.contains(&m), "{:?} ({:?})", m, m.to_simple_move());
    }

    for sq in (0..64).map(crate::game::Square) {
        if let Some(p) = b.get_piece(sq) {
            if p.color != b.side_to_play() {
                let attacks_to: Vec<_> = all_attacks.iter().cloned()
                    .filter(|m| m.to() == sq.0 as u32)
                    .collect();
                assert_eq!(!attacks_to.is_empty(), s.can_attack_to(sq, b.side_to_play()), "{:?}", sq);

                fn attacker_cost(m: &Move) -> i32 {
                    match m.to_kind() {
                        0 => 100,
                        1 => 350,
                        2 => 350,
                        3 => 525,
                        4 => 1000,
                        5 => 10000,
                        _ => unreachable!(),
                    }
                }
                let expected = attacks_to.iter().cloned().min_by_key(attacker_cost);
                match (expected, s.cheapest_attack_to(sq, b.side_to_play())) {
                    (None, None) => {}
                    (Some(em), Some(m)) => {
                        assert!(attacks_to.contains(&m), "{:?}", m);
                        assert_eq!(attacker_cost(&em), attacker_cost(&m), "{:?} {:?}", em, m);
                    }
                    z => panic!("{:?} {:?}", sq, z),
                }
            }
        }
    }

    for gm in all_gmoves {
        info!("{:#?}", b.render());
        info!("{:?}", gm);
        let m = Move::from_simple_move(gm, &s);

        s.make_move(m, &mut undo_log);
        assert!(s.check());
        let mut b2: BoardState = b.clone();
        b2.make_move(gm);
        b2.clear_irrelevant_en_passant_square();
        let s2: State = (&b2).into();
        assert_eq!(s, s2);
        let b3: BoardState = (&s).into();
        assert_eq!(b3, b2);
        s.unmake_move(m, &mut undo_log);
        assert_eq!(s, s0);
    }
}

struct BitsIter(u64);

impl Iterator for BitsIter {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let res = self.0 & self.0.wrapping_neg();
            self.0 &= self.0 - 1;
            Some(res)
        }
    }
}

pub fn iter_one_positions(x: u64) -> impl Iterator<Item=u32> {
    BitsIter(x).map(u64::trailing_zeros)
}
