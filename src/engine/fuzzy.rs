use std::{cmp, mem::swap};

pub fn distance(s1_in: &str, s2_in: &str) -> usize {
    if s1_in == s2_in {
        return 0;
    }

    let s1: &str;
    let mut s1_len = s1_in.chars().count();

    let s2: &str;
    let mut s2_len = s2_in.chars().count();

    if s2_len > s1_len {
        s1 = s2_in;
        s2 = s1_in;
        swap(&mut s1_len, &mut s2_len);
    } else {
        s1 = s1_in;
        s2 = s2_in;
    }
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

pub fn score(needle: &str, hay: &str) -> f64 {
    let needle_tokens: Vec<_> = needle.split_whitespace().collect();
    let hay_tokens: Vec<_> = hay.split_whitespace().collect();
    let tokens = needle_tokens.len();
    let runs = cmp::min(needle_tokens.len(), hay_tokens.len());
    let mut score: f64 = 0.0;
    for i in 0..runs {
        score += score_inner(needle_tokens[i], hay_tokens[i]);
    }

    if score == 0.0 {
        0.0
    } else {
        score / tokens as f64
    }
}

pub fn score_deep(needle: &str, hay: &str) -> f64 {
    let needle_tokens: Vec<_> = needle.split_whitespace().collect();
    let hay_tokens: Vec<_> = hay.split_whitespace().collect();
    let tokens = needle_tokens.len();
    let runs = cmp::min(needle_tokens.len(), hay_tokens.len());
    let mut score: f64 = 0.0;
    for i in 0..runs {
        score += score_inner(needle_tokens[i], hay_tokens[i]);
    }

    score = if score == 0.0 {
        0.0
    } else {
        score / tokens as f64
    };
    score += score_inner(needle, hay);
    if score == 0.0 { 0.0 } else { score / 2.0 }
}

fn score_inner(s1: &str, s2: &str) -> f64 {
    let dist = distance(s1, s2);
    if dist == 0 {
        1.0
    } else {
        1.0 - (distance(s1, s2) as f64 / cmp::max(s1.chars().count(), s2.chars().count()) as f64)
    }
}
