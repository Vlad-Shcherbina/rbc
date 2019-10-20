use crate::game::{Color, Square, Move, Piece, BoardState};
use crate::cfr::{NodeInfo, Game};
use crate::infoset::Infoset;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    ChoosePosition(BoardState),
    Sense(Square),
    Move(Option<Move>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Inflet {
    ChoosePosition(BoardState),
    Sense(Square, Vec<(Square, Option<Piece>)>),
    OpponentCapture(Option<Square>),
    Move {
        requested: Option<Move>,
        taken: Option<Move>,
        capture: Option<Square>,
    }
}

pub struct RbcGame<'a> {
    pub ctx: &'a mut crate::eval::Ctx,
}

// work around https://github.com/rust-lang/rust/issues/52560
impl<'a> std::fmt::Debug for RbcGame<'a> {
    fn fmt(&self, _f: &mut std::fmt::Formatter) -> std::fmt::Result {
        unimplemented!()
    }
}

#[derive(Debug)]
enum State {
    ChooseSense(Color),
    ChooseMove(Color),
}

impl<'a> Game for RbcGame<'a> {
    type Action = Action;
    type Infoset = Vec<Inflet>;

    fn node_info(&mut self, h: &[Self::Action]) -> NodeInfo<Self::Action, Self::Infoset> {
        let mut board = BoardState::initial();
        let mut infoset = [Infoset::new(Color::White), Infoset::new(Color::Black)];
        let mut observation = [Vec::new(), Vec::new()];
        let mut state = State::ChooseSense(Color::White);

        for a in h {
            match state {
                State::ChooseSense(color) => {
                    match a {
                        &Action::Sense(sq) => {
                            let sr = board.sense(sq);
                            infoset[color as usize].sense(sq, &sr);
                            observation[color as usize].push(Inflet::Sense(sq, sr));
                            state = State::ChooseMove(color);
                        }
                        _ => unreachable!("{:?}", a),
                    }
                }
                State::ChooseMove(color) => {
                    match a {
                        &Action::Move(requested) => {
                            let taken = board.requested_to_taken(requested);
                            let capture = board.make_move(taken);
                            infoset[color as usize].my_move(requested, taken, capture);
                            observation[color as usize].push(Inflet::Move { requested, taken, capture });
                            infoset[color.opposite() as usize].opponent_move(capture);
                            observation[color.opposite() as usize].push(Inflet::OpponentCapture(capture));
                            state = State::ChooseSense(color.opposite());
                        }
                        _ => unreachable!("{:?}", a),
                    }
                }
            }
        }

        if h.len() < 5 {
            match state {
                State::ChooseSense(color) => {
                    let ss = infoset[color as usize].sensible_senses(&infoset[color as usize].possible_states);
                    return NodeInfo::Choice {
                        player: color as usize,
                        infoset: observation[color as usize].clone(),
                        actions: ss.keys().map(|sq| Action::Sense(*sq)).collect(),
                    }
                }
                State::ChooseMove(color) => {
                    let req_moves = infoset[color as usize].sensible_moves(&infoset[color as usize].possible_states);
                    return NodeInfo::Choice {
                        player: color as usize,
                        infoset: observation[color as usize].clone(),
                        actions: req_moves.into_iter().map(Action::Move).collect(),
                    }
                }
            }
        }

        self.ctx.reset(board.clone());
        crate::eval::search(1, -10000, 10000, self.ctx);
        crate::eval::search(2, -10000, 10000, self.ctx);
        let score = crate::eval::search(3, -10000, 10000, self.ctx) * (1 - 2 * (board.side_to_play() as i32));

        let score = score as f32 + 7.0 * (
            -(infoset[0].possible_states.len() as f32).log2()
            +(infoset[1].possible_states.len() as f32).log2());

        NodeInfo::Terminal(score)
    }
}