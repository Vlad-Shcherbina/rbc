use serde::Deserialize;

#[derive(Clone, Copy)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
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
        match c {
            'P' => Piece { color: Color::White, kind: PieceKind::Pawn },
            'N' => Piece { color: Color::White, kind: PieceKind::Knight },
            'B' => Piece { color: Color::White, kind: PieceKind::Bishop },
            'R' => Piece { color: Color::White, kind: PieceKind::Rook },
            'Q' => Piece { color: Color::White, kind: PieceKind::Queen },
            'K' => Piece { color: Color::White, kind: PieceKind::King },
            'p' => Piece { color: Color::Black, kind: PieceKind::Pawn },
            'n' => Piece { color: Color::Black, kind: PieceKind::Knight },
            'b' => Piece { color: Color::Black, kind: PieceKind::Bishop },
            'r' => Piece { color: Color::Black, kind: PieceKind::Rook },
            'q' => Piece { color: Color::Black, kind: PieceKind::Queen },
            'k' => Piece { color: Color::Black, kind: PieceKind::King },
            _ => panic!(),
        }
    }

    fn to_char(self) -> char {
        match self {
            Piece { color: Color::White, kind: PieceKind::Pawn } => 'P',
            Piece { color: Color::White, kind: PieceKind::Knight } => 'N',
            Piece { color: Color::White, kind: PieceKind::Bishop } => 'B',
            Piece { color: Color::White, kind: PieceKind::Rook } => 'R',
            Piece { color: Color::White, kind: PieceKind::Queen } => 'Q',
            Piece { color: Color::White, kind: PieceKind::King } => 'K',
            Piece { color: Color::Black, kind: PieceKind::Pawn } => 'p',
            Piece { color: Color::Black, kind: PieceKind::Knight } => 'n',
            Piece { color: Color::Black, kind: PieceKind::Bishop } => 'b',
            Piece { color: Color::Black, kind: PieceKind::Rook } => 'r',
            Piece { color: Color::Black, kind: PieceKind::Queen } => 'q',
            Piece { color: Color::Black, kind: PieceKind::King } => 'k',
        }
    }
}

impl std::fmt::Debug for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<{}>", self.to_char())
    }
}
