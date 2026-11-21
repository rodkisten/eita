//! Apples-to-apples *search-only* comparison on the same in-memory strings.
//! These numbers exclude disk access, catalog generation, process startup and UI.
use crate::catalog::Candidate;
use frizbee::{Config as FrizbeeConfig, Matcher as FrizbeeMatcher};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config as NucleoConfig, Matcher as NucleoMatcher,
};
use serde::Serialize;
use std::hint::black_box;
use std::time::Instant;

#[derive(Serialize)]
struct Stats {
    algorithm: &'static str,
    runs: usize,
    candidates: usize,
    p50_us: u128,
    p95_us: u128,
    p99_us: u128,
    matches_last: usize,
}

fn summarize(
    algorithm: &'static str,
    candidates: usize,
    mut micros: Vec<u128>,
    matches_last: usize,
) -> Stats {
    micros.sort_unstable();
    let percentile = |n: usize| micros[(micros.len() - 1) * n / 100];
    Stats {
        algorithm,
        runs: micros.len(),
        candidates,
        p50_us: percentile(50),
        p95_us: percentile(95),
        p99_us: percentile(99),
        matches_last,
    }
}

pub fn compare(
    catalog: &[Candidate],
    iterations: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    if iterations == 0 || iterations > 100_000 {
        return Err("iterations must be between 1 and 100000".into());
    }
    let labels: Vec<&str> = catalog
        .iter()
        .map(|candidate| candidate.value.as_str())
        .collect();
    let queries = [
        "git com",
        "docker comp",
        "npm run",
        "cargo bu",
        "kubectl logs",
    ];
    let mut frizbee = Vec::with_capacity(iterations);
    let mut nucleo = Vec::with_capacity(iterations);
    let mut frizbee_matches = 0;
    let mut nucleo_matches = 0;
    let mut matcher = NucleoMatcher::new(NucleoConfig::DEFAULT);
    for index in 0..(iterations + 20) {
        let query = queries[index % queries.len()];
        let begin = Instant::now();
        let mut finder = FrizbeeMatcher::new(query, &FrizbeeConfig::default());
        frizbee_matches = black_box(finder.match_list(&labels)).len();
        let elapsed = begin.elapsed().as_micros();
        if index >= 20 {
            frizbee.push(elapsed);
        }

        let begin = Instant::now();
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        nucleo_matches = black_box(pattern.match_list(labels.iter().copied(), &mut matcher)).len();
        let elapsed = begin.elapsed().as_micros();
        if index >= 20 {
            nucleo.push(elapsed);
        }
    }
    Ok(serde_json::to_string_pretty(&[
        summarize("frizbee", labels.len(), frizbee, frizbee_matches),
        summarize("nucleo-matcher", labels.len(), nucleo, nucleo_matches),
    ])?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_contains_both_algorithms() {
        let items = vec![Candidate {
            value: "git commit".into(),
            description: "".into(),
            source: "test".into(),
        }];
        let report = compare(&items, 2).unwrap();
        assert!(report.contains("frizbee"));
        assert!(report.contains("nucleo-matcher"));
    }
}
