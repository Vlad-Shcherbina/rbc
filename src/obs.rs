use crate::game::{Square, Color, Piece, Move, BoardState};

pub trait Obs {
    fn replace_piece(&mut self, sq: Square, old: Option<Piece>, new: Option<Piece>);
}

pub struct NullObs;

impl Obs for NullObs {
    fn replace_piece(&mut self, _sq: Square, _old: Option<Piece>, _new: Option<Piece>) {}
}

struct UndoState {
    edit_pos: usize,
    flags: crate::game::BoardFlags,
    en_passant_square: Option<Square>,
}

pub struct StateObs {
    undo_states: Vec<UndoState>,
    edits: Vec<(Square, Option<Piece>, Option<Piece>)>,
    pub material: i32,
}

impl StateObs {
    pub fn new(b: &BoardState) -> StateObs {
        let mut obs = StateObs {
            undo_states: Vec::new(),
            edits: Vec::new(),
            material: 0,
        };
        for sq in (0..64).map(Square) {
            let p = b.get_piece(sq);
            if p.is_some() {
                obs.replace_piece_raw(sq, None, p);
            }
        }
        obs
    }

    fn replace_piece_raw(&mut self, _sq: Square, old: Option<Piece>, new: Option<Piece>) {
        if let Some(p) = old {
            match p.color {
                Color::White =>
                    self.material -= crate::eval::material_value(p.kind),
                Color::Black =>
                    self.material += crate::eval::material_value(p.kind),
            }
        }
        if let Some(p) = new {
            match p.color {
                Color::White =>
                    self.material += crate::eval::material_value(p.kind),
                Color::Black =>
                    self.material -= crate::eval::material_value(p.kind),
            }
        }
    }

    pub fn push(&mut self, b: &BoardState) {
        self.undo_states.push(UndoState {
            edit_pos: self.edits.len(),
            flags: b.flags,
            en_passant_square: b.en_passant_square,
        });
    }

    pub fn pop(&mut self, b: &mut BoardState) {
        let us = self.undo_states.pop().unwrap();
        assert!(self.edits.len() >= us.edit_pos);
        while self.edits.len() > us.edit_pos {
            let (sq, old, new) = self.edits.pop().unwrap();
            self.replace_piece_raw(sq, new, old);
            let new2 = b.replace_piece(sq, old, &mut NullObs);
            assert_eq!(new, new2);
        }
        b.flags = us.flags;
        b.en_passant_square = us.en_passant_square;
    }
}

impl Obs for StateObs {
    fn replace_piece(&mut self, sq: Square, old: Option<Piece>, new: Option<Piece>) {
        self.edits.push((sq, old, new));
        self.replace_piece_raw(sq, old, new);
    }
}

#[cfg(test)]
#[test]
fn test_obs() {
    use crate::game::Move;
    let mut b = BoardState::initial();
    let mut obs = StateObs::new(&b);
    let m = Move::from_uci("e2e4");
    obs.push(&b);
    b.make_move(Some(m), &mut obs);
    obs.pop(&mut b);
    assert_eq!(b, BoardState::initial());
}

pub struct BigState {
    pub board: BoardState,
    pub obs: StateObs,
}

impl BigState {
    pub fn new(board: BoardState) -> BigState {
        BigState {
            obs: StateObs::new(&board),
            board,
        }
    }

    pub fn push(&mut self) {
        self.obs.push(&self.board);
    }

    pub fn pop(&mut self) {
        self.obs.pop(&mut self.board);
    }

    pub fn make_move(&mut self, m: Option<Move>) -> Option<Square> {
        self.board.make_move(m, &mut self.obs)
    }
}
