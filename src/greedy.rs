use std::io::Write;
use std::collections::HashMap;
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
            move_number: match color {
                Color::White => 0,
                Color::Black => 1,
            },
        })
    }
}

struct GreedyPlayer {
    rng: StdRng,
    color: Color,
    summary: Vec<u8>,
    #[allow(dead_code)] experiment: bool,
    move_number: i32,
}

fn sparsen<T>(max_size: usize, rng: &mut StdRng, it: impl ExactSizeIterator<Item=T>) -> Vec<T> {
    if it.len() <= max_size {
        return it.collect();
    }
    let orig_size = it.len();
    let p = max_size as f64 / it.len() as f64;
    let mut result = Vec::with_capacity(max_size * 11 / 10);
    for x in it {
        if rng.gen_bool(p) {
            result.push(x);
        }
    }
    info!("sparsen {} to {}", orig_size, result.len());
    result
}

impl Player for GreedyPlayer {
    fn begin(&mut self, _html: &mut dyn Write) {}

    fn handle_opponent_move(&mut self,
        capture_square: Option<Square>,
        infoset: &Infoset,
        html: &mut dyn Write,
    ) {
        assert_eq!(self.color, infoset.fog_state.side_to_play());
        info!("opp capture: {:?}", capture_square);
        info!("{} possible states", infoset.possible_states.len());
        if let Some(cs) = capture_square {
            writeln!(html, "<p>Opponent captured <b>{:?}</b>.</p>", cs).unwrap();
        }
    }

    fn choose_sense(&mut self, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Square, f32)> {
        info!("choose_sense (move {})", self.move_number);
        writeln!(html, r#"<h3 id="move{}">Move {}</h3>"#, self.move_number, self.move_number).unwrap();
        assert_eq!(self.color, infoset.fog_state.side_to_play());
        write!(self.summary, "{:>6}", infoset.possible_states.len()).unwrap();
        append_to_summary!(html, r##"<tr>
            <td class=numcol><a href="#move{}">#{}</a></td>
            <td class=numcol>{}</td>"##,
            self.move_number, self.move_number,
            infoset.possible_states.len());

        write!(html, "<p>{}</p>", infoset.to_html()).unwrap();
        html.flush().unwrap();
        let timer = std::time::Instant::now();

        let possible_states = sparsen(1000, &mut self.rng, infoset.possible_states.iter().cloned());

        let mut by_taken: fnv::FnvHashMap<BoardState, fnv::FnvHashMap<Option<Move>, i32>> = Default::default();
        let mut ctx = crate::eval::Ctx::new(BoardState::initial());
        ctx.expensive_eval = true;
        by_taken.reserve(possible_states.len());
        let mut depth = 0;
        loop {
            for s in &possible_states {
                let e = by_taken.entry(s.clone()).or_default();
                let all_moves = s.all_moves().into_iter().map(Option::Some).chain(Some(None));
                e.reserve(all_moves.size_hint().1.unwrap());
                for m in all_moves {
                    let mut s2 = s.clone();
                    s2.make_move(m);
                    ctx.reset(s2);
                    let score = -crate::eval::search(depth, -10000, 10000, &mut ctx);
                    e.insert(m, score);
                }
            }
            writeln!(html, "<p>score by_taken (depth {}) took {:>5.1}s</p>", depth, timer.elapsed().as_secs_f64()).unwrap();
            html.flush().unwrap();
            if timer.elapsed().as_secs_f64() >= 1.0 || depth > 10 {
                break;
            }
            by_taken.clear();
            depth += 1;
        }
        writeln!(html, "<pre>{:#?}</pre>", ctx.stats).unwrap();

        let sense_entries = infoset.sensible_senses(&possible_states);
        let squares: Vec<Square> = sense_entries.keys().cloned().collect();
        info!("{} sensible sense squares", squares.len());

        let mut ctx = crate::eval::Ctx::new(BoardState::initial());
        ctx.expensive_eval = true;
        let mut iv: fnv::FnvHashMap<Square, i32> = Default::default();
        let candidate_moves = infoset.fog_state.all_sensible_requested_moves();
        assert!(!candidate_moves.is_empty());
        for (&sq, se) in sense_entries.iter() {
            assert!(!se.states_by_sr.is_empty());
            let v = se.states_by_sr.values().map(|sidx: &Vec<usize>| {
                assert!(!sidx.is_empty());
                candidate_moves.iter().map(|&requested: &Move| {
                    sidx.iter().map(|&i: &usize| {
                        let s = &possible_states[i];
                        let taken = s.requested_to_taken(requested);
                        by_taken[s][&taken]
                    }).min().unwrap()
                }).max().unwrap()
            }).min().unwrap();
            iv.insert(sq, v);
        }

        write!(html, "<table>").unwrap();
        for rank in (1..7).rev() {
            write!(html, "<tr>").unwrap();
            write!(html, "<td>{}</td>", rank + 1).unwrap();
            for file in 1..7 {
                let sq = Square(rank * 8 + file);
                if let Some(se) = sense_entries.get(&sq) {
                    write!(html, "<td class=numcol><div>{:.2}</div><div>{}</div></td>", se.entropy, iv[&sq]).unwrap();
                } else {
                    write!(html, "<td></td>").unwrap();
                }
            }
            write!(html, "</tr>").unwrap();
        }
        for file in " bcdefg".chars() {
            write!(html, "<td class=numcol>{}</td>", file).unwrap();
        }
        writeln!(html, "</table>").unwrap();

        let iv_weight = (2.0 - infoset.possible_states.len() as f64 / 50_000.0).max(0.0).min(1.0) * 2e-3;
        writeln!(html, "<p>weight: {}</p>", iv_weight).unwrap();

        let mut hz = Vec::new();
        for sq in squares {
            hz.push((sq, sense_entries[&sq].entropy + iv[&sq] as f64 * iv_weight));
        }

        write!(self.summary, " {:>5.1}s", timer.elapsed().as_secs_f64()).unwrap();
        append_to_summary!(html, "<td class=numcol>{:.1}s</td>", timer.elapsed().as_secs_f64());
        html.flush().unwrap();
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
        info!("{} possible states", infoset.possible_states.len());
        write!(self.summary, " {:>5}", infoset.possible_states.len()).unwrap();
        append_to_summary!(html, "<td class=numcol>{}</td>", infoset.possible_states.len());
        write!(html, "<p>{}</p>", infoset.to_html()).unwrap();
        html.flush().unwrap();
    }

    fn choose_move(&mut self, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Option<Move>, f32)> {
        assert_eq!(self.color, infoset.fog_state.side_to_play());
        let timer = std::time::Instant::now();
        info!("choose_move (move {})", self.move_number);

        let candidates = infoset.fog_state.all_sensible_requested_moves();
        let states: Vec<&BoardState> = sparsen(2000, &mut self.rng, infoset.possible_states.iter());
        let m = candidates.len();
        let n = states.len();
        let mut payoff = vec![0f32; m * n];

        struct CacheEntry {
            value: f32,
            pv: Vec<Move>,
            bonus: f32,
        }
        let depth = if n * m < 100 {
            3
        } else if n * m < 500 {
            2
        } else if n * m < 2000 {
            1
        } else {
            0
        };
        writeln!(html, "<p>search depth {}</p>", depth).unwrap();
        html.flush().unwrap();
        let search_timer = std::time::Instant::now();
        let mut eval_cache = HashMap::<BoardState, CacheEntry>::new();
        let mut ctx = crate::eval::Ctx::new(BoardState::initial());
        ctx.expensive_eval = true;
        for (i, &requested) in candidates.iter().enumerate() {
            for (j, &s) in states.iter().enumerate() {
                let taken = s.requested_to_taken(requested);
                let mut s2 = s.clone();
                let cap = s2.make_move(taken);
                let e = eval_cache.entry(s2.clone()).or_insert_with(|| {
                    ctx.reset(s2.clone());
                    let mut e = CacheEntry {
                        value: -crate::eval::search(depth, -10000, 10000, &mut ctx) as f32,
                        pv: ctx.pvs[0].clone(),
                        bonus: 0.0,
                    };
                    if cap.is_none() && e.value.abs() < 9950.0 {
                        if let Some(sq) = s2.find_king(s2.side_to_play()) {
                            if !s2.all_attacks_to(sq, s2.side_to_play().opposite()).is_empty() {
                                e.bonus = 30.0;
                            }
                        }
                    }
                    e
                });
                payoff[i * n + j] = e.value + e.bonus;
            }
        }
        writeln!(html, "<p>search took {:>5.1}s</p>", search_timer.elapsed().as_secs_f64()).unwrap();
        writeln!(html, "<p>{} matrix cells, {} unique</p>", n * m, eval_cache.len()).unwrap();
        html.flush().unwrap();
        info!("{} matrix cells, {} unique", n * m, eval_cache.len());
        info!("solving...");
        let sol = fictitious_play(m, n, &payoff, 100_000);
        let mut jx: Vec<usize> = (0..n).collect();
        jx.sort_by(|&j1, &j2| sol.strategy2[j2].partial_cmp(&sol.strategy2[j1]).unwrap());
        jx = jx.into_iter().take(6).take_while(|&j| sol.strategy2[j] > 0.01).collect();
        let mut ix: Vec<usize> = (0..m).collect();
        ix.sort_by(|&j1, &j2| sol.strategy1[j2].partial_cmp(&sol.strategy1[j1]).unwrap());
        ix = ix.into_iter().take(10).take_while(|&i| sol.strategy1[i] > 0.01).collect();

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
                let mut s2 = states[j].clone();
                let taken = s2.requested_to_taken(candidates[i]);
                s2.make_move(taken);
                let e = &eval_cache[&s2];
                let moves = Some(taken).into_iter().chain(e.pv.iter().cloned().map(Option::Some));
                let moves = crate::html::moves_to_html(&states[j], moves);
                write!(html, "<td class=numcol><div><b>{:.0}", e.value).unwrap();
                if e.bonus != 0.0 {
                    write!(html, "{:+.0}", e.bonus).unwrap();
                }
                write!(html, "</b></div>{}</td></td>", moves).unwrap();
            }
            writeln!(html, "</tr>").unwrap();
        }
        writeln!(html, "</table>").unwrap();
        writeln!(html, "<p>Game value: {:.1}</p>", sol.game_value).unwrap();

        write!(self.summary, " {:>5.1}s", timer.elapsed().as_secs_f64()).unwrap();
        append_to_summary!(html, "<td class=numcol>{:.1}s</td>", timer.elapsed().as_secs_f64());
        append_to_summary!(html, "<td class=numcol>{:.1}</td>", sol.game_value);
        html.flush().unwrap();
        candidates.into_iter()
            .map(Option::Some)
            .zip(sol.strategy1)
            .filter(|&(_, p)| p >= 2e-2)
            .collect()
    }

    fn handle_move(&mut self,
        requested: Option<Move>, taken: Option<Move>, capture_square: Option<Square>,
        infoset: &Infoset,
        html: &mut dyn Write,
    ) {
        assert_eq!(self.color.opposite(), infoset.fog_state.side_to_play());
        info!("requested move: {:?}", requested);
        info!("taken move :    {:?}", taken);
        info!("capture square: {:?}", capture_square);
        info!("{} possible states after my move", infoset.possible_states.len());
        writeln!(html, "<p>requested: {:?}</p>", requested).unwrap();
        writeln!(html, "<p>taken: {:?}.</p>", taken).unwrap();
        if let Some(cs) = capture_square {
            writeln!(html, "<p>captured <b>{:?}</b></p>", cs).unwrap();
        }
        writeln!(self.summary, " {:>5}", infoset.possible_states.len()).unwrap();
        append_to_summary!(html, "<td class=numcol>{}</td></tr>", infoset.possible_states.len());
        html.flush().unwrap();
        self.move_number += 2;
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
