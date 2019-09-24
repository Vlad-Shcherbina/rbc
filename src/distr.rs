use std::hash::Hash;
use std::collections::HashMap;
use rand::prelude::*;

pub fn draw<'a, T>(distr: &'a [(T, f32)], rng: &mut impl Rng) -> &'a T {
    let d = rand::distributions::WeightedIndex::new(distr.iter().map(|&(_, w)| w)).unwrap();
    &distr[d.sample(rng)].0
}

pub fn normalize<T>(distr: &mut [(T, f32)]) {
    let s: f32 = distr.iter().map(|&(_, p)| p).sum();
    assert!(s > 0.0);
    let inv_s = 1.0 / s;
    for (_, p) in distr {
        *p *= inv_s;
    }
}

pub fn draw_correlated<'a, T: std::fmt::Debug + Hash + Eq>(
    distr1: &'a [(T, f32)],
    distr2: &'a [(T, f32)],
    rng: &mut impl Rng,
) -> (&'a T, &'a T) {
    let h1: HashMap<&T, f32> = distr1.iter().map(|(t, p)| (t, *p)).collect();
    let h2: HashMap<&T, f32> = distr2.iter().map(|(t, p)| (t, *p)).collect();
    let mut common = Vec::with_capacity(distr1.len());
    let mut unique1 = Vec::with_capacity(distr1.len());
    let mut unique2 = Vec::with_capacity(distr2.len());
    for &(ref t, p1) in distr1 {
        let p2 = h2.get(&t).cloned().unwrap_or(0.0);
        common.push(p1.min(p2));
        unique1.push(p1 - p1.min(p2));
    }
    for &(ref t, p2) in distr2 {
        let p1 = h1.get(&t).cloned().unwrap_or(0.0);
        unique2.push(p2 - p2.min(p1));
    }

    let p_common = common.iter().sum::<f32>();
    let p_unique1 = unique1.iter().sum::<f32>();
    let p_unique2 = unique2.iter().sum::<f32>();
    assert!((p_common + p_unique1 - 1.0).abs() < 1e-4, "{}, {}", p_common, p_unique1);
    assert!((p_common + p_unique2 - 1.0).abs() < 1e-4, "{}, {}", p_common, p_unique2);

    if rng.gen_bool(p_common.min(1.0) as f64) {
        let d = rand::distributions::WeightedIndex::new(&common).unwrap();
        let t = &distr1[d.sample(rng)].0;
        (t, t)
    } else {
        let d1 = rand::distributions::WeightedIndex::new(&unique1).unwrap();
        let d2 = rand::distributions::WeightedIndex::new(&unique2).unwrap();
        (&distr1[d1.sample(rng)].0, &distr2[d2.sample(rng)].0)
    }
}
