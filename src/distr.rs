use rand::prelude::*;

pub fn draw<'a, T>(distr: &'a [(T, f32)], rng: &mut impl Rng) -> &'a T {
    let d = rand::distributions::WeightedIndex::new(distr.iter().map(|&(_, w)| w)).unwrap();
    &distr[d.sample(rng)].0
}
