use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

pub(crate) mod fuzzy;
pub mod geo;
pub mod time;

pub trait Identifiable {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn normalized_name(&self) -> &str;
}

/// Generic fuzzy search function built for multithreaded searching.
pub fn search<'a, T>(needle: &'a str, haystack: &'a [T]) -> Vec<&'a T>
where
    T: Send + Sync + Identifiable,
{
    let normalized_needle = needle.to_lowercase();
    let threads = rayon::current_num_threads();
    let chunk_size = haystack.len().div_ceil(threads);
    let mut results: Vec<Vec<(&T, f64)>> = Vec::with_capacity(threads);
    for _ in 0..threads {
        results.push(Vec::with_capacity(chunk_size));
    }
    results.par_iter_mut().enumerate().for_each(|(chunk, vec)| {
        for i in 0..chunk_size {
            let index = (chunk * chunk_size) + i;
            if index > haystack.len() - 1 {
                break;
            }
            let hay = &haystack[index];
            let score = fuzzy::score(&normalized_needle, hay.normalized_name());
            vec.push((hay, score));
        }
    });
    let mut results: Vec<_> = results.into_iter().flatten().collect();
    results.sort_by(|(_, score_a), (_, score_b)| score_b.total_cmp(score_a));
    results.iter().map(|(entity, _)| *entity).collect()
}
