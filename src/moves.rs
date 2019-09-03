use crate::game::{Color, PieceKind, Piece, Move, BoardState};

impl BoardState {
    pub fn make_move_under_fog(&mut self, capture_square: Option<i32>) {
        self.side_to_play = self.side_to_play.opposite();
        if let Some(p) = capture_square {
            match self.side_to_play {
                Color::White => {
                    if p == 0 {
                        self.white_can_ooo = false;
                    }
                    if p == 7 {
                        self.white_can_oo = false;
                    }
                    if p == 4 {
                        self.white_can_ooo = false;
                        self.white_can_oo = false;
                    }
                }
                Color::Black => {
                    if p == 56 {
                        self.black_can_ooo = false;
                    }
                    if p == 63 {
                        self.black_can_oo = false;
                    }
                    if p == 60 {
                        self.black_can_ooo = false;
                        self.black_can_oo = false;
                    }
                }
            }
            let p = self.pieces.0[p as usize].take();
            assert_eq!(p.unwrap().color, self.side_to_play);
        }
    }

    #[allow(clippy::cognitive_complexity)]
    pub fn make_move(&mut self, m: Option<Move>) {
        self.side_to_play = match self.side_to_play {
            Color::White => Color::Black,
            Color::Black => { self.fullmove_number += 1; Color::White },
        };
        self.halfmove_clock += 1;
        let m = match m {
            Some(m) => m,
            None => {
                self.en_passant_square = None;
                return;
            }
        };
        let mut p = self.pieces.0[m.from as usize].take();
        match p {
            Some(Piece { kind: PieceKind::Pawn, ..}) => {
                if let Some(ep) = self.en_passant_square.take() {
                    if m.to == ep {
                        match self.side_to_play {
                            Color::White => self.pieces.0[(ep + 8) as usize] = None,
                            Color::Black => self.pieces.0[(ep - 8) as usize] = None,
                        }
                    }
                }

                if m.to < 8 || m.to >= 56 {
                    p.as_mut().unwrap().kind = m.promotion.unwrap();
                }
                self.halfmove_clock = 0;

                if m.to == m.from - 16 {
                    let file = m.to % 8;
                    if file > 0 &&
                       self.pieces.0[(m.from - 17) as usize] ==
                       Some(Piece { color: Color::White, kind: PieceKind::Pawn}) {
                        self.en_passant_square = Some(m.from - 8);
                    }
                    if file < 7 &&
                       self.pieces.0[(m.from - 15) as usize] ==
                       Some(Piece { color: Color::White, kind: PieceKind::Pawn}) {
                        self.en_passant_square = Some(m.from - 8);
                    }
                }
                if m.to == m.from + 16 {
                    let file = m.to % 8;
                    if file > 0 &&
                       self.pieces.0[(m.from + 15) as usize] ==
                       Some(Piece { color: Color::Black, kind: PieceKind::Pawn}) {
                        self.en_passant_square = Some(m.from + 8);
                    }
                    if file < 7 &&
                       self.pieces.0[(m.from + 17) as usize] ==
                       Some(Piece { color: Color::Black, kind: PieceKind::Pawn}) {
                        self.en_passant_square = Some(m.from + 8);
                    }
                }
            }
            Some(_) => {
                self.en_passant_square = None;
            }
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