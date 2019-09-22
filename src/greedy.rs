use std::io::Write;
use log::info;
use rand::prelude::*;
use crate::game::{Square, Color, PieceKind, Piece, Move, BoardState};
use crate::ai_interface::{Ai, Player};
use crate::infoset::Infoset;

#[derive(Clone)]
pub struct GreedyAi {
    pub experiment: bool,
}

impl Ai for GreedyAi {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player> {
        let infoset = Infoset::new(color);
        Box::new(GreedyPlayer {
            rng: StdRng::seed_from_u64(seed),
            color,
            infoset,
            summary: Vec::new(),
            experiment: self.experiment,
        })
    }
}

struct GreedyPlayer {
    rng: StdRng,
    color: Color,
    infoset: Infoset,
    summary: Vec<u8>,
    experiment: bool,
}

impl Player for GreedyPlayer {
    fn handle_opponent_move(&mut self, capture_square: Option<Square>, html: &mut dyn Write) {
        assert!(self.color != self.infoset.fog_state.side_to_play);
        info!("opp capture: {:?}", capture_square);
        self.infoset.opponent_move(capture_square);
        info!("{} possible states after capture", self.infoset.possible_states.len());
        if let Some(cs) = capture_square {
            writeln!(html, "<p>Opponent captured <b>{:?}</b>.</p>", cs).unwrap();
        }
    }

    fn choose_sense(&mut self, html: &mut dyn Write) -> Square {
        assert_eq!(self.color, self.infoset.fog_state.side_to_play);
        write!(self.summary, "{:>6}", self.infoset.possible_states.len()).unwrap();
        info!("{:#?}", self.infoset.render());
        write!(html, "<p>{}</p>", self.infoset.to_html()).unwrap();
        let timer = std::time::Instant::now();
        if self.experiment {
            unimplemented!("experiment")
        } else {
            let mut best_sense_rank = -1.0;
            let mut best_sense = Square(0);
            for rank in (1..7).rev() {
                let mut line = String::new();
                for file in 1..7 {
                    let sq = Square(rank * 8 + file);
                    let e = self.infoset.sense_entropy(sq) + self.rng.gen_range(0.0, 1e-4);
                    line.push_str(&format!("{:>7.2}", e));
                    if e > best_sense_rank {
                        best_sense_rank = e;
                        best_sense = sq;
                    }
                }
                info!("entropy: {}", line)
            }
            info!("best sense: {:?} {:.3}", best_sense, best_sense_rank);
            write!(self.summary, " {:>5.1}s", timer.elapsed().as_secs_f64()).unwrap();
            writeln!(html, "<p>Sense: <b>{}</b></p>", best_sense.to_san()).unwrap();
            best_sense
        }
    }

    fn handle_sense(&mut self,
        sense: Square, sense_result: &[(Square, Option<Piece>)],
        html: &mut dyn Write,
    ) {
        assert_eq!(self.color, self.infoset.fog_state.side_to_play);
        info!("sense {:?} -> {:?}", sense, sense_result);
        self.infoset.sense(sense, sense_result);
        info!("{:#?}", self.infoset.render());
        write!(self.summary, " {:>5}", self.infoset.possible_states.len()).unwrap();
        write!(html, "<p>{}</p>", self.infoset.to_html()).unwrap();
    }

    fn choose_move(&mut self, html: &mut dyn Write) -> Option<Move> {
        assert_eq!(self.color, self.infoset.fog_state.side_to_play);
        let timer = std::time::Instant::now();

        let candidates = self.infoset.fog_state.all_sensible_requested_moves();
        let m = candidates.len();
        let n = self.infoset.possible_states.len();
        let mut payoff = vec![0f32; m * n];

        let mut eval_hash = std::collections::HashMap::new();

        let depth = if n < 1000 {
            3
        } else if n < 5000 {
            2
        } else {
            1
        };

        for (i, &requested) in candidates.iter().enumerate() {
            for (j, s) in self.infoset.possible_states.iter().enumerate() {
                let taken = s.requested_to_taken(requested);
                let mut s2 = s.clone();
                s2.make_move(taken);
                let e = *eval_hash.entry(s2.clone()).or_insert_with(|| evaluate(&s2, depth, -3000, 3000));
                payoff[i * n + j] = -e as f32;
            }
            info!("{} rows left", m - 1 - i);
        }
        info!("eval_hash size: {}", eval_hash.len());
        info!("solving...");
        let sol = fictitious_play(m, n, &payoff, 100_000);
        let mut jx: Vec<usize> = (0..n).collect();
        jx.sort_by(|&j1, &j2| sol.strategy2[j2].partial_cmp(&sol.strategy2[j1]).unwrap());
        jx = jx.into_iter().take_while(|&j| sol.strategy2[j] > 0.1).collect();
        for &j in &jx {
            info!("dangerous: {} {:#?}", sol.strategy2[j], self.infoset.possible_states[j].render());
        }
        info!("game value: {}", sol.game_value);
        let mut ix: Vec<usize> = (0..m).collect();
        ix.sort_by(|&j1, &j2| sol.strategy1[j2].partial_cmp(&sol.strategy1[j1]).unwrap());
        ix = ix.into_iter().take_while(|&i| sol.strategy1[i] > 0.05).collect();
        for &i in &ix {
            info!("good move: {} {}", candidates[i].to_uci(), sol.strategy1[i]);
        }

        writeln!(html, "<table>").unwrap();
        writeln!(html, "<tr>").unwrap();
        writeln!(html, "<td></td><td></td>").unwrap();
        for &j in &jx {
            writeln!(html, "<td>{}</td>", self.infoset.possible_states[j].to_html()).unwrap();
        }
        writeln!(html, "</tr>").unwrap();
        writeln!(html, "<tr>").unwrap();
        writeln!(html, "<td></td><td></td>").unwrap();
        for &j in &jx {
            writeln!(html, "<td class=numcol>{:.3}</td>", sol.strategy2[j]).unwrap();
        }
        writeln!(html, "</tr>").unwrap();
        for &i in &ix {
            writeln!(html, "<tr>").unwrap();
            writeln!(html, "<td>{}{}</td>",
                self.infoset.fog_state.get_piece(candidates[i].from).unwrap().to_emoji(),
                candidates[i].to_uci(),
            ).unwrap();
            writeln!(html, "<td class=numcol>{:.3}</td>", sol.strategy1[i]).unwrap();
            for &j in &jx {
                writeln!(html, "<td class=numcol>{:.1}</td>", payoff[i * n + j]).unwrap();
            }
            writeln!(html, "</tr>").unwrap();
        }
        writeln!(html, "</table>").unwrap();

        let dist = rand::distributions::WeightedIndex::new(&sol.strategy1).unwrap();
        write!(self.summary, " {:>5.1}s", timer.elapsed().as_secs_f64()).unwrap();

        Some(candidates[dist.sample(&mut self.rng)])
    }

    fn handle_move(&mut self,
        requested: Option<Move>, taken: Option<Move>, capture_square: Option<Square>, 
        _html: &mut dyn Write,
    ) {
        assert_eq!(self.color, self.infoset.fog_state.side_to_play);
        info!("requested move: {:?}", requested);
        info!("taken move :    {:?}", taken);
        info!("capture square: {:?}", capture_square);
        self.infoset.my_move(requested, taken, capture_square);
        info!("{} possible states after my move", self.infoset.possible_states.len());
        info!("{:#?}", self.infoset.fog_state.render());
        writeln!(self.summary, " {:>5}", self.infoset.possible_states.len()).unwrap();
    }

    fn get_summary(&self) -> String {
        String::from_utf8(self.summary.clone()).unwrap()
    }
}

pub struct Solution {
    pub game_value: f32,
    pub strategy1: Vec<f32>,
    pub strategy2: Vec<f32>,
}

pub fn fictitious_play(m: usize, n: usize, a: &[f32], num_steps: i32) -> Solution {
    let mut strategy1 = vec![0f32; m];
    let mut strategy2 = vec![0f32; n];

    let mut vals1 = vec![0f32; m];
    let mut vals2 = vec![0f32; n];

    for _ in 0..num_steps {
        {
            let mut best = 0;
            for i in 1..m {
                if vals1[i] > vals1[best] {
                    best = i;
                }
            }
            for j in 0..n {
                vals2[j] += a[best * n + j];
            }
            strategy1[best] += 1.0;
        }
        {
            let mut best = 0;
            for j in 1..n {
                if vals2[j] < vals2[best] {
                    best = j;
                }
            }
            for i in 0..m {
                vals1[i] += a[i * n + best];
            }
            strategy2[best] += 1.0;
        }
    }

    let inv = 1.0 / num_steps as f32;
    for p in &mut strategy1 {
        *p *= inv;
    }
    for p in &mut strategy2 {
        *p *= inv;
    }
    let mut game_value = 0.0;
    for i in 0..m {
        for j in 0..n {
            game_value += a[i * n + j] * strategy1[i] * strategy2[j];
        }
    }

    Solution {
        game_value,
        strategy1,
        strategy2,
    }
}

fn material_value(k: PieceKind) -> i64 {
    match k {
        PieceKind::Pawn => 1,
        PieceKind::Knight => 3,
        PieceKind::Bishop => 3,
        PieceKind::Rook => 6,
        PieceKind::Queen => 9,
        PieceKind::King => 20,
    }
}

fn evaluate(s: &BoardState, max_depth: i32, mut alpha: i64, beta: i64) -> i64 {
    assert!(alpha <= beta);

    let mut static_val = 0;
    let mut my_king = false;
    let mut opp_king = false;
    for i in 0..64 {
        if let Some(p) = s.get_piece(Square(i)) {
            let sign = if p.color == s.side_to_play { 1 } else { - 1};
            static_val += 100 * sign * material_value(p.kind);
            if p.kind == PieceKind::King {
                if p.color == s.side_to_play {
                    my_king = true;
                } else {
                    opp_king = true;
                }
            }
        }
    }
    assert!(my_king || opp_king);
    if !opp_king {
        return beta;
    }
    if !my_king {
        return alpha;
    }

    let all_moves = s.all_moves();
    static_val += all_moves.len() as i64;
    let mut s2 = s.clone();
    s2.make_move(None);
    static_val -= s2.all_moves().len() as i64;

    if static_val >= beta {
        return beta;
    }
    alpha = alpha.max(static_val);
    if max_depth == 0 {
        return alpha;
    }

    for m in all_moves {
        let mut s2 = s.clone();
        let cs = s2.make_move(Some(m));
        if cs.is_none() {
            continue;
        }
        let t = -evaluate(&s2, max_depth - 1, -beta, -alpha);
        if t >= beta {
            return beta;
        }
        alpha = alpha.max(t);
    }
    alpha
}
