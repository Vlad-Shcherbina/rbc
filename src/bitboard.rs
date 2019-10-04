struct BitsIter(u64);

impl Iterator for BitsIter {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let res = self.0 & self.0.wrapping_neg();
            self.0 &= self.0 - 1;
            Some(res)
        }
    }
}

pub fn iter_one_positions(x: u64) -> impl Iterator<Item=u32> {
    BitsIter(x).map(u64::trailing_zeros)
}

#[cfg(test)]
#[test]
fn test_bits_iter() {
    let ones: Vec<_> = iter_one_positions(0b10010).collect();
    assert_eq!(ones, [1, 4]);
}

pub fn render_bb(x: u64) -> Vec<String> {
    let mut result = Vec::new();
    for rank in (0..8).rev() {
        let mut s = String::new();
        for file in 0..8 {
            let pos = rank * 8 + file;
            s.push(' ');
            if (x >> pos) & 1 != 0 {
                s.push('1');
            } else {
                s.push('.');
            }
        }
        s.push(' ');
        result.push(s);
    }
    result
}

fn in_between(from: i8, to: i8) -> Option<u64> {
    if from == to {
        return None;
    }
    let mut rank = from / 8;
    let mut file = from % 8;
    let dr = to / 8 - rank;
    let df = to % 8 - file;
    if dr != 0 && df != 0 && dr.abs() != df.abs() {
        return None;
    }
    let dr = dr.signum();
    let df = df.signum();
    let mut result = 0;
    loop {
        rank += dr;
        file += df;
        let pos = rank * 8 + file;
        if pos == to {
            break;
        }
        result |= 1 << pos;
    }
    Some(result)
}

#[cfg(test)]
#[test]
fn test_in_between() {
    assert!(in_between(8, 2).is_none());
    dbg!(render_bb(in_between(0, 36).unwrap()));
}

fn attacks_by_pred(pred: &dyn Fn(i8, i8) -> bool) -> [u64; 64] {
    let mut res = [0; 64];
    for from in 0..64i8 {
        for to in 0..64i8 {
            if from == to {
                continue;
            }
            let dr = from / 8 - to / 8;
            let df = from % 8 - to % 8;
            if pred(dr, df) {
                res[from as usize] |= 1 << to;
            }
        }
    }
    res
}

lazy_static::lazy_static! {
    pub static ref IN_BETWEEN: [u64; 64 * 64] = {
        let mut res = [0; 64 * 64];
        for from in 0..64 {
            for to in 0..64 {
                if let Some(b) = in_between(from, to) {
                    res[from as usize * 64 + to as usize] = b;
                }
            }
        }
        res
    };

    pub static ref KNIGHT_ATTACKS: [u64; 64] =
        attacks_by_pred(&|dr, df| dr.abs().min(df.abs()) == 1 && dr.abs().max(df.abs()) == 2);

    pub static ref BISHOP_ATTACKS: [u64; 64] =
        attacks_by_pred(&|dr, df| dr.abs() == df.abs());

    pub static ref ROOK_ATTACKS: [u64; 64] =
        attacks_by_pred(&|dr, df| dr == 0 || df == 0);

    pub static ref KING_ATTACKS: [u64; 64] =
        attacks_by_pred(&|dr, df| dr.abs() <= 1 && df.abs() <= 1);
}

#[cfg(test)]
#[test]
fn test_render_bb() {
    dbg!(render_bb(KING_ATTACKS[36]));
}
