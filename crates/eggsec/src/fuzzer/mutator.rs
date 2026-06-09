use crate::utils::urlencoding;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationType {
    CaseToggle,
    UrlEncode,
    DoubleUrlEncode,
    NullByte,
    Duplicate,
    Truncate,
    Prefix,
    Suffix,
    Comment,
    Whitespace,
    Reverse,
    Swap,
}

#[derive(Debug, Clone)]
pub struct Mutator {
    mutation_types: Vec<MutationType>,
    rng: StdRng,
}

impl Default for Mutator {
    fn default() -> Self {
        Self::new()
    }
}

impl Mutator {
    pub fn new() -> Self {
        Self {
            mutation_types: vec![
                MutationType::CaseToggle,
                MutationType::UrlEncode,
                MutationType::DoubleUrlEncode,
                MutationType::NullByte,
                MutationType::Duplicate,
                MutationType::Truncate,
                MutationType::Prefix,
                MutationType::Suffix,
                MutationType::Comment,
                MutationType::Whitespace,
            ],
            rng: StdRng::from_entropy(),
        }
    }

    pub fn with_mutation_types(types: Vec<MutationType>) -> Self {
        Self {
            mutation_types: types,
            rng: StdRng::from_entropy(),
        }
    }

    pub fn mutate(&mut self, payload: &str) -> Vec<String> {
        let mutation_types = self.mutation_types.clone();
        let mut mutations = Vec::new();

        for mutation_type in &mutation_types {
            if let Some(mutated) = self.apply_mutation(payload, mutation_type) {
                mutations.push(mutated);
            }
        }

        mutations
    }

    pub fn mutate_random(&mut self, payload: &str) -> Option<String> {
        if self.mutation_types.is_empty() {
            return None;
        }

        let idx = self.rng.gen_range(0..self.mutation_types.len());
        let mutation_type = self.mutation_types[idx];
        self.apply_mutation(payload, &mutation_type)
    }

    fn apply_mutation(&mut self, payload: &str, mutation_type: &MutationType) -> Option<String> {
        match mutation_type {
            MutationType::CaseToggle => Some(self.case_toggle(payload)),
            MutationType::UrlEncode => Some(self.url_encode(payload)),
            MutationType::DoubleUrlEncode => Some(self.double_url_encode(payload)),
            MutationType::NullByte => Some(self.add_null_byte(payload)),
            MutationType::Duplicate => Some(self.duplicate(payload)),
            MutationType::Truncate => self.truncate(payload),
            MutationType::Prefix => Some(self.add_prefix(payload)),
            MutationType::Suffix => Some(self.add_suffix(payload)),
            MutationType::Comment => Some(self.add_comment(payload)),
            MutationType::Whitespace => Some(self.add_whitespace(payload)),
            MutationType::Reverse => Some(self.reverse(payload)),
            MutationType::Swap => Some(self.swap_chars(payload)),
        }
    }

    fn case_toggle(&mut self, payload: &str) -> String {
        payload
            .chars()
            .map(|c| {
                if self.rng.gen_bool(0.5) {
                    c.to_ascii_uppercase()
                } else {
                    c.to_ascii_lowercase()
                }
            })
            .collect()
    }

    fn url_encode(&self, payload: &str) -> String {
        urlencoding::encode(payload)
    }

    fn double_url_encode(&self, payload: &str) -> String {
        let first = urlencoding::encode(payload);
        urlencoding::encode(&first)
    }

    fn add_null_byte(&self, payload: &str) -> String {
        format!("{}\x00", payload)
    }

    fn duplicate(&mut self, payload: &str) -> String {
        let times = self.rng.gen_range(2..=3);
        payload.repeat(times)
    }

    fn truncate(&mut self, payload: &str) -> Option<String> {
        let char_count = payload.chars().count();
        if char_count > 3 {
            let new_len = self.rng.gen_range(1..char_count - 1);
            Some(payload.chars().take(new_len).collect())
        } else {
            None
        }
    }

    fn add_prefix(&mut self, payload: &str) -> String {
        let prefixes = ["x", "test", "admin", "../", "./", "//"];
        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
        format!("{}{}", prefix, payload)
    }

    fn add_suffix(&mut self, payload: &str) -> String {
        let suffixes = ["x", ".bak", ".old", ".txt", "/..", "%00", "//"];
        let suffix = suffixes[self.rng.gen_range(0..suffixes.len())];
        format!("{}{}", payload, suffix)
    }

    fn add_comment(&mut self, payload: &str) -> String {
        let comments = ["/**/", "/*!50000*/", "--", "#", "/*comment*/"];
        let comment = comments[self.rng.gen_range(0..comments.len())];

        let char_count = payload.chars().count();
        if char_count > 1 {
            let mid = char_count / 2;
            let (prefix, suffix): (String, String) = (
                payload.chars().take(mid).collect(),
                payload.chars().skip(mid).collect(),
            );
            format!("{}{}{}", prefix, comment, suffix)
        } else {
            format!("{}{}", comment, payload)
        }
    }

    fn add_whitespace(&mut self, payload: &str) -> String {
        let whitespace = [" ", "\t", "\n", "\r\n", "%09", "%0a", "%0d"];
        let ws = whitespace[self.rng.gen_range(0..whitespace.len())];

        let char_count = payload.chars().count();
        if char_count > 1 {
            let pos = self.rng.gen_range(1..char_count);
            let (prefix, suffix): (String, String) = (
                payload.chars().take(pos).collect(),
                payload.chars().skip(pos).collect(),
            );
            format!("{}{}{}", prefix, ws, suffix)
        } else {
            format!("{}{}", payload, ws)
        }
    }

    fn reverse(&self, payload: &str) -> String {
        payload.chars().rev().collect()
    }

    fn swap_chars(&mut self, payload: &str) -> String {
        let mut chars: Vec<char> = payload.chars().collect();
        if chars.len() >= 2 {
            let i = self.rng.gen_range(0..chars.len() - 1);
            chars.swap(i, i + 1);
        }
        chars.into_iter().collect()
    }
}

pub fn generate_mutations(payload: &str, count: usize) -> Vec<String> {
    let mut mutator = Mutator::new();
    let mut mutations = vec![payload.to_string()];

    for _ in 0..count {
        if let Some(mutated) = mutator.mutate_random(payload) {
            if !mutations.contains(&mutated) {
                mutations.push(mutated);
            }
        }
    }

    mutations.truncate(count + 1);
    mutations
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_generate_mutations_count() {
        let mutations = generate_mutations("' OR 1=1--", 5);
        assert!(mutations.len() <= 6); // original + up to 5 mutations
        assert!(mutations.len() >= 1); // at least the original
    }

    #[test]
    fn test_generate_mutations_includes_original() {
        let mutations = generate_mutations("test", 3);
        assert_eq!(mutations[0], "test");
    }

    #[test]
    fn test_generate_mutations_unique() {
        let mutations = generate_mutations("' OR 1=1--", 10);
        let mut unique = mutations.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(mutations.len(), unique.len());
    }

    proptest! {
        #[test]
        fn test_generate_mutations_returns_at_most_count_plus_one(payload in "[ -~]{1,30}", count in 1usize..10) {
            let mutations = generate_mutations(&payload, count);
            prop_assert!(mutations.len() <= count + 1);
        }
    }
}
