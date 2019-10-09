use log::info;
use crate::game::{Square, Color, PieceKind, Piece, BoardFlags, BoardState};

#[derive(Clone, Copy)]
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

    fn null() -> Move {
        Move::new(1, 1, 0, 0, 0, 8)
    }

    fn from_simple_move(m: crate::game::Move, s: &State) -> Move {
        let from_kind = s.get_opt_kind(m.from.0);
        assert!(from_kind > 0);
        let from_kind = from_kind - 1;
        let to_kind = m.promotion.map_or(from_kind, |k| k as u32);
        let cap = s.get_opt_kind(m.to.0);
        let mut ep_file = 8;
        if from_kind == PieceKind::Pawn as u32 {
            let c = 1 - s.side_to_play();
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
            result.replace_piece(Square(sq), Some(Piece { color, kind }), &mut crate::obs::NullObs);
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

    fn side_to_play(&self) -> u8 {
        self.flags >> 4
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
        result ^= self.side_to_play() as u64 * pre.zobrist_black_to_play;
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
        let c = self.side_to_play();
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
            if m.0 & 32767 == 0b101_000110_000100 {  // White OO
                debug_assert_eq!(c, 0);
                self.by_color[0] ^= 1 << 7 | 1 << 5;
                let r = PieceKind::Rook as usize;
                self.by_kind[r] ^= 1 << 7 | 1 << 5;
                self.hash ^= pre.zobrist[(r * 2 + 0) * 64 + 7] ^
                             pre.zobrist[(r * 2 + 0) * 64 + 5];
            } else if m.0 & 32767 == 0b101_111110_111100 {  // Black OO
                debug_assert_eq!(c, 1);
                self.by_color[1] ^= 1 << 56 + 7 | 1 << 56 + 5;
                let r = PieceKind::Rook as usize;
                self.by_kind[r] ^= 1 << 56 + 7 | 1 << 56 + 5;
                self.hash ^= pre.zobrist[(r * 2 + 1) * 64 + 56 + 7] ^
                             pre.zobrist[(r * 2 + 1) * 64 + 56 + 5];
            } else if m.0 & 32767 == 0b101_000010_000100 {  // White OOO
                debug_assert_eq!(c, 0);
                self.by_color[0] ^= 1 << 0 | 1 << 3;
                let r = PieceKind::Rook as usize;
                self.by_kind[r] ^= 1 << 0 | 1 << 3;
                self.hash ^= pre.zobrist[(r * 2 + 0) * 64 + 0] ^
                             pre.zobrist[(r * 2 + 0) * 64 + 3];
            } else if m.0 & 32767 == 0b101_111010_111100 {  // Black OOO
                debug_assert_eq!(c, 1);
                self.by_color[1] ^= 1 << 56 + 0 | 1 << 56 + 3;
                let r = PieceKind::Rook as usize;
                self.by_kind[r] ^= 1 << 56 + 0 | 1 << 56 + 3;
                self.hash ^= pre.zobrist[(r * 2 + 1) * 64 + 56 + 0] ^
                             pre.zobrist[(r * 2 + 1) * 64 + 56 + 3];
            } else if self.ep_file < 8 {
                if ((m.0 >> 6) & 0b111_111111) + ((c as u32) << 9) == 0b0_000_101000 + self.ep_file as u32 {
                    // white ep capture
                    self.by_color[1] ^= 1 << 4 * 8 + self.ep_file;
                    self.by_kind[0] ^= 1 << 4 * 8 + self.ep_file;
                    self.hash ^= pre.zobrist[1 * 64 + 4 * 8 + self.ep_file as usize];
                } else if ((m.0 >> 6) & 0b111_111111) + ((c as u32) << 9) == 0b1_000_010000 + self.ep_file as u32 {
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

    pub fn unmake_move(&mut self, m: Move, undo_log: &mut Vec<UndoEntry>) {
        let u = undo_log.pop().unwrap();
        self.hash = u.hash;
        self.flags = u.flags;
        self.ep_file = u.ep_file;

        let from_bit = 1 << m.from();
        let to_bit = 1 << m.to();
        let c = self.side_to_play();
        self.by_color[c as usize] ^= from_bit ^ to_bit;
        self.by_kind[m.from_kind() as usize] ^= from_bit;
        self.by_kind[m.to_kind() as usize] ^= to_bit;

        let cap = m.cap();
        if cap != 0 {
            self.by_color[1 - c as usize] ^= to_bit;
            self.by_kind[cap as usize - 1] ^= to_bit;
        } else {
            if m.0 & 32767 == 0b101_000110_000100 {  // White OO
                debug_assert_eq!(c, 0);
                self.by_color[0] ^= 1 << 7 | 1 << 5;
                self.by_kind[PieceKind::Rook as usize] ^= 1 << 7 | 1 << 5;
            } else if m.0 & 32767 == 0b101_111110_111100 {  // Black OO
                debug_assert_eq!(c, 1);
                self.by_color[1] ^= 1 << 56 + 7 | 1 << 56 + 5;
                self.by_kind[PieceKind::Rook as usize] ^= 1 << 56 + 7 | 1 << 56 + 5;
            } else if m.0 & 32767 == 0b101_000010_000100 {  // White OOO
                debug_assert_eq!(c, 0);
                self.by_color[0] ^= 1 << 0 | 1 << 3;
                self.by_kind[PieceKind::Rook as usize] ^= 1 << 0 | 1 << 3;
            } else if m.0 & 32767 == 0b101_111010_111100 {  // Black OOO
                debug_assert_eq!(c, 1);
                self.by_color[1] ^= 1 << 56 + 0 | 1 << 56 + 3;
                self.by_kind[PieceKind::Rook as usize] ^= 1 << 56 + 0 | 1 << 56 + 3;
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

        debug_assert_eq!(self.hash, self.recompute_hash());
    }
}

pub struct UndoEntry {
    hash: u64,
    flags: u8,
    ep_file: u8,
}

struct Precomputed {
    zobrist: [u64; 6 * 2 * 64],
    zobrist_castling: [u64; 16],
    zobrist_black_to_play: u64,
    zobrist_ep: [u64; 9],
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
        Precomputed {
            zobrist,
            zobrist_castling,
            zobrist_black_to_play: rng.gen(),
            zobrist_ep,
        }
    }
}

lazy_static::lazy_static! {
    static ref PRECOMPUTED: Precomputed = Precomputed::init();
}

pub fn verify(mut b: BoardState) {
    let mut s: State = (&b).into();
    b.clear_irrelevant_en_passant_square();
    let b2: BoardState = (&s).into();
    assert_eq!(b, b2);

    let mut undo_log = Vec::new();
    let s0 = s.clone();

    let all_moves = b.all_moves().into_iter().map(Option::Some).chain(Some(None));

    for gm in all_moves {
        info!("{:#?}", b.render());
        info!("{:?}", gm);
        let m = gm.map_or(Move::null(), |gm| Move::from_simple_move(gm, &s));

        s.make_move(m, &mut undo_log);
        let mut b2: BoardState = b.clone();
        b2.make_move(gm, &mut crate::obs::NullObs);
        b2.clear_irrelevant_en_passant_square();
        let s2: State = (&b2).into();
        assert_eq!(s, s2);
        let b3: BoardState = (&s).into();
        assert_eq!(b3, b2);
        s.unmake_move(m, &mut undo_log);
        assert_eq!(s, s0);
    }
}
