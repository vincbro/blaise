use std::cmp;

pub fn distance(s1_in: &str, s2_in: &str) -> usize {
    if s1_in == s2_in {
        return 0;
    }

    let s1: &str;
    let s2: &str;

    if s2_in.chars().count() > s1_in.chars().count() {
        s1 = s2_in;
        s2 = s1_in;
    } else {
        s1 = s1_in;
        s2 = s2_in;
    }

    let s1_len = s1.chars().count();
    let s2_len = s2.chars().count();
    assert!(s1_len >= s2_len);

    let mut matrix = vec![vec![0usize; s2_len + 1]; s1_len + 1];
    (0..cmp::max(s1_len, s2_len) + 1).for_each(|i| {
        if s1_len >= i {
            matrix[i][0] = i;
        }
        if s2_len >= i {
            matrix[0][i] = i;
        }
    });

    s2.chars().enumerate().for_each(|(j, jc)| {
        s1.chars().enumerate().for_each(|(i, ic)| {
            let sub_cost = if ic == jc { 0 } else { 1 };
            let a = matrix[i][j + 1] + 1;
            let b = matrix[i + 1][j] + 1;
            let c = matrix[i][j] + sub_cost;
            matrix[i + 1][j + 1] = cmp::min(a, cmp::min(b, c));
        });
    });
    matrix[s1_len][s2_len]
}

pub fn score(s1: &str, s2: &str) -> f64 {
    let dist = distance(s1, s2);
    if dist == 0 {
        1.0
    } else {
        1.0 - (distance(s1, s2) as f64 / cmp::max(s1.chars().count(), s2.chars().count()) as f64)
    }
}
