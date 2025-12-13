use ontrack::engine::fuzzy;

#[test]
fn fuzzy_empty_vs_empty() {
    let dist = fuzzy::distance("", "");
    assert_eq!(dist, 0);
}

#[test]
fn fuzzy_empty_vs_nonempty() {
    let dist = fuzzy::distance("", "abc");
    assert_eq!(dist, 3);
}

#[test]
fn fuzzy_nonempty_vs_empty() {
    let dist = fuzzy::distance("abc", "");
    assert_eq!(dist, 3);
}

#[test]
fn fuzzy_completely_different() {
    let dist = fuzzy::distance("kitten", "orange");
    assert_eq!(dist, 6);
}

#[test]
fn fuzzy_substitution() {
    let dist = fuzzy::distance("cat", "cut");
    assert_eq!(dist, 1);
}

#[test]
fn fuzzy_insertion() {
    let dist = fuzzy::distance("cat", "cart");
    assert_eq!(dist, 1);
}

#[test]
fn fuzzy_deletion() {
    let dist = fuzzy::distance("cart", "cat");
    assert_eq!(dist, 1);
}

#[test]
fn fuzzy_unicode_equal() {
    let dist = fuzzy::distance("café", "café");
    assert_eq!(dist, 0);
}

#[test]
fn fuzzy_unicode_distinct() {
    let dist = fuzzy::distance("café", "cafe");
    assert_eq!(dist, 1);
}

#[test]
fn fuzzy_unicode_multi() {
    let dist = fuzzy::distance("😀😁😂", "😀😂😁");
    assert_eq!(dist, 2);
}

#[test]
fn fuzzy_prefix_changes() {
    let dist = fuzzy::distance("abcdef", "zbcdef");
    assert_eq!(dist, 1);
}

#[test]
fn fuzzy_suffix_changes() {
    let dist = fuzzy::distance("abcdef", "abcdez");
    assert_eq!(dist, 1);
}

#[test]
fn fuzzy_longer_sequence() {
    let dist = fuzzy::distance("intention", "execution");
    assert_eq!(dist, 5);
}
