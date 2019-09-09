use serde::{Serialize, Deserialize};

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
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
        match self {
            PieceKind::Pawn => 1,
            PieceKind::Knight => 2,
            PieceKind::Bishop => 3,
            PieceKind::Rook => 4,
            PieceKind::Queen => 5,
            PieceKind::King => 6,
        }
    }

    pub fn from_int(i: u32) -> PieceKind {
        match i {
            1 => PieceKind::Pawn,
            2 => PieceKind::Knight,
            3 => PieceKind::Bishop,
            4 => PieceKind::Rook,
            5 => PieceKind::Queen,
            6 => PieceKind::King,
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
#[derive(Serialize, Deserialize)]
#[serde(from = "bool", into="bool")]
pub enum Color {
    White,
    Black,
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
#[derive(Deserialize)]
#[serde(from = "crate::api::TypeValue")]
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
                kind.to_int() * 2 - 1 + match color {
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
            kind: PieceKind::from_int((x + 1) / 2),
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

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct BoardState {
    pieces: [u32; 8],
    pub side_to_play: Color,
    pub white_can_oo: bool,
    pub white_can_ooo: bool,
    pub black_can_oo: bool,
    pub black_can_ooo: bool,
    pub en_passant_square: Option<i32>,
    pub halfmove_clock: i32,
    pub fullmove_number: i32,
}

impl From<fen::BoardState> for BoardState {
    fn from(b: fen::BoardState) -> BoardState {
        let mut result = BoardState {
            pieces: [0, 0, 0, 0, 0, 0, 0, 0],
            side_to_play: b.side_to_play.into(),
            white_can_oo: b.white_can_oo,
            white_can_ooo: b.white_can_ooo,
            black_can_oo: b.black_can_oo,
            black_can_ooo: b.black_can_ooo,
            en_passant_square: b.en_passant_square.map(i32::from),
            halfmove_clock: b.halfmove_clock as i32,
            fullmove_number: b.fullmove_number as i32,
        };
        for (i, p) in b.pieces.into_iter().enumerate() {
            result.replace_piece(i as i32, p.map(Piece::from));
        }
        result
    }
}

impl BoardState {
    pub fn get_piece(&self, i: i32) -> Option<Piece> {
        let i = i as usize;
        let k = (self.pieces[i / 8] >> (i % 8 * 4)) & 15;
        Piece::from_int(k)
    }

    pub fn replace_piece(&mut self, i: i32, new_piece: Option<Piece>) -> Option<Piece> {
        let i = i as usize;
        let old = (self.pieces[i / 8] >> (i % 8 * 4)) & 15;
        self.pieces[i / 8] &= !(15 << (i % 8 * 4));
        self.pieces[i / 8] |= Piece::to_int(new_piece) << (i % 8 * 4);
        Piece::from_int(old)
    }

    pub fn render(&self) -> Vec<String> {
        let mut result = Vec::new();
        result.push(format!("{:?} to move", self.side_to_play));
        for rank in (0..8).rev() {
            let mut line = (rank + 1).to_string();
            for file in 0..8 {
                line.push(' ');
                line.push(self.get_piece(file + 8 * rank).map_or('.', Piece::to_char));
            }
            result.push(line);
        }
        result.push("  a b c d e f g h".to_owned());

        result
    }
    pub fn fog_of_war(&mut self, color: Color) {
        for sq in 0..64 {
            let p = self.get_piece(sq);
            if p.is_some() && p.unwrap().color != color {
                self.replace_piece(sq, None);
            }
        }
        self.en_passant_square = None;
        self.halfmove_clock = -1;
        self.fullmove_number = -1;
        match color {
            Color::White => {
                self.black_can_oo = true;
                self.black_can_ooo = true;
            }
            Color::Black => {
                self.white_can_oo = true;
                self.white_can_ooo = true;
            }
        }
    }

    pub fn sense(&self, p: i32) -> Vec<(i32, Option<Piece>)> {
        let mut result = Vec::with_capacity(9);
        let r = p / 8;
        let f = p % 8;
        for r in (0.max(r - 1)..=7.min(r + 1)).rev() {
            for f in 0.max(f - 1)..=7.min(f + 1) {
                let q = r * 8 + f;
                result.push((q, self.get_piece(q)));
            }
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Move {
    pub from: i32,
    pub to: i32,
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
            from: square_from_uci(&s[..2]),
            to: square_from_uci(&s[2..4]),
            promotion: p.map(PieceKind::from_char),
        }
    }

    pub fn to_uci(&self) -> String {
        let mut result = format!("{}{}", square_to_uci(self.from), square_to_uci(self.to));
        if let Some(p) = self.promotion {
            result.push(p.to_char());
        }
        result
    }
}

pub fn square_to_uci(s: i32) -> String {
    assert!(0 <= s && s < 64);
    format!("{}{}", ('a' as i32 + s % 8) as u8 as char, s / 8 + 1)
}

fn square_from_uci(s: &str) -> i32 {
    let mut it = s.chars();
    let file = it.next().unwrap();
    let rank = it.next().unwrap();
    assert!(it.next().is_none());
    assert!('a' <= file && file <= 'h');
    assert!('1' <= rank && rank <= '8');
    (file as i32 - 'a' as i32) + 8 * (rank as i32 - '1' as i32)
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
    fn test_square_to_from_uci() {
        assert_eq!(square_to_uci(0), "a1");
        assert_eq!(square_to_uci(1), "b1");
        assert_eq!(square_to_uci(63), "h8");

        assert_eq!(square_from_uci("a1"), 0);
        assert_eq!(square_from_uci("b1"), 1);
        assert_eq!(square_from_uci("h8"), 63);
    }

    #[test]
    fn test_move_to_from_uci() {
        assert_eq!(Move::from_uci("a2c1"), Move { from: 8, to: 2, promotion: None });
        assert_eq!(Move::from_uci("a2c1q"), Move { from: 8, to: 2, promotion: Some(PieceKind::Queen) });
        assert_eq!(Move { from: 8, to: 2, promotion: None }.to_uci(), "a2c1");
        assert_eq!(Move { from: 8, to: 2, promotion: Some(PieceKind::Queen) }.to_uci(), "a2c1q");
    }
}
