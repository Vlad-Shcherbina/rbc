use crate::game::{Square, Color, PieceKind, Piece, BoardFlags, BoardState};

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
        if let Some(ep) = b.en_passant_square {
            // TODO: clear irrelevant ep
            result.ep_file = (ep.0 % 8) as u8;
        } else {
            result.ep_file = 8;
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

    fn get_kind(&self, sq: i8) -> u32 {
        debug_assert!(((self.by_color[0] | self.by_color[1]) >> sq) & 1 != 0);
        let kind =
            (((self.by_kind[1] | self.by_kind[3] | self.by_kind[5]) >> sq) & 1) +
            (((self.by_kind[2] | self.by_kind[3]) >> sq) & 1) * 2 +
            (((self.by_kind[4] | self.by_kind[5]) >> sq) & 1) * 4;
        debug_assert!(kind < 6);
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
        result ^= (self.flags >> 4) as u64 * pre.zobrist_black_to_play;
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

pub fn verify(b: BoardState) {
    let s: State = (&b).into();
    let b2: BoardState = (&s).into();
    assert_eq!(b, b2);
}
