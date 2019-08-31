use serde::Deserialize;

pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Deserialize)]
#[serde(from = "bool")]
pub enum Color {
    White,
    Black,
}

#[derive(Clone, Copy)]
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

    fn to_char(self) -> char {
        match self.color {
            Color::Black => self.kind.to_char(),
            Color::White => self.kind.to_char().to_ascii_uppercase(),
        }
    }
}

impl std::fmt::Debug for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<{}>", self.to_char())
    }
}

#[derive(Debug, PartialEq, Eq)]
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
    fn test_square_from_uci() {
        assert_eq!(square_from_uci("a1"), 0);
        assert_eq!(square_from_uci("b1"), 1);
        assert_eq!(square_from_uci("h8"), 63);
    }

    #[test]
    fn test_move_from_uci() {
        assert_eq!(Move::from_uci("a2c1"), Move { from: 8, to: 2, promotion: None });
        assert_eq!(Move::from_uci("a2c1q"), Move { from: 8, to: 2, promotion: Some(PieceKind::Queen) });
    }
}
