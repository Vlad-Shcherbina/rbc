use crate::game::{Color, PieceKind, Piece, Move, BoardState};

const PROMOTION_TARGETS: &[Option<PieceKind>] = &[
    Some(PieceKind::Knight),
    Some(PieceKind::Bishop),
    Some(PieceKind::Rook),
    Some(PieceKind::Queen),
    None,
];

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

    #[allow(clippy::cognitive_complexity)]
    pub fn all_sensible_requested_moves(&self) -> Vec<Move> {
        let mut result = Vec::new();
        for (from, p) in self.pieces.0.iter().enumerate() {
            if p.is_none() {
                continue;
            }
            let from = from as i32;
            let p = p.unwrap();
            assert_eq!(p.color, self.side_to_play);
            match p.kind {
                PieceKind::Pawn => {
                    let rank = from / 8;
                    let file = from % 8;
                    let (dr, home_rank, promotion_rank) = match self.side_to_play {
                        Color::White => (1, 1, 6),
                        Color::Black => (-1, 6, 1),
                    };

                    assert!(0 <= rank + dr && rank + dr < 8);
                    if self.pieces.0[(from + 8 * dr) as usize].is_none() {
                        let promotion_targets = if rank == promotion_rank {
                            PROMOTION_TARGETS
                        } else {
                            &[None]
                        };
                        for &promotion in promotion_targets {
                            result.push(Move {
                                from,
                                to: from + 8 * dr,
                                promotion,
                            });
                        }
                        if rank == home_rank && self.pieces.0[(from + 16 * dr) as usize].is_none() {
                            result.push(Move {
                                from,
                                to: from + 16 * dr,
                                promotion: None,
                            });
                        }
                    }
                    for df in &[-1, 1] {
                        if file + df < 0 || file + df >= 8 {
                            continue;
                        }
                        let to = (rank + dr) * 8 + file + df;
                        if self.pieces.0[to as usize].is_none() {
                            let promotion_targets = if rank + dr == 0 || rank + dr == 7 {
                                PROMOTION_TARGETS
                            } else {
                                &[None]
                            };
                            for &promotion in promotion_targets {
                                result.push(Move { from, to, promotion });
                            }
                        }
                    }
                },
                PieceKind::Bishop |
                PieceKind::Rook |
                PieceKind::Queen => {
                    let dirs: &[_] = match p.kind {
                        PieceKind::Bishop => &[(-1, -1), (-1, 1), (1, -1), (1, 1)],
                        PieceKind::Rook => &[(-1, 0), (0, -1), (0, 1), (1, 0)],
                        PieceKind::Queen => &[(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)],
                        _ => unreachable!(),
                    };
                    for &(dr, df) in dirs {
                        let mut r = from / 8;
                        let mut f = from % 8;
                        loop {
                            r += dr;
                            f += df;
                            if r < 0 || r >= 8 || f < 0 || f >= 8 {
                                break;
                            }
                            let to = r * 8 + f;
                            if self.pieces.0[to as usize].is_some() {
                                break;
                            }
                            result.push(Move { from, to, promotion: None });
                        }
                    }
                }
                PieceKind::Knight |
                PieceKind::King => {
                    let dirs: &[_] = match p.kind {
                        PieceKind::Knight => &[(-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1)],
                        PieceKind::King => &[(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)],
                        _ => unreachable!(),
                    };
                    for &(dr, df) in dirs {
                        let r = from / 8 + dr;
                        let f = from % 8 + df;
                        if r < 0 || r >= 8 || f < 0 || f >= 8 {
                            continue;
                        }
                        let to = r * 8 + f;
                        if self.pieces.0[to as usize].is_some() {
                            continue;
                        }
                        result.push(Move { from, to, promotion: None });
                    }
                }
            }
        }
        match self.side_to_play {
            Color::White => {
                if self.white_can_oo &&
                   self.pieces.0[5].is_none() &&
                   self.pieces.0[6].is_none() {
                    result.push(Move { from: 4, to: 6, promotion: None });
                }
                if self.white_can_ooo &&
                   self.pieces.0[1].is_none() &&
                   self.pieces.0[2].is_none() &&
                   self.pieces.0[3].is_none() {
                    result.push(Move { from: 4, to: 2, promotion: None });
                }
            }
            Color::Black => {
                if self.black_can_oo &&
                   self.pieces.0[56 + 5].is_none() &&
                   self.pieces.0[56 + 6].is_none() {
                    result.push(Move { from: 56 + 4, to: 56 + 6, promotion: None });
                }
                if self.black_can_ooo &&
                   self.pieces.0[56 + 1].is_none() &&
                   self.pieces.0[56 + 2].is_none() &&
                   self.pieces.0[56 + 3].is_none() {
                    result.push(Move { from: 56 + 4, to: 56 + 2, promotion: None });
                }
            }
        }
        result
    }

    pub fn requested_to_taken(&self, m: Move) -> Option<Move> {
        let p = self.pieces.0[m.from as usize].unwrap();
        assert_eq!(p.color, self.side_to_play);
        match p.kind {
            PieceKind::Pawn => {
                let dr = match self.side_to_play {
                    Color::White => 1,
                    Color::Black => -1,
                };
                if m.to == m.from + 8 * dr {
                    if self.pieces.0[m.to as usize].is_none() {
                        if m.to < 8 || m.to >= 56 {
                           Some(Move {
                                from: m.from,
                                to: m.to,
                                promotion: Some(m.promotion.unwrap_or(PieceKind::Queen)),
                            })
                        } else {
                            Some(m)
                        }
                    } else {
                        None
                    }
                } else if m.to == m.from + 16 * dr {
                    if self.pieces.0[(m.from + 8 * dr) as usize].is_some() {
                        None
                    } else if self.pieces.0[m.to as usize].is_some() {
                        Some(Move { from: m.from, to: m.from + 8 * dr, promotion: None })
                    } else {
                        Some(m)
                    }
                } else if self.en_passant_square == Some(m.to) {
                    Some(m)
                } else if self.pieces.0[m.to as usize].is_some() {
                    if m.to < 8 || m.to >= 56 {
                        Some(Move {
                            from: m.from,
                            to: m.to,
                            promotion: Some(m.promotion.unwrap_or(PieceKind::Queen)),
                        })
                    } else {
                        Some(m)
                    }
                } else {
                    None
                }
            }
            PieceKind::Knight => Some(m),
            PieceKind::King => {
                if m.to == m.from - 2 {
                    if self.pieces.0[(m.from - 1) as usize].is_none() &&
                       self.pieces.0[(m.from - 2) as usize].is_none() &&
                       self.pieces.0[(m.from - 3) as usize].is_none() {
                        Some(m)
                    } else {
                        None
                    }
                } else if m.to == m.from + 2 {
                    if self.pieces.0[(m.from + 1) as usize].is_none() &&
                       self.pieces .0[(m.from + 2) as usize].is_none() {
                        Some(m)
                    } else {
                        None
                    }
                } else {
                    Some(m)
                }
            }
            PieceKind::Bishop |
            PieceKind::Rook |
            PieceKind::Queen => {
                let mut r = m.from / 8;
                let mut f = m.from % 8;
                let dr = (m.to / 8 - r).signum();
                let df = (m.to % 8 - f).signum();
                let to = loop {
                    r += dr;
                    f += df;
                    let to = r * 8 + f;
                    if to == m.to || self.pieces.0[to as usize].is_some() {
                        break to;
                    }
                };
                Some(Move { from: m.from, to, promotion: None })
            }
        }
    }
}
