use crate::game::{Square, Piece, Move, BoardState};
use crate::game::Color::*;
use crate::game::PieceKind::*;
use crate::infoset::Infoset;

pub const PREAMBLE: &str = r#"
<meta charset="utf-8">
<link href="../static/style.css" rel="stylesheet">
<script>
let summary = "";
</script>
<div id="summary"></div>
"#;

macro_rules! append_to_summary {
    ($w:expr, $($arg:tt)*) => ({
        write!($w, r#"<script>document.getElementById("summary").innerHTML = (summary += {})</script>"#,
            serde_json::to_string(&format!($($arg)*)).unwrap()).unwrap();
    })
}

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

pub fn moves_to_html(b: &BoardState, moves: impl Iterator<Item=Option<Move>>) -> String {
    let mut result = Vec::<String>::new();
    let mut b = b.clone();
    for m in moves {
        let mut b2 = b.clone();
        let cap = b2.make_move(m);
        if let Some(m) = m {
            let mut s = format!(
                r#"<span style="white-space:nowrap">{}<span class=negspace></span>{:?}"#,
                b.get_piece(m.from).unwrap().to_emoji(), m.from);
            if let Some(cap) = cap {
                s.push_str("&thinsp;x");
                s.push(b.get_piece(cap).unwrap().to_emoji());
                s.push_str("<span class=negspace></span>");
                s.push_str(&m.to.to_san());
            } else {
                s.push_str(&m.to.to_san());
            }
            if let Some(p) = m.promotion {
                let p = Piece {
                    color: b.side_to_play(),
                    kind: p,
                };
                s.push_str("<span class=negspace></span>");
                s.push(p.to_emoji());
            }
            if let Some(king) = b2.find_king(b2.side_to_play()) {
                if !b2.all_attacks_to(king, b.side_to_play()).is_empty() {
                    s.push('+');
                }
            }
            s.push_str("</span>");
            result.push(s);
        } else {
            result.push("pass".to_string());
        }
        b = b2;
    }
    result.join("; ")
}
