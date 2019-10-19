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
    type Action;
    type Infoset;

    fn node_info(&self, h: &[Self::Action]) -> NodeInfo<Self::Action, Self::Infoset>;
}

type CompactNode = usize;

#[derive(Debug)]
struct CompactInfoset<OrigAction, OrigInfoset>
{
    orig: OrigInfoset,
    player: usize,
    actions: Vec<OrigAction>,

    observable_history: Vec<(usize, usize)>,
    // Pairs (infoset, action) for all past choices by this player.
    // To check perfect recall property.
}

#[derive(Debug)]
pub struct Encoding<G: Game>
where
    G::Infoset: Eq + std::hash::Hash,
    G::Action: std::fmt::Debug,
{
    infoset_by_orig: HashMap<G::Infoset, usize>,
    infosets: Vec<CompactInfoset<G::Action, G::Infoset>>,
    nodes: Vec<NodeInfo</*Action*/CompactNode, /*Infoset*/usize>>,
    parents: Vec<Option<(usize, G::Action)>>,
    root: CompactNode,
}

impl<G: Game> Encoding<G>
where
    G::Action: Clone + Eq + std::hash::Hash + std::fmt::Debug,
    G::Infoset: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    pub fn new(g: &G) -> Self {
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
        g: &G,
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
                assert_eq!(ci.player, player);
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
