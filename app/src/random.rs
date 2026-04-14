use std::collections::HashMap;

use rand::{seq::SliceRandom, Rng};

pub(crate) fn random_index(len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }

    Some(rand::thread_rng().gen_range(0..len))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RandomIndexDeck {
    len: usize,
    remaining: Vec<usize>,
}

impl RandomIndexDeck {
    fn new(len: usize) -> Self {
        let mut remaining = (0..len).collect::<Vec<_>>();
        remaining.shuffle(&mut rand::thread_rng());
        Self { len, remaining }
    }

    fn next_index(&mut self) -> usize {
        self.remaining
            .pop()
            .expect("random index deck should contain at least one element")
    }
}

#[derive(Debug, Default)]
pub(crate) struct RandomIndexDecks {
    decks: HashMap<String, RandomIndexDeck>,
}

impl RandomIndexDecks {
    pub(crate) fn next_index(&mut self, query: Option<&str>, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }

        let key = normalize_query_key(query);
        let deck = self
            .decks
            .entry(key)
            .or_insert_with(|| RandomIndexDeck::new(len));
        if deck.len != len || deck.remaining.is_empty() {
            *deck = RandomIndexDeck::new(len);
        }

        Some(deck.next_index())
    }
}

fn normalize_query_key(query: Option<&str>) -> String {
    let mut terms = query
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(|query| {
            query
                .split_whitespace()
                .map(|term| term.to_lowercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    terms.sort();
    terms.dedup();
    terms.join("\n")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{random_index, RandomIndexDecks};

    #[test]
    fn random_index_returns_none_for_empty_input() {
        assert_eq!(random_index(0), None);
    }

    #[test]
    fn random_index_returns_zero_for_single_candidate() {
        for _ in 0..32 {
            assert_eq!(random_index(1), Some(0));
        }
    }

    #[test]
    fn random_index_stays_within_bounds() {
        for _ in 0..256 {
            let index = random_index(7).expect("non-empty input should always produce an index");
            assert!(index < 7);
        }
    }

    #[test]
    fn random_index_decks_return_none_for_empty_candidates() {
        let mut decks = RandomIndexDecks::default();

        assert_eq!(decks.next_index(Some("pad"), 0), None);
    }

    #[test]
    fn random_index_decks_do_not_repeat_within_a_cycle() {
        let mut decks = RandomIndexDecks::default();
        let mut seen = HashSet::new();

        for _ in 0..160 {
            let index = decks
                .next_index(Some("pad"), 160)
                .expect("non-empty candidates should return an index");
            assert!(
                seen.insert(index),
                "duplicate index returned within the same cycle"
            );
        }

        assert_eq!(seen.len(), 160);
    }

    #[test]
    fn random_index_decks_keep_progress_per_query() {
        let mut decks = RandomIndexDecks::default();

        let pad_first = decks.next_index(Some("pad"), 3).unwrap();
        let bass_first = decks.next_index(Some("bass"), 2).unwrap();
        let pad_second = decks.next_index(Some("pad"), 3).unwrap();
        let bass_second = decks.next_index(Some("bass"), 2).unwrap();
        let pad_third = decks.next_index(Some("pad"), 3).unwrap();

        assert_ne!(pad_first, pad_second);
        assert_ne!(pad_first, pad_third);
        assert_ne!(pad_second, pad_third);
        assert_ne!(bass_first, bass_second);
    }

    #[test]
    fn random_index_decks_normalize_query_terms_for_the_same_cycle() {
        let mut decks = RandomIndexDecks::default();
        let mut seen = HashSet::new();

        for query in [Some("Pad Soft"), Some("soft pad"), Some("pad  soft  pad")] {
            let index = decks.next_index(query, 3).unwrap();
            assert!(
                seen.insert(index),
                "equivalent queries should share the same deck state"
            );
        }

        assert_eq!(seen.len(), 3);
    }

    #[test]
    fn random_index_decks_reset_after_a_cycle_is_exhausted() {
        let mut decks = RandomIndexDecks::default();
        let first = decks.next_index(Some("pad"), 1).unwrap();
        let second = decks.next_index(Some("pad"), 1).unwrap();

        assert_eq!(first, 0);
        assert_eq!(second, 0);
    }
}
