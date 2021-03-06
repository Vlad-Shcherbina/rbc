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
        let mut ctx = crate::eval::Ctx::new(BoardState::initial());
        ctx.expensive_eval = true;
        Box::new(GreedyPlayer {
            rng: StdRng::seed_from_u64(seed),
            color,
            summary: Vec::new(),
            experiment: self.experiment,
            move_number: match color {
                Color::White => 0,
                Color::Black => 1,
            },
            last_capture: None,
            ctx,
            last_sense_result: None,
            strategy: None,
        })
    }
}

type SenseResult = (Square, Vec<(Square, Option<Piece>)>);

struct GreedyPlayer {
    rng: StdRng,
    color: Color,
    summary: Vec<u8>,
    #[allow(dead_code)] experiment: bool,
    move_number: i32,
    last_capture: Option<Piece>,
    ctx: crate::eval::Ctx,
    last_sense_result: Option<SenseResult>,
    strategy: Option<HashMap<SenseResult, (f32, f32, Vec<(Option<Move>, f32)>)>>,
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
        capture: Option<(Square, Piece)>,
        infoset: &Infoset,
        html: &mut dyn Write,
    ) {
        assert_eq!(self.color, infoset.fog_state.side_to_play());
        info!("opp capture: {:?}", capture);
        info!("{} possible states", infoset.possible_states.len());
        if let Some((cs, piece)) = capture {
            writeln!(html, "<p>Opponent captured {} at <b>{:?}</b>.</p>", piece.to_emoji(), cs).unwrap();
            self.last_capture = Some(piece);
        } else {
            self.last_capture = None;
        }
    }

    fn choose_sense(&mut self, remaining_time: f64, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Square, f32)> {
        info!("choose_sense (move {})", self.move_number);
        writeln!(html, r#"<h3 id="sense{}">Move {}</h3>"#, self.move_number, self.move_number).unwrap();
        writeln!(html, "<h4>Sense</h4>").unwrap();
        let urgency = (remaining_time / 450.0).min(1.0).max(0.02);
        writeln!(html, "<p>urgency = {}</p>", urgency).unwrap();

        assert_eq!(self.color, infoset.fog_state.side_to_play());
        write!(self.summary, "{:>6}", infoset.possible_states.len()).unwrap();
        append_to_summary!(html, r##"<tr>
            <td class=numcol><i>{:.0}s</i></td>
            <td>{}</td>
            <td class=numcol><a href="#sense{}">#{}</a> <a href="#move{}">m</a></td>
            <td class=numcol>{}</td>"##,
            remaining_time,
            self.last_capture.map_or(' ', Piece::to_emoji),
            self.move_number, self.move_number, self.move_number,
            infoset.possible_states.len());

        write!(html, "<p>{}</p>", infoset.to_html()).unwrap();
        html.flush().unwrap();
        let timer = std::time::Instant::now();

        let possible_states = sparsen(1000, &mut self.rng, infoset.possible_states.iter().cloned());
        let sense_entries = infoset.sensible_senses(&possible_states);
        if sense_entries.len() == 1 {
            writeln!(html, "<p>only one sense option</p>").unwrap();
            let sq: Square = *sense_entries.keys().next().unwrap();
            append_to_summary!(html, "<td class=numcol>---</td><td></td>");
            return vec![(sq, 1.0)];
        }
        if self.move_number == 1 {
            writeln!(html, "<p>opening</p>").unwrap();
            append_to_summary!(html, "<td class=numcol>open</td><td></td>");
            return vec![
                (Square::from_san("d2"), 2.0),
                (Square::from_san("e2"), 2.0),
                (Square::from_san("b2"), 1.0),
                (Square::from_san("g2"), 1.0),
            ];
        }

        self.strategy = None;
        if (possible_states.len() as f64) < 400.0 * urgency {
            // TODO: dedup (anchor: vpMLtnvncYMi)
            let cfr_timer = std::time::Instant::now();
            let mut search_depth = 0;
            let strategy = loop {
                let mut game = crate::rbc_xf::RbcGame::new(
                    1 + 2, search_depth, &mut self.ctx,
                    crate::rbc_xf::State::ChoosePositionBeforeSense(self.color.opposite()),
                    possible_states.clone());
                let enc = crate::cfr::Encoding::new(&mut game);
                writeln!(html, "<p>Building game tree (search depth {}) took {:.3}s</p>", search_depth, cfr_timer.elapsed().as_secs_f64()).unwrap();
                if search_depth == 0 {
                    writeln!(html, "<p>{} nodes, {} infosets</p>", enc.nodes.len(), enc.infosets.len()).unwrap();
                }
                html.flush().unwrap();

                if timer.elapsed().as_secs_f64() >= 1.5 * urgency || search_depth > 9 {
                    let sol_timer = std::time::Instant::now();
                    let mut cfr = crate::cfr::Cfr::new(&enc);
                    for step in 0..1_000_000 {
                        cfr.step(&enc);
                        if sol_timer.elapsed().as_secs_f64() > 0.5 + 5.5 * urgency {
                            writeln!(html, "CFR made {} iterations", step).unwrap();
                            break;
                        }
                    }
                    break cfr.get_strategy(&enc);
                }
                search_depth += 1;
            };
            let infoset_strat = &strategy[&vec![crate::rbc_xf::Inflet::MyColor(self.color)]];
            let mut d: Vec<(Square, f32)> = infoset_strat.actions.iter().map(|(a, prob)| {
                match a {
                    crate::rbc_xf::Action::Sense(sq) => (*sq, *prob),
                    _ => unreachable!("{:?}", a),
                }
            }).collect();
            d.sort_by(|(_, p1), (_, p2)| p2.partial_cmp(p1).unwrap());
            d.retain(|&(_, p)| p > 0.03);
            self.strategy = Some(strategy.iter().filter_map(|(obs, strat)| {
                match &obs[..] {
                    [crate::rbc_xf::Inflet::MyColor(c), crate::rbc_xf::Inflet::Sense(sq, sr)] if *c == self.color => {
                        let mut resp: Vec<(Option<Move>, f32)> = strat.actions.iter().map(|(a, prob)| match a {
                            crate::rbc_xf::Action::Move(m) => (*m, *prob),
                            _ => unreachable!("{:?}", a),
                        }).filter(|&(_, prob)| prob >= 0.01).collect();
                        resp.sort_by(|(_, p1), (_, p2)| p2.partial_cmp(p1).unwrap());
                        let ev = match self.color {
                            Color::White => strat.expected_value,
                            Color::Black => -strat.expected_value,
                        };
                        Some(((*sq, sr.clone()), (ev, strat.visit_prob, resp)))
                    }
                    _ => None
                }
            }).collect());
            writeln!(html, "<p>CFR strat: {:?}</p>", d).unwrap();
            let ev = match self.color {
                Color::White => infoset_strat.expected_value,
                Color::Black => -infoset_strat.expected_value,
            };
            append_to_summary!(html,
                "<td class=numcol>*{:.1}s</td><td class=numcol>{:.1}</td>",
                timer.elapsed().as_secs_f64(),
                ev);
            return d;
        }

        let mut by_taken: fnv::FnvHashMap<BoardState, fnv::FnvHashMap<Option<Move>, i32>> = Default::default();
        by_taken.reserve(possible_states.len());
        for depth in 0..10 {
            by_taken.clear();
            for s in &possible_states {
                let e = by_taken.entry(s.clone()).or_default();
                let all_moves = s.all_moves();
                e.reserve(all_moves.len());
                for m in all_moves {
                    let mut s2 = s.clone();
                    s2.make_move(m);
                    self.ctx.reset(s2);
                    let score = -crate::eval::search(depth, -10000, 10000, &mut self.ctx);
                    e.insert(m, score);
                }
            }
            writeln!(html, "<p>score by_taken (depth {}) took {:>5.1}s</p>", depth, timer.elapsed().as_secs_f64()).unwrap();
            html.flush().unwrap();
            if timer.elapsed().as_secs_f64() >= 1.0 * urgency {
                break;
            }
        }
        writeln!(html, "<pre>{:#?}</pre>", self.ctx.stats).unwrap();

        let squares: Vec<Square> = sense_entries.keys().cloned().collect();
        info!("{} sensible sense squares", squares.len());

        // TODO: dedup (anchor: TjifpfTOFCUV)
        let mut iv: fnv::FnvHashMap<Square, i32> = Default::default();
        let candidate_moves = infoset.fog_state.all_sensible_requested_moves();
        assert!(!candidate_moves.is_empty());
        for (&sq, se) in sense_entries.iter() {
            assert!(!se.states_by_sr.is_empty());
            let v = se.states_by_sr.values().map(|sidx: &Vec<usize>| {
                assert!(!sidx.is_empty());
                candidate_moves.iter().map(|&requested: &Option<Move>| {
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
        append_to_summary!(html, "<td class=numcol>{:.1}s</td><td></td>", timer.elapsed().as_secs_f64());
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
        self.last_sense_result = Some((sense, sense_result.to_owned()));
        info!("sense {:?} -> {:?}", sense, sense_result);
        info!("{} possible states", infoset.possible_states.len());
        write!(self.summary, " {:>5}", infoset.possible_states.len()).unwrap();
        append_to_summary!(html, "<td class=numcol>{}</td>", infoset.possible_states.len());
        write!(html, "<p>{}</p>", infoset.to_html()).unwrap();
        html.flush().unwrap();
    }

    fn choose_move(&mut self, remaining_time: f64, infoset: &Infoset, html: &mut dyn Write) -> Vec<(Option<Move>, f32)> {
        assert_eq!(self.color, infoset.fog_state.side_to_play());
        let timer = std::time::Instant::now();
        info!("choose_move (move {})", self.move_number);
        writeln!(html, r#"<h4 id="move{}">Move</h3>"#, self.move_number).unwrap();
        let urgency = (remaining_time / 450.0).min(1.0).max(0.02);
        writeln!(html, "<p>urgency = {}</p>", urgency).unwrap();

        append_to_summary!(html, "<td class=numcol><i>{:.0}s</i></td>", remaining_time);

        if self.move_number == 0 {
            writeln!(html, "<p>opening</p>").unwrap();
            append_to_summary!(html, "<td class=numcol>open</td><td></td>");
            return vec![
                (Some(Move::from_uci("e2e4")), 1.0),
                (Some(Move::from_uci("d2d4")), 1.0),
                (Some(Move::from_uci("b1c3")), 1.0),
                (Some(Move::from_uci("g1f3")), 1.0),
                (Some(Move::from_uci("b2b3")), 1.0),
                (Some(Move::from_uci("g2g3")), 1.0),
                (Some(Move::from_uci("h2h4")), 1.0),
            ];
        }

        if infoset.possible_states.len() > 1 && self.rng.gen_bool(0.95) {
            if let Some(strategy) = self.strategy.take() {
                let (ev, prob, d) = strategy[&self.last_sense_result.take().unwrap()].clone();
                writeln!(html, "<p>Using stored strategy (visit prob {})</p>", prob).unwrap();
                writeln!(html, "<table>").unwrap();
                for (m, p) in &d {
                    writeln!(html, "<tr><td>{:?}</td><td>{}</td>", m, p).unwrap();
                }
                writeln!(html, "</table>").unwrap();
                append_to_summary!(html, "<td class=numcol>stored</td><td class=numcol>{:.1}</td>", ev);
                return d;
            }
        }

        if (infoset.possible_states.len() as f64) < 15.0 * urgency {
            // TODO: dedup (anchor: vpMLtnvncYMi)
            let cfr_timer = std::time::Instant::now();
            let mut search_depth = 0;
            let strategy = loop {
                let mut game = crate::rbc_xf::RbcGame::new(
                    1 + 3, search_depth, &mut self.ctx,
                    crate::rbc_xf::State::ChoosePositionBeforeMove(self.color.opposite()),
                    infoset.possible_states.clone());
                let enc = crate::cfr::Encoding::new(&mut game);
                writeln!(html, "<p>Building game tree (search depth {}) took {:.3}s</p>", search_depth, cfr_timer.elapsed().as_secs_f64()).unwrap();
                if search_depth == 0 {
                    writeln!(html, "<p>{} nodes, {} infosets</p>", enc.nodes.len(), enc.infosets.len()).unwrap();
                }
                html.flush().unwrap();

                if timer.elapsed().as_secs_f64() >= 1.5 * urgency || search_depth > 9 {
                    let sol_timer = std::time::Instant::now();
                    let mut cfr = crate::cfr::Cfr::new(&enc);
                    for step in 0..1_000_000 {
                        cfr.step(&enc);
                        if sol_timer.elapsed().as_secs_f64() > 0.5 + 5.5 * urgency {
                            writeln!(html, "CFR made {} iterations", step).unwrap();
                            break;
                        }
                    }
                    break cfr.get_strategy(&enc);
                }
                search_depth += 1;
            };
            let infoset_strat = &strategy[&vec![crate::rbc_xf::Inflet::MyColor(self.color)]];
            let mut d: Vec<(Option<Move>, f32)> = infoset_strat.actions.iter().map(|(a, prob)| {
                match a {
                    crate::rbc_xf::Action::Move(m) => (*m, *prob),
                    _ => unreachable!("{:?}", a),
                }
            }).collect();
            d.sort_by(|(_, p1), (_, p2)| p2.partial_cmp(p1).unwrap());
            d.retain(|&(_, p)| p > 0.001);

            writeln!(html, "<table>").unwrap();
            for (m, p) in &d {
                writeln!(html, "<tr><td>{:?}</td><td>{}</td>", m, p).unwrap();
            }
            writeln!(html, "</table>").unwrap();

            let ev = match self.color {
                Color::White => infoset_strat.expected_value,
                Color::Black => -infoset_strat.expected_value,
            };
            append_to_summary!(html, "<td class=numcol>*{:.1}s</td><td class=numcol>{:.1}</td>",
                timer.elapsed().as_secs_f64(),
                ev);
            return d;
        }

        let mut candidates = infoset.fog_state.all_sensible_requested_moves();
        let null_move = candidates.pop().unwrap();
        assert_eq!(null_move, None);
        let states: Vec<&BoardState> = sparsen(2000, &mut self.rng, infoset.possible_states.iter());

        struct CacheEntry {
            value: f32,
            pv: Vec<Move>,
            bonus: f32,
        }
        // TODO: dedup (anchor: TjifpfTOFCUV)
        let mut by_taken: fnv::FnvHashMap<BoardState, fnv::FnvHashMap<Option<Move>, CacheEntry>> = Default::default();
        by_taken.reserve(states.len());
        for depth in 0..10 {
            by_taken.clear();
            for &s in &states {
                let entry = by_taken.entry(s.clone()).or_default();
                let all_moves = s.all_moves();
                entry.reserve(all_moves.len());
                for m in all_moves {
                    let mut s2 = s.clone();
                    let cap = s2.make_move(m);
                    self.ctx.reset(s2.clone());
                    let score = -crate::eval::search(depth, -10000, 10000, &mut self.ctx);
                    let mut e = CacheEntry {
                        value: score as f32,
                        pv: self.ctx.pvs[0].clone(),
                        bonus: 0.0,
                    };
                    if cap.is_none() && e.value.abs() < 9950.0 {
                        if let Some(sq) = s2.find_king(s2.side_to_play()) {
                            if !s2.all_attacks_to(sq, s2.side_to_play().opposite()).is_empty() {
                                e.bonus = 30.0;
                            }
                        }
                    }
                    entry.insert(m, e);
                }
            }
            writeln!(html, "<p>score by_taken (depth {}) took {:>5.1}s</p>", depth, timer.elapsed().as_secs_f64()).unwrap();
            html.flush().unwrap();
            if timer.elapsed().as_secs_f64() >= 1.1 * urgency {
                break;
            }
        }
        writeln!(html, "<pre>{:#?}</pre>", self.ctx.stats).unwrap();

        let m = candidates.len();
        let n = states.len();
        let mut payoff = vec![0f32; m * n];

        for (i, &requested) in candidates.iter().enumerate() {
            for (j, &s) in states.iter().enumerate() {
                let taken = s.requested_to_taken(requested);
                let e = &by_taken[s][&taken];
                payoff[i * n + j] = e.value + e.bonus;
            }
        }
        html.flush().unwrap();
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
                infoset.fog_state.get_piece(candidates[i].unwrap().from).unwrap().to_emoji(),
                candidates[i].unwrap().to_uci(),
            ).unwrap();
            writeln!(html, "<td class=numcol>{:.3}</td>", sol.strategy1[i]).unwrap();
            for &j in &jx {
                let taken = states[j].requested_to_taken(candidates[i]);
                let e = &by_taken[states[j]][&taken];
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
            .zip(sol.strategy1)
            .filter(|&(_, p)| p >= 2e-2)
            .collect()
    }

    fn handle_move(&mut self,
        requested: Option<Move>, taken: Option<Move>,
        capture: Option<(Square, Vec<Piece>)>,
        infoset: &Infoset,
        html: &mut dyn Write,
    ) {
        assert_eq!(self.color.opposite(), infoset.fog_state.side_to_play());
        info!("requested move: {:?}", requested);
        info!("taken move :    {:?}", taken);
        info!("capture: {:?}", capture);
        info!("{} possible states after my move", infoset.possible_states.len());
        writeln!(html, "<p>requested: {:?}</p>", requested).unwrap();
        writeln!(html, "<p>taken: {:?}.</p>", taken).unwrap();
        if let Some((cs, ref cp)) = capture {
            writeln!(html, "<p>captured {} at <b>{:?}</b></p>",
                cp.iter().cloned().map(Piece::to_emoji).collect::<String>(),
                cs,
            ).unwrap();
        }
        writeln!(self.summary, " {:>5}", infoset.possible_states.len()).unwrap();
        append_to_summary!(html, "<td class=numcol>{}</td><td>{}</td></tr>",
            infoset.possible_states.len(),
            capture.map_or(Vec::new(), |c| c.1).into_iter().map(Piece::to_emoji).collect::<String>(),
        );
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
