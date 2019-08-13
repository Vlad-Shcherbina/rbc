pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

pub enum Color {
    White,
    Black,
}

pub struct Piece {
    pub kind: PieceKind,
    pub color: Color,
}
