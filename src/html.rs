use crate::game::{Square, Piece, BoardState};
use crate::game::Color::*;
use crate::game::PieceKind::*;
use crate::infoset::Infoset;

pub const PREAMBLE: &str = r#"
<meta charset="utf-8">
<link href="../static/style.css" rel="stylesheet">
"#;

impl Piece {
    pub fn to_emoji(self) -> char {
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
                    s.push_str("<td class=bc>");
                } else {
                    s.push_str("<td>");
                }
                let sq = Square(rank * 8 + file);
                if let Some(piece) = self.get_piece(sq) {
                    s.push_str("<div class=piece>");
                    s.push(piece.to_emoji());
                    s.push_str("</div>");
                }
                s.push_str("</td>");
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

impl Infoset {
    pub fn to_html(&self) -> String {
        let mut piece_sets = vec![0u16; 64];
        for s in &self.possible_states {
            for (i, p) in piece_sets.iter_mut().enumerate() {
                *p |= 1u16 << Piece::to_int(s.get_piece(Square(i as i8)));
            }
        }
        let mut s = String::new();
        s.push_str("<table class=board-large>");
        for rank in (0..8).rev() {
            s.push_str("<tr>");
            s.push_str(&format!("<td class=coord>{}</td>", rank + 1));
            for file in 0..8 {
                if (rank + file) % 2 == 0 {
                    s.push_str("<td class=bc>");
                } else {
                    s.push_str("<td>");
                }

                let mut ps: String = (1..13)
                    .filter(|i| piece_sets[rank * 8 + file] & (1u16 << i) != 0)
                    .map(|i| Piece::from_int(i).unwrap().to_emoji())
                    .collect();
                if piece_sets[rank * 8 + file] & 1 == 1 {
                    ps.push('?');
                }

                if ps != "?" {
                    s.push_str(&format!(r#"<div class="piece cnt{}">"#, ps.chars().count()));
                    if ps.chars().count() == 7 {
                        s.push_str("???");
                    } else {
                        s.push_str(&ps);
                    }
                    s.push_str("</div>");
                }
                s.push_str("</td>");
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
        s.push_str(&format!("{} possibilities", self.possible_states.len()));
        s
    }
}
