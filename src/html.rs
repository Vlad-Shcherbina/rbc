use crate::game::{Square, Piece, BoardState};
use crate::game::Color::*;
use crate::game::PieceKind::*;

pub const PREAMBLE: &str = r#"
<meta charset="utf-8">
<link href="../static/style.css" rel="stylesheet">
"#;

impl Piece {
    fn to_emoji(self) -> char {
        match (self.color, self.kind) {
            (White, Pawn) => '♙',
            (White, Knight) => '♘',
            (White, Bishop) => '♗',
            (White, Rook) => '♖',
            (White, Queen) => '♕',
            (White, King) => '♔',
            (Black, Pawn) => '♟',
            (Black, Knight) => '♞',
            (Black, Bishop) => '♝',
            (Black, Rook) => '♜',
            (Black, Queen) => '♛',
            (Black, King) => '♚',
        }
    }
}

impl BoardState {
    pub fn to_html(&self) -> String {
        let mut s = String::new();
        s.push_str("<table class=board>");
        for rank in (0..8).rev() {
            s.push_str("<tr>");
            s.push_str(&format!("<td class=coord>{}</td>", rank + 1));
            for file in 0..8 {
                if (rank + file) % 2 == 0 {
                    s.push_str("<td class=bc><div class=piece>");
                } else {
                    s.push_str("<td><div class=piece>");
                }
                let sq = Square(rank * 8 + file);
                if let Some(piece) = self.get_piece(sq) {
                    s.push(piece.to_emoji());
                }
                s.push_str("</div></td>");
            }
            s.push_str("</tr>");
        }
        s.push_str("<tr>");
        for c in " abcdefgh".chars() {
            s.push_str("<td class=coord>");
            s.push(c);
            s.push_str("</td>");
        }
        s.push_str("</tr>");
        s.push_str("</table>");
        s
    }
}
