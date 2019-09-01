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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Deserialize)]
#[serde(from = "bool")]
pub enum Color {
    White,
    Black,
}

impl From<fen::Color> for Color {
    fn from(c: fen::Color) -> Color {
        match c {
            fen::Color::White => Color::White,
            fen::Color::Black => Color::Black,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

impl From<fen::Piece> for Piece {
    fn from(p: fen::Piece) -> Piece {
        Piece {
            color: p.color.into(),
            kind: p.kind.into(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct BoardState {
    pub pieces: crate::derive_wrapper::Wrapper<[Option<Piece>; 64]>,
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
        let mut pieces = [None; 64];
        assert_eq!(b.pieces.len(), 64);
        for (i, p) in b.pieces.into_iter().enumerate() {
            pieces[i] = p.map(|p| p.into());
        }
        BoardState {
            pieces: crate::derive_wrapper::Wrapper::new(pieces),
            side_to_play: b.side_to_play.into(),
            white_can_oo: b.white_can_oo,
            white_can_ooo: b.white_can_ooo,
            black_can_oo: b.black_can_oo,
            black_can_ooo: b.black_can_ooo,
            en_passant_square: b.en_passant_square.map(i32::from),
            halfmove_clock: b.halfmove_clock as i32,
            fullmove_number: b.fullmove_number as i32,
        }
    }
}

impl BoardState {
    #[allow(clippy::cognitive_complexity)]
    pub fn make_move(&mut self, m: Option<Move>) {
        self.side_to_play = match self.side_to_play {
            Color::White => Color::Black,
            Color::Black => { self.fullmove_number += 1; Color::White },
        };
        self.halfmove_clock += 1 ;
        let m = match m {
            Some(m) => m,
            None => return,
        };
        let mut p = self.pieces.0[m.from as usize].take();
        match p {
            Some(Piece { kind: PieceKind::Pawn, ..}) => {
                if m.to < 8 || m.to >= 56 {
                    p.as_mut().unwrap().kind = m.promotion.unwrap();
                }
                self.halfmove_clock = 0;

                if let Some(ep) = self.en_passant_square {
                    if m.to == ep {
                        match self.side_to_play {
                            Color::White => self.pieces.0[(ep + 8) as usize] = None,
                            Color::Black => self.pieces.0[(ep - 8) as usize] = None,
                        }
                    }
                }
            }
            Some(_) => {}
            None => panic!()
        }
        if self.pieces.0[m.to as usize].is_some() {
            self.halfmove_clock = 0;
        }
        self.pieces.0[m.to as usize] = p;

        if p.unwrap().kind == PieceKind::King && m.from == 4 && m.to == 6 {
            assert!(self.white_can_oo);
            assert_eq!(self.pieces.0[7].unwrap().kind, PieceKind::Rook);
            self.white_can_oo = false;
            self.white_can_ooo = false;
            assert!(self.pieces.0[5].is_none());
            self.pieces.0[5] = self.pieces.0[7].take();
        }
        if p.unwrap().kind == PieceKind::King && m.from == 4 && m.to == 2 {
            assert!(self.white_can_ooo);
            assert_eq!(self.pieces.0[0].unwrap().kind, PieceKind::Rook);
            self.white_can_oo = false;
            self.white_can_ooo = false;
            assert!(self.pieces.0[3].is_none());
            self.pieces.0[3] = self.pieces.0[0].take();
        }

        if p.unwrap().kind == PieceKind::King && m.from == 60 && m.to == 62 {
            assert!(self.black_can_oo);
            assert_eq!(self.pieces.0[63].unwrap().kind, PieceKind::Rook);
            self.black_can_oo = false;
            self.black_can_ooo = false;
            assert!(self.pieces.0[61].is_none());
            self.pieces.0[61] = self.pieces.0[63].take();
        }
        if p.unwrap().kind == PieceKind::King && m.from == 60 && m.to == 58 {
            assert!(self.black_can_ooo);
            assert_eq!(self.pieces.0[56].unwrap().kind, PieceKind::Rook);
            self.black_can_oo = false;
            self.black_can_ooo = false;
            assert!(self.pieces.0[59].is_none());
            self.pieces.0[59] = self.pieces.0[56].take();
        }

        if m.to == 4 || m.from == 4 {
            self.white_can_oo = false;
            self.white_can_ooo = false;
        }
        if m.to == 0 || m.from == 0 {
            self.white_can_ooo = false;
        }
        if m.to == 7 || m.from == 7 {
            self.white_can_oo = false;
        }

        if m.to == 60 || m.from == 60 {
            self.black_can_oo = false;
            self.black_can_ooo = false;
        }
        if m.to == 56 || m.from == 56 {
            self.black_can_ooo = false;
        }
        if m.to == 63 || m.from == 63 {
            self.black_can_oo = false;
        }
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
    fn test_square_to_from_uci() {
        assert_eq!(square_to_uci(0), "a1");
        assert_eq!(square_to_uci(1), "b1");
        assert_eq!(square_to_uci(63), "h8");

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
