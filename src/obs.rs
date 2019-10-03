use crate::game::{Square, Piece};

pub trait Obs {
    fn replace_piece(&mut self, sq: Square, old: Option<Piece>, new: Option<Piece>);
}

pub struct NullObs;

impl Obs for NullObs {
    fn replace_piece(&mut self, _sq: Square, _old: Option<Piece>, _new: Option<Piece>) {}
}
