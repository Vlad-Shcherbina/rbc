use std::io::Write;
use log::info;
use rand::prelude::*;
use crate::game::{Square, Color, Piece, Move, BoardState};
use crate::ai_interface::{Ai, Player};
use crate::infoset::Infoset;

#[derive(Clone)]
pub struct GreedyAi {
    pub experiment: bool,
}

impl Ai for GreedyAi {
    fn make_player(&self, color: Color, seed: u64) -> Box<dyn Player> {
        Box::new(GreedyPlayer {
            rng: StdRng::seed_from_u64(seed),
            color,
            summary: Vec::new(),
            experiment: self.experiment,
        })
    }
}

struct GreedyPlayer {
    rng: StdRng,
    color: Color,
    summary: Vec<u8>,
    experiment: bool,
}

impl Player for GreedyPlayer {
    fn handle_opponent_move(&mut self,
        capture_square: Option<Square>,
        infoset: &Infoset,
        html: &mut dyn Write,
    ) {
        assert_eq!(self.color, infoset.fog_state.side_to_play());
        info!("opp capture: {:?}", capture_square);
        info!("{} possible states after capture", infoset.possible_states.len());
        if let Some(cs) = capture_square {
            writeln!(html, "<p>Opponent captured <b>{:?}</b>.</p>", cs).unwrap();
        }
    }

    fn choose_sense(&mut self, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Square, f32)> {
        assert_eq!(self.color, infoset.fog_state.side_to_play());
        write!(self.summary, "{:>6}", infoset.possible_states.len()).unwrap();
        info!("{:#?}", infoset.render());
        write!(html, "<p>{}</p>", infoset.to_html()).unwrap();
        let timer = std::time::Instant::now();
        let mut hz = Vec::new();
        for rank in (1..7).rev() {
            let mut line = String::new();
            for file in 1..7 {
                let sq = Square(rank * 8 + file);
                let e = infoset.sense_entropy(sq);
                line.push_str(&format!("{:>7.2}", e));
                hz.push((sq, e));
            }
            info!("entropy: {}", line)
        }
        write!(self.summary, " {:>5.1}s", timer.elapsed().as_secs_f64()).unwrap();
        let m: f64 = hz.iter()
            .map(|&(_, e)| e)
            .max_by(|e1, e2| e1.partial_cmp(e2).unwrap())
            .unwrap();
        hz.into_iter()
            .filter_map(|(sq, e)| if e >= m - 1e-4 { Some((sq, 1.0)) } else { None })
            .collect()
    }

    fn handle_sense(&mut self,
        sense: Square, sense_result: &[(Square, Option<Piece>)],
        infoset: &Infoset,
        html: &mut dyn Write,
    ) {
        assert_eq!(self.color, infoset.fog_state.side_to_play());
        info!("sense {:?} -> {:?}", sense, sense_result);
        info!("{:#?}", infoset.render());
        write!(self.summary, " {:>5}", infoset.possible_states.len()).unwrap();
        write!(html, "<p>{}</p>", infoset.to_html()).unwrap();
    }

    fn choose_move(&mut self, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Option<Move>, f32)> {
        assert_eq!(self.color, infoset.fog_state.side_to_play());
        let timer = std::time::Instant::now();

        let max_states = 5000;
        let states: Vec<&BoardState> = if infoset.possible_states.len() <= max_states {
            infoset.possible_states.iter().collect()
        } else {
            let p = max_states as f64 / infoset.possible_states.len() as f64;
            let res: Vec<_> = infoset.possible_states.iter()
                .filter(|_| self.rng.gen_bool(p))
                .collect();
            writeln!(html, "<p>sparsening to {}</p>", res.len()).unwrap();
            info!("sparsening to {}", res.len());
            res
        };

        let candidates = infoset.fog_state.all_sensible_requested_moves();
        let m = candidates.len();
        let n = states.len();
        let mut payoff = vec![0f32; m * n];

        let mut eval_hash = std::collections::HashMap::new();
        for (i, &requested) in candidates.iter().enumerate() {
            for (j, &s) in states.iter().enumerate() {
                let taken = s.requested_to_taken(requested);
                let mut s2 = s.clone();
                s2.make_move(taken);
                let e = *eval_hash.entry(s2.clone()).or_insert_with(|| {
                    let mut e =  -crate::eval::quiescence(&s2, 0, -3000, 3000);
                    if let Some(sq) = s2.find_king(s2.side_to_play()) {
                        if !s2.all_attacks_to(sq, s2.side_to_play().opposite()).is_empty() {
                            e += 50;
                        }
                    }
                    e
                });
                payoff[i * n + j] = e as f32;
            }
            info!("{} rows left", m - 1 - i);
        }
        info!("eval_hash size: {}", eval_hash.len());
        info!("solving...");
        let sol = fictitious_play(m, n, &payoff, 100_000);
        let mut jx: Vec<usize> = (0..n).collect();
        jx.sort_by(|&j1, &j2| sol.strategy2[j2].partial_cmp(&sol.strategy2[j1]).unwrap());
        jx = jx.into_iter().take(6).take_while(|&j| sol.strategy2[j] > 0.01).collect();
        for &j in &jx {
            info!("dangerous: {} {:#?}", sol.strategy2[j], states[j].render());
        }
        info!("game value: {}", sol.game_value);
        let mut ix: Vec<usize> = (0..m).collect();
        ix.sort_by(|&j1, &j2| sol.strategy1[j2].partial_cmp(&sol.strategy1[j1]).unwrap());
        ix = ix.into_iter().take(10).take_while(|&i| sol.strategy1[i] > 0.01).collect();
        for &i in &ix {
            info!("good move: {} {}", candidates[i].to_uci(), sol.strategy1[i]);
        }

        writeln!(html, "<table>").unwrap();
        writeln!(html, "<tr>").unwrap();
        writeln!(html, "<td></td><td></td>").unwrap();
        for &j in &jx {
            writeln!(html, "<td>{}</td>", states[j].to_html()).unwrap();
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
                infoset.fog_state.get_piece(candidates[i].from).unwrap().to_emoji(),
                candidates[i].to_uci(),
            ).unwrap();
            writeln!(html, "<td class=numcol>{:.3}</td>", sol.strategy1[i]).unwrap();
            for &j in &jx {
                writeln!(html, "<td class=numcol>{:.1}</td>", payoff[i * n + j]).unwrap();
            }
            writeln!(html, "</tr>").unwrap();
        }
        writeln!(html, "</table>").unwrap();

        write!(self.summary, " {:>5.1}s", timer.elapsed().as_secs_f64()).unwrap();
        candidates.into_iter()
            .map(Option::Some)
            .zip(sol.strategy1)
            .filter(|&(_, p)| p >= 1e-3)
            .collect()
    }

    fn handle_move(&mut self,
        requested: Option<Move>, taken: Option<Move>, capture_square: Option<Square>,
        infoset: &Infoset,
        _html: &mut dyn Write,
    ) {
        assert_eq!(self.color.opposite(), infoset.fog_state.side_to_play());
        info!("requested move: {:?}", requested);
        info!("taken move :    {:?}", taken);
        info!("capture square: {:?}", capture_square);
        info!("{} possible states after my move", infoset.possible_states.len());
        info!("{:#?}", infoset.fog_state.render());
        writeln!(self.summary, " {:>5}", infoset.possible_states.len()).unwrap();
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
