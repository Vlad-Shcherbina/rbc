#[cfg(feature = "heavy")]
use serde::{Serialize, Deserialize};

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "heavy",
    derive(Serialize, Deserialize),
    serde(from="i8", into="i8"))]
pub struct Square(pub i8);

impl From<i8> for Square {
    fn from(x: i8) -> Self {
        assert!(0 <= x && x < 64);
        Square(x)
    }
}

impl From<Square> for i8 {
    fn from(s: Square) -> i8 {
        s.0
    }
}

impl Square {
    pub fn to_san(self) -> String {
        let x = self.0;
        format!("{}{}", ('a' as i8 + x % 8) as u8 as char, x / 8 + 1)
    }

    pub fn from_san(s: &str) -> Square {
        let mut it = s.chars();
        let file = it.next().unwrap();
        let rank = it.next().unwrap();
        assert!(it.next().is_none());
        assert!('a' <= file && file <= 'h');
        assert!('1' <= rank && rank <= '8');
        Square((file as i8 - 'a' as i8) + 8 * (rank as i8 - '1' as i8))
    }
}

impl std::fmt::Debug for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_san())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceKind {
    fn from_char(c: char) -> PieceKind {
        match c {
            'p' => PieceKind::Pawn,
            'n' => PieceKind::Knight,
            'b' => PieceKind::Bishop,
            'r' => PieceKind::Rook,
            'q' => PieceKind::Queen,
            'k' => PieceKind::King,
            _ => panic!("{:?}", c),
        }
    }

    fn to_char(self) -> char {
        match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        }
    }

    pub fn to_int(self) -> u32 {
        self as u32
    }

    pub fn from_int(i: u32) -> PieceKind {
        match i {
            0 => PieceKind::Pawn,
            1 => PieceKind::Knight,
            2 => PieceKind::Bishop,
            3 => PieceKind::Rook,
            4 => PieceKind::Queen,
            5 => PieceKind::King,
            _ => unreachable!(),
        }
    }
}

impl From<fen::PieceKind> for PieceKind {
    fn from(p: fen::PieceKind) -> PieceKind {
        match p {
            fen::PieceKind::Pawn => PieceKind::Pawn,
            fen::PieceKind::Knight => PieceKind::Knight,
            fen::PieceKind::Bishop => PieceKind::Bishop,
            fen::PieceKind::Rook => PieceKind::Rook,
            fen::PieceKind::Queen => PieceKind::Queen,
            fen::PieceKind::King => PieceKind::King,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "heavy",
    derive(Serialize, Deserialize),
    serde(from = "bool", into="bool"))]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    pub fn opposite(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl From<fen::Color> for Color {
    fn from(c: fen::Color) -> Color {
        match c {
            fen::Color::White => Color::White,
            fen::Color::Black => Color::Black,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "heavy",
    derive(Deserialize),
    serde(from = "crate::api::TypeValue"))]
pub struct Piece {
    pub kind: PieceKind,
    pub color: Color,
}

impl Piece {
    pub fn from_char(c: char) -> Piece {
        if c.is_ascii_lowercase() {
            Piece {
                color: Color::Black,
                kind: PieceKind::from_char(c),
            }
        } else {
            Piece {
                color: Color::White,
                kind: PieceKind::from_char(c.to_ascii_lowercase()),
            }
        }
    }

    pub fn to_char(self) -> char {
        match self.color {
            Color::Black => self.kind.to_char(),
            Color::White => self.kind.to_char().to_ascii_uppercase(),
        }
    }

    pub fn to_int(this: Option<Piece>) -> u32 {
        match this {
            None => 0,
            Some(Piece { kind, color }) => {
                kind.to_int() * 2 + 1 + match color {
                    Color::White => 0,
                    Color::Black => 1,
                }
            }
        }
    }

    pub fn from_int(x: u32) -> Option<Piece> {
        if x == 0 {
            return None;
        }
        Some(Piece {
            color: match (x + 1) % 2 {
                0 => Color::White,
                1 => Color::Black,
                _ => unreachable!(),
            },
            kind: PieceKind::from_int((x - 1) / 2),
        })
    }
}

impl std::fmt::Debug for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<{}>", self.to_char())
    }
}

impl From<fen::Piece> for Piece {
    fn from(p: fen::Piece) -> Piece {
        Piece {
            color: p.color.into(),
            kind: p.kind.into(),
        }
    }
}

bitflags::bitflags! {
    pub struct BoardFlags: u8 {
        const WHITE_TO_PLAY = 0b00001;
        const WHITE_CAN_OO  = 0b00010;
        const WHITE_CAN_OOO = 0b00100;
        const BLACK_CAN_OO  = 0b01000;
        const BLACK_CAN_OOO = 0b10000;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct BoardState {
    pieces: [u32; 8],
    pub flags: BoardFlags,
    pub en_passant_square: Option<Square>,
}

impl From<fen::BoardState> for BoardState {
    fn from(b: fen::BoardState) -> BoardState {
        let mut flags = BoardFlags::empty();
        if b.side_to_play == fen::Color::White {
            flags |= BoardFlags::WHITE_TO_PLAY;
        }
        if b.white_can_oo {
            flags |= BoardFlags::WHITE_CAN_OO;
        }
        if b.white_can_ooo {
            flags |= BoardFlags::WHITE_CAN_OOO;
        }
        if b.black_can_oo {
            flags |= BoardFlags::BLACK_CAN_OO;
        }
        if b.black_can_ooo {
            flags |= BoardFlags::BLACK_CAN_OOO;
        }
        let mut result = BoardState {
            pieces: [0, 0, 0, 0, 0, 0, 0, 0],
            flags,
            en_passant_square: b.en_passant_square.map(|s| Square(s as i8)),
        };
        for (i, p) in b.pieces.into_iter().enumerate() {
            result.replace_piece(Square(i as i8), p.map(Piece::from), &mut crate::obs::NullObs);
        }
        result
    }
}

impl BoardState {
    pub fn empty() -> BoardState {
        BoardState {
            pieces:  [0, 0, 0, 0, 0, 0, 0, 0],
            flags: BoardFlags::empty(),
            en_passant_square: None,
        }
    }

    pub fn initial() -> BoardState {
        fen::BoardState::from_fen(STARTING_FEN).unwrap().into()
    }

    pub fn side_to_play(&self) -> Color {
        if self.flags.contains(BoardFlags::WHITE_TO_PLAY) {
            Color::White
        } else {
            Color::Black
        }
    }

    pub fn get_piece(&self, i: Square) -> Option<Piece> {
        let i = i.0 as usize;
        let k = (self.pieces[i / 8] >> (i % 8 * 4)) & 15;
        Piece::from_int(k)
    }

    pub fn replace_piece(
        &mut self,
        sq: Square, new_piece: Option<Piece>,
        obs: &mut impl crate::obs::Obs,
    ) -> Option<Piece> {
        let i = sq.0 as usize;
        let old = (self.pieces[i / 8] >> (i % 8 * 4)) & 15;
        self.pieces[i / 8] &= !(15 << (i % 8 * 4));
        self.pieces[i / 8] |= Piece::to_int(new_piece) << (i % 8 * 4);
        let old = Piece::from_int(old);
        obs.replace_piece(sq, old, new_piece);
        old
    }

    pub fn render(&self) -> Vec<String> {
        let mut header = String::new();
        if let Some(ep) = self.en_passant_square {
            header.push_str(&format!("ep={:?} ", ep));
        }
        if self.flags.contains(BoardFlags::WHITE_CAN_OO) {
            header.push('K');
        }
        if self.flags.contains(BoardFlags::WHITE_CAN_OOO) {
            header.push('Q');
        }
        if self.flags.contains(BoardFlags::BLACK_CAN_OO) {
            header.push('k');
        }
        if self.flags.contains(BoardFlags::BLACK_CAN_OOO) {
            header.push('q');
        }
        header.push_str(&format!("  {:?} to move", self.side_to_play()));
        let mut result = vec![header];
        for rank in (0..8).rev() {
            let mut line = (rank + 1).to_string();
            for file in 0..8 {
                line.push(' ');
                line.push(self.get_piece(Square(file + 8 * rank)).map_or('.', Piece::to_char));
            }
            result.push(line);
        }
        result.push("  a b c d e f g h".to_owned());

        result
    }

    pub fn clear_irrelevant_en_passant_square(&mut self) {
        if self.en_passant_square.is_none() {
            return;
        }
        let Square(ep) = self.en_passant_square.unwrap();
        let file = ep % 8;
        match self.side_to_play() {
            Color::White => {
                if file > 0 && self.get_piece(Square(ep - 9)) ==
                    Some(Piece { color: Color::White, kind: PieceKind::Pawn}) {
                    return;
                }
                if file < 7 && self.get_piece(Square(ep - 7)) ==
                    Some(Piece { color: Color::White, kind: PieceKind::Pawn}) {
                    return;
                }
            }
            Color::Black => {
                if file > 0 && self.get_piece(Square(ep + 7)) ==
                    Some(Piece { color: Color::Black, kind: PieceKind::Pawn}) {
                    return;
                }
                if file < 7 && self.get_piece(Square(ep + 9)) ==
                    Some(Piece { color: Color::Black, kind: PieceKind::Pawn}) {
                    return;
                }
            }
        }
        self.en_passant_square = None;
    }

    pub fn fog_of_war(&mut self, color: Color) {
        for sq in 0..64 {
            let sq = Square(sq);
            let p = self.get_piece(sq);
            if p.is_some() && p.unwrap().color != color {
                self.replace_piece(sq, None, &mut crate::obs::NullObs);
            }
        }
        self.en_passant_square = None;
        match color {
            Color::White => {
                self.flags |= BoardFlags::BLACK_CAN_OO | BoardFlags::BLACK_CAN_OOO;
            }
            Color::Black => {
                self.flags |= BoardFlags::WHITE_CAN_OO | BoardFlags::WHITE_CAN_OOO;
            }
        }
    }

    pub fn sense(&self, p: Square) -> Vec<(Square, Option<Piece>)> {
        let mut result = Vec::with_capacity(9);
        let r = p.0 / 8;
        let f = p.0 % 8;
        for r in (0.max(r - 1)..=7.min(r + 1)).rev() {
            for f in 0.max(f - 1)..=7.min(f + 1) {
                let q = Square(r * 8 + f);
                result.push((q, self.get_piece(q)));
            }
        }
        result
    }

    pub fn sense_fingerprint(&self, p: Square) -> u32 {
        let mut result = 0;
        let r = p.0 / 8;
        let f = p.0 % 8;
        for r in (0.max(r - 1)..=7.min(r + 1)).rev() {
            for f in 0.max(f - 1)..=7.min(f + 1) {
                let q = Square(r * 8 + f);
                result *= 7;
                result += self.get_piece(q).map_or(6, |p| p.kind.to_int());
            }
        }
        result
    }

    #[allow(dead_code)]
    pub fn find_king_naive(&self, color: Color) -> Option<Square> {
        (0..64).map(Square)
        .find(|&s| self.get_piece(s) ==
                   Some(Piece { color, kind: PieceKind::King }))
    }

    #[inline(never)]
    pub fn find_king(&self, color: Color) -> Option<Square> {
        let x = Piece::to_int(Some(Piece { color, kind: PieceKind::King }));
        let mask: u32 = 0x11111111 * x;
        for (rank, &row) in self.pieces.iter().enumerate() {
            let mut t = row ^ mask;
            t |= t >> 1;
            t |= t >> 2;
            t = 0x11111111 & !t;
            if t == 0 {
                continue;
            }
            assert!(t & (t - 1) == 0);
            let file = (t.wrapping_mul(0x01234567) >> 28) & 7;
            let result = Some(Square(rank as i8 * 8 + file as i8));
            // assert_eq!(result, self.find_king_naive(color));
            return result;
        }
        // assert!(self.find_king_naive(color).is_none());
        None
    }

    pub fn winner(&self) -> Option<Color> {
        match (self.find_king(Color::White), self.find_king(Color::Black)) {
            (Some(_), Some(_)) => None,
            (Some(_), None) => Some(Color::White),
            (None, Some(_)) => Some(Color::Black),
            (None, None) => unreachable!(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
}

impl Move {
    pub fn from_uci(s: &str) -> Move {
        let p = if s.len() == 5 {
            Some(s.chars().rev().next().unwrap())
        } else if s.len() == 4 {
            None
        } else {
            panic!("{:?}", s)
        };
        Move {
            from: Square::from_san(&s[..2]),
            to: Square::from_san(&s[2..4]),
            promotion: p.map(PieceKind::from_char),
        }
    }

    pub fn to_uci(&self) -> String {
        let mut result = format!("{}{}", self.from.to_san(), self.to.to_san());
        if let Some(p) = self.promotion {
            result.push(p.to_char());
        }
        result
    }
}

impl std::fmt::Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piece_to_from_char() {
        for c in "pnbrqkPNBRQK".chars() {
            assert_eq!(Piece::from_char(c).to_char(), c);
        }
    }

    #[test]
    fn test_piece_to_from_int() {
        for i in 0..13 {
            assert_eq!(i, Piece::to_int(Piece::from_int(i)));
        }
    }

    #[test]
    fn test_square_to_from_san() {
        assert_eq!(Square(0).to_san(), "a1");
        assert_eq!(Square(1).to_san(), "b1");
        assert_eq!(Square(63).to_san(), "h8");

        assert_eq!(Square::from_san("a1").0, 0);
        assert_eq!(Square::from_san("b1").0, 1);
        assert_eq!(Square::from_san("h8").0, 63);
    }

    #[test]
    fn test_move_to_from_uci() {
        assert_eq!(Move::from_uci("a2c1"),
                   Move { from: Square(8), to: Square(2), promotion: None });
        assert_eq!(Move::from_uci("a2c1q"),
                   Move { from: Square(8), to: Square(2), promotion: Some(PieceKind::Queen) });
        assert_eq!(Move { from: Square(8), to: Square(2), promotion: None }.to_uci(), "a2c1");
        assert_eq!(Move { from: Square(8), to: Square(2), promotion: Some(PieceKind::Queen) }.to_uci(), "a2c1q");
    }
}
