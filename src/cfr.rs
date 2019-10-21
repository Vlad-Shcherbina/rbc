use std::collections::HashMap;

#[derive(Debug)]
pub enum NodeInfo<Action, Infoset> {
    Terminal(f32),
    Chance(Vec<(f32, Action)>),
    Choice {
        player: usize,
        infoset: Infoset,
        actions: Vec<Action>,
    }
}

pub trait Game: Sized {
    type Action: Clone + Eq + std::fmt::Debug;
    type Infoset: Clone + Eq + std::hash::Hash + std::fmt::Debug;

    fn node_info(&mut self, h: &[Self::Action]) -> NodeInfo<Self::Action, Self::Infoset>;
}

type CompactNode = usize;

#[derive(Debug)]
pub struct CompactInfoset<OrigAction, OrigInfoset>
{
    pub orig: OrigInfoset,
    pub player: usize,
    pub actions: Vec<OrigAction>,

    observable_history: Vec<(usize, usize)>,
    // Pairs (infoset, action) for all past choices by this player.
    // To check perfect recall property.
}

#[derive(Debug)]
pub struct Encoding<G: Game> {
    pub infoset_by_orig: HashMap<G::Infoset, usize>,
    pub infosets: Vec<CompactInfoset<G::Action, G::Infoset>>,
    pub nodes: Vec<NodeInfo</*Action*/CompactNode, /*Infoset*/usize>>,
    pub parents: Vec<Option<(usize, G::Action)>>,
    pub root: CompactNode,
}

impl<G: Game> Encoding<G> {
    pub fn new(g: &mut G) -> Self {
        let mut enc = Encoding {
            infoset_by_orig: HashMap::new(),
            infosets: Vec::new(),
            nodes: Vec::new(),
            parents: Vec::new(),
            root: 42,
        };
        enc.root = enc.translate_node(g, &mut Vec::new(), &mut [Vec::new(), Vec::new()], None);
        enc
    }
    fn translate_node(
        &mut self,
        g: &mut G,
        h: &mut Vec<G::Action>,
        obs_history: &mut [Vec<(usize, usize)>; 2],
        parent: Option<(CompactNode, G::Action)>,
    ) -> CompactNode {
        self.nodes.push(NodeInfo::Terminal(42.0));
        self.parents.push(parent);
        let result = self.nodes.len() - 1;
        match g.node_info(h) {
            NodeInfo::Terminal(x) => {
                self.nodes[result] = NodeInfo::Terminal(x);
            }
            NodeInfo::Chance(probs) => {
                let mut chance = Vec::with_capacity(probs.len());
                for (prob, action) in probs {
                    h.push(action.clone());
                    chance.push((prob, self.translate_node(g, h, obs_history, Some((result, action)))));
                    h.pop().unwrap();
                }
                self.nodes[result] = NodeInfo::Chance(chance);
            }
            NodeInfo::Choice { player, infoset, actions } => {
                let infosets = &mut self.infosets;
                let infoset_idx = *self.infoset_by_orig.entry(infoset.clone()).or_insert_with(|| {
                    infosets.push(CompactInfoset {
                        orig: infoset.clone(),
                        player: player,
                        actions: actions.clone(),
                        observable_history: obs_history[player].clone(),
                    });
                    infosets.len() - 1
                });
                let ci = &self.infosets[infoset_idx];
                assert_eq!(ci.player, player, "{:?}", infoset);
                assert_eq!(ci.actions, actions);
                assert_eq!(ci.observable_history, obs_history[player], "perfect recall property violated");

                let mut choice_actions = Vec::with_capacity(actions.len());
                for (action_idx, action) in actions.into_iter().enumerate() {
                    h.push(action.clone());
                    obs_history[player].push((infoset_idx, action_idx));
                    choice_actions.push(self.translate_node(g, h, obs_history, Some((result, action))));
                    obs_history[player].pop().unwrap();
                    h.pop();
                }

                self.nodes[result] = NodeInfo::Choice {
                    player: player,
                    infoset: infoset_idx,
                    actions: choice_actions,
                };
            }
        }
        result
    }
}

#[derive(Debug)]
struct CfrEntry {
    total_regret: Vec<f32>,
    total_sigma: Vec<f32>,
    cur_sigma: Vec<f32>,
    tmp: Vec<f32>,
    total_value: f32,
    total_visit_prob: f32,
}

impl CfrEntry {
    fn new(num_actions: usize) -> Self {
        CfrEntry {
            total_regret: vec![0.0; num_actions],
            total_sigma: vec![0.0; num_actions],
            cur_sigma: vec![0.0; num_actions],
            tmp: vec![0.0; num_actions],
            total_value: 42.0,
            total_visit_prob: 42.0,
        }
    }
}

#[derive(Debug)]
pub struct Cfr {
    entries: Vec<CfrEntry>,
}

#[derive(Debug)]
pub struct InfosetStrategy<Action> {
    pub actions: Vec<(Action, f32)>,
    pub expected_value: f32,
    pub visit_prob: f32,
}

impl Cfr {
    pub fn new<G: Game>(enc: &Encoding<G>) -> Self {
        Cfr {
            entries: enc.infosets.iter().map(|i| CfrEntry::new(i.actions.len())).collect(),
        }
    }

    pub fn step<G: Game>(&mut self, enc: &Encoding<G>) {
        self.compute_cur_sigma();
        self.visit(enc, enc.root, [1.0, 1.0], 1.0);
    }

    pub fn get_strategy<G: Game>(&mut self, enc: &Encoding<G>) -> HashMap<G::Infoset, InfosetStrategy<G::Action>> {
        for e in &mut self.entries {
            e.total_value = 0.0;
            e.total_visit_prob = 0.0;
            normalize_to(&e.total_sigma, &mut e.tmp);
        }
        self.visit2(enc, enc.root, [1.0, 1.0], 1.0);
        self.entries.iter().enumerate().map(|(i, e)| {
            let k = enc.infosets[i].orig.clone();
            let v = InfosetStrategy {
                actions: enc.infosets[i].actions.iter().cloned().zip(e.tmp.iter().cloned()).collect(),
                expected_value: e.total_value / e.total_visit_prob.max(1e-9),
                visit_prob: e.total_visit_prob,
            };
            (k, v)
        }).collect()
    }

    fn visit2<G: Game>(&mut self, enc: &Encoding<G>, node: CompactNode, pi: [f32; 2], pi_chance: f32) -> f32 {
        match &enc.nodes[node] {
            NodeInfo::Terminal(x) => *x,
            NodeInfo::Chance(actions) => {
                let mut s = 0.0;
                for &(prob, next_node) in actions {
                    s += prob * self.visit2(enc, next_node, pi, pi_chance * prob);
                }
                s
            }
            &NodeInfo::Choice { player, infoset, ref actions } => {
                let mut s = 0.0;
                for i in 0..actions.len() {
                    let mut pp = pi;
                    pp[player] *= self.entries[infoset].tmp[i];
                    let ev = self.visit2(enc, actions[i], pp, pi_chance);
                    s += ev * self.entries[infoset].tmp[i];
                }
                let t = pi[0] * pi[1] * pi_chance;
                self.entries[infoset].total_visit_prob += t;
                self.entries[infoset].total_value += s * t;
                s
            }
        }
    }

    fn compute_cur_sigma(&mut self) {
        for e in self.entries.iter_mut() {
            normalize_to(&e.total_regret, &mut e.cur_sigma);
        }
    }

    fn visit<G: Game>(&mut self, enc: &Encoding<G>, node: CompactNode, pi: [f32; 2], pi_chance: f32) -> f32 {
        if pi[0] < 1e-7 && pi[1] < 1e-7 {
            return 0.0;
        }
        match &enc.nodes[node] {
            NodeInfo::Terminal(x) => *x,
            NodeInfo::Chance(actions) => {
                let mut s = 0.0;
                for &(prob, next_node) in actions {
                    s += prob * self.visit(enc, next_node, pi, pi_chance * prob);
                }
                s
            }
            &NodeInfo::Choice { player, infoset, ref actions } => {
                let num_actions = actions.len();

                let mut s = 0.0;
                for i in 0..num_actions {
                    let sigma = self.entries[infoset].cur_sigma[i];
                    let mut pp = pi;
                    pp[player] *= sigma;
                    let ev = self.visit(enc, actions[i], pp, pi_chance);
                    s += sigma * ev;
                    self.entries[infoset].tmp[i] = ev;
                }
                let entry = &mut self.entries[infoset];

                let factor = pi_chance * pi[1 - player] * if player == 0 { 1.0 } else { -1.0 };
                for i in 0..num_actions {
                    entry.total_regret[i] += factor * (entry.tmp[i] - s);
                }

                for i in 0..num_actions {
                    entry.total_sigma[i] += pi[player] * entry.cur_sigma[i];
                }
                s
            }
        }
    }
}

fn normalize_to(xs: &[f32], dst: &mut [f32]) {
    let s: f32 = xs.iter().map(|x| x.max(0.0)).sum();
    assert_eq!(xs.len(), dst.len());
    if s == 0.0 {
        let q = 1.0 / dst.len() as f32;
        for y in dst.iter_mut() {
            *y = q;
        }
    } else {
        let q = 1.0 / s;
        for (x, y) in xs.iter().zip(dst.iter_mut()) {
            *y = x.max(0.0) * q;
        }
    }
}
