use crate::game::{Square, Color, PieceKind, Piece, Move, BoardState};

const PROMOTION_TARGETS: &[Option<PieceKind>] = &[
    Some(PieceKind::Knight),
    Some(PieceKind::Bishop),
    Some(PieceKind::Rook),
    Some(PieceKind::Queen),
    None,
];

impl BoardState {
    #[inline(never)]
    pub fn make_move_under_fog(&mut self, capture_square: Option<Square>) {
        self.side_to_play = self.side_to_play.opposite();
        if let Some(Square(p)) = capture_square {
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
            let p = self.replace_piece(Square(p), None);
            assert_eq!(p.unwrap().color, self.side_to_play);
        }
    }

    #[inline(never)]
    #[allow(clippy::cognitive_complexity)]
    pub fn make_move(&mut self, m: Option<Move>) -> Option<Square> {
        self.side_to_play = self.side_to_play.opposite();
        let m = match m {
            Some(m) => m,
            None => {
                self.en_passant_square = None;
                return None;
            }
        };

        let mut capture_square = if self.get_piece(m.to).is_some() {
            Some(m.to)
        } else {
            None
        };

        let mut p = self.replace_piece(m.from, None);
        match p {
            Some(Piece { kind: PieceKind::Pawn, ..}) => {
                if let Some(ep) = self.en_passant_square.take() {
                    if m.to == ep {
                        let cap = match self.side_to_play {
                            Color::White => ep.0 + 8,
                            Color::Black => ep.0 - 8,
                        };
                        capture_square = Some(Square(cap));
                        self.replace_piece(Square(cap), None);
                    }
                }

                if m.to.0 < 8 || m.to.0 >= 56 {
                    p.as_mut().unwrap().kind = m.promotion.unwrap();
                }

                if m.to.0 == m.from.0 - 16 {
                    self.en_passant_square = Some(Square(m.from.0 - 8));
                }
                if m.to.0 == m.from.0 + 16 {
                    self.en_passant_square = Some(Square(m.from.0 + 8));
                }
            }
            Some(_) => {
                self.en_passant_square = None;
            }
            None => panic!()
        }
        self.replace_piece(m.to, p);

        if p.unwrap().kind == PieceKind::King && m.from.0 == 4 && m.to.0 == 6 {
            assert!(self.white_can_oo);
            assert_eq!(self.get_piece(Square(7)).unwrap().kind, PieceKind::Rook);
            self.white_can_oo = false;
            self.white_can_ooo = false;
            let rook = self.replace_piece(Square(7), None);
            let e = self.replace_piece(Square(5), rook);
            assert!(e.is_none());
        }
        if p.unwrap().kind == PieceKind::King && m.from.0 == 4 && m.to.0 == 2 {
            assert!(self.white_can_ooo);
            assert_eq!(self.get_piece(Square(0)).unwrap().kind, PieceKind::Rook);
            self.white_can_oo = false;
            self.white_can_ooo = false;
            let rook = self.replace_piece(Square(0), None);
            let e = self.replace_piece(Square(3), rook);
            assert!(e.is_none());
        }

        if p.unwrap().kind == PieceKind::King && m.from.0 == 60 && m.to.0 == 62 {
            assert!(self.black_can_oo);
            assert_eq!(self.get_piece(Square(63)).unwrap().kind, PieceKind::Rook);
            self.black_can_oo = false;
            self.black_can_ooo = false;
            let rook = self.replace_piece(Square(63), None);
            let e = self.replace_piece(Square(61), rook);
            assert!(e.is_none());
        }
        if p.unwrap().kind == PieceKind::King && m.from.0 == 60 && m.to.0 == 58 {
            assert!(self.black_can_ooo);
            assert_eq!(self.get_piece(Square(56)).unwrap().kind, PieceKind::Rook);
            self.black_can_oo = false;
            self.black_can_ooo = false;
            let rook = self.replace_piece(Square(56), None);
            let e = self.replace_piece(Square(59), rook);
            assert!(e.is_none());
        }

        if m.to.0 == 4 || m.from.0 == 4 {
            self.white_can_oo = false;
            self.white_can_ooo = false;
        }
        if m.to.0 == 0 || m.from.0 == 0 {
            self.white_can_ooo = false;
        }
        if m.to.0 == 7 || m.from.0 == 7 {
            self.white_can_oo = false;
        }

        if m.to.0 == 60 || m.from.0 == 60 {
            self.black_can_oo = false;
            self.black_can_ooo = false;
        }
        if m.to.0 == 56 || m.from.0 == 56 {
            self.black_can_ooo = false;
        }
        if m.to.0 == 63 || m.from.0 == 63 {
            self.black_can_oo = false;
        }
        capture_square
    }

    #[inline(never)]
    #[allow(clippy::cognitive_complexity)]
    pub fn all_sensible_requested_moves(&self) -> Vec<Move> {
        let mut result = Vec::with_capacity(128);
        for from in 0..64 {
            let from = Square(from);
            let p = self.get_piece(from);
            if p.is_none() {
                continue;
            }
            let p = p.unwrap();
            assert_eq!(p.color, self.side_to_play);
            match p.kind {
                PieceKind::Pawn => {
                    let rank = from.0 / 8;
                    let file = from.0 % 8;
                    let (dr, home_rank, promotion_rank) = match self.side_to_play {
                        Color::White => (1, 1, 6),
                        Color::Black => (-1, 6, 1),
                    };

                    assert!(0 <= rank + dr && rank + dr < 8);
                    if self.get_piece(Square(from.0 + 8 * dr)).is_none() {
                        let promotion_targets = if rank == promotion_rank {
                            PROMOTION_TARGETS
                        } else {
                            &[None]
                        };
                        for &promotion in promotion_targets {
                            result.push(Move {
                                from,
                                to: Square(from.0 + 8 * dr),
                                promotion,
                            });
                        }
                        if rank == home_rank && self.get_piece(Square(from.0 + 16 * dr)).is_none() {
                            result.push(Move {
                                from,
                                to: Square(from.0 + 16 * dr),
                                promotion: None,
                            });
                        }
                    }
                    for df in &[-1, 1] {
                        if file + df < 0 || file + df >= 8 {
                            continue;
                        }
                        let to = Square((rank + dr) * 8 + file + df);
                        if self.get_piece(to).is_none() {
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
                        let mut r = from.0 / 8;
                        let mut f = from.0 % 8;
                        loop {
                            r += dr;
                            f += df;
                            if r < 0 || r >= 8 || f < 0 || f >= 8 {
                                break;
                            }
                            let to = Square(r * 8 + f);
                            if self.get_piece(to).is_some() {
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
                        let r = from.0 / 8 + dr;
                        let f = from.0 % 8 + df;
                        if r < 0 || r >= 8 || f < 0 || f >= 8 {
                            continue;
                        }
                        let to = Square(r * 8 + f);
                        if self.get_piece(to).is_some() {
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
                   self.get_piece(Square(5)).is_none() &&
                   self.get_piece(Square(6)).is_none() {
                    result.push(Move { from: Square(4), to: Square(6), promotion: None });
                }
                if self.white_can_ooo &&
                   self.get_piece(Square(1)).is_none() &&
                   self.get_piece(Square(2)).is_none() &&
                   self.get_piece(Square(3)).is_none() {
                    result.push(Move { from: Square(4), to: Square(2), promotion: None });
                }
            }
            Color::Black => {
                if self.black_can_oo &&
                   self.get_piece(Square(56 + 5)).is_none() &&
                   self.get_piece(Square(56 + 6)).is_none() {
                    result.push(Move { from: Square(56 + 4), to: Square(56 + 6), promotion: None });
                }
                if self.black_can_ooo &&
                   self.get_piece(Square(56 + 1)).is_none() &&
                   self.get_piece(Square(56 + 2)).is_none() &&
                   self.get_piece(Square(56 + 3)).is_none() {
                    result.push(Move { from: Square(56 + 4), to: Square(56 + 2), promotion: None });
                }
            }
        }
        result
    }

    #[inline(never)]
    pub fn requested_to_taken(&self, m: Move) -> Option<Move> {
        let p = self.get_piece(m.from).unwrap();
        assert_eq!(p.color, self.side_to_play);
        match p.kind {
            PieceKind::Pawn => {
                let dr = match self.side_to_play {
                    Color::White => 1,
                    Color::Black => -1,
                };
                if m.to.0 == m.from.0 + 8 * dr {
                    if self.get_piece(m.to).is_none() {
                        if m.to.0 < 8 || m.to.0 >= 56 {
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
                } else if m.to.0 == m.from.0 + 16 * dr {
                    if self.get_piece(Square(m.from.0 + 8 * dr)).is_some() {
                        None
                    } else if self.get_piece(m.to).is_some() {
                        Some(Move { from: m.from, to: Square(m.from.0 + 8 * dr), promotion: None })
                    } else {
                        Some(m)
                    }
                } else if self.en_passant_square == Some(m.to) {
                    Some(m)
                } else if self.get_piece(m.to).is_some() {
                    if m.to.0 < 8 || m.to.0 >= 56 {
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
                if m.to.0 == m.from.0 - 2 {
                    if self.get_piece(Square(m.from.0 - 1)).is_none() &&
                       self.get_piece(Square(m.from.0 - 2)).is_none() &&
                       self.get_piece(Square(m.from.0 - 3)).is_none() {
                        Some(m)
                    } else {
                        None
                    }
                } else if m.to.0 == m.from.0 + 2 {
                    if self.get_piece(Square(m.from.0 + 1)).is_none() &&
                       self.get_piece(Square(m.from.0 + 2)).is_none() {
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
                let mut r = m.from.0 / 8;
                let mut f = m.from.0 % 8;
                let dr = (m.to.0 / 8 - r).signum();
                let df = (m.to.0 % 8 - f).signum();
                let to = loop {
                    r += dr;
                    f += df;
                    let to = Square(r * 8 + f);
                    if to == m.to || self.get_piece(to).is_some() {
                        break to;
                    }
                };
                Some(Move { from: m.from, to, promotion: None })
            }
        }
    }

    #[inline(never)]
    pub fn all_moves(&self) -> Vec<Move> {
        // TODO: inefficient
        let mut fog_state = self.clone();
        fog_state.fog_of_war(self.side_to_play);

        let mut moves: Vec<Move> =
            fog_state.all_sensible_requested_moves()
            .into_iter()
            .filter_map(|m| self.requested_to_taken(m))
            .collect();
        let mut seen = std::collections::HashSet::with_capacity(moves.len());
        moves.retain(|m| seen.insert(*m));
        moves
    }
}
