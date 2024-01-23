use std::{cmp::Ordering, collections::BinaryHeap};

#[derive(Debug, Default, Clone)]
pub struct Leaderboard {
    entries: BinaryHeap<LeaderboardEntry>,
}

impl Leaderboard {
    pub fn new() -> Self {
        Leaderboard::default()
    }

    pub fn rankings(&self) -> impl Iterator<Item = LeaderboardEntry> {
        let mut entries = self.entries.clone();
        std::iter::from_fn(move || entries.pop())
    }

    pub fn update(&mut self, entry: LeaderboardEntry) {
        let mut new = BinaryHeap::new();

        let mut updated = false;
        while let Some(current) = self.entries.pop() {
            if current.user_id == entry.user_id {
                new.push(LeaderboardEntry {
                    score: current.score.max(entry.score),
                    ..current
                });
                updated = true;
            } else {
                new.push(current);
            }
        }

        if !updated {
            new.push(entry);
        }

        self.entries = new;
    }
}

#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub score: u32,
    pub username: String,
    pub user_id: i64,
}

impl PartialEq for LeaderboardEntry {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for LeaderboardEntry {}

impl PartialOrd for LeaderboardEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LeaderboardEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(user_id: i64, score: u32) -> LeaderboardEntry {
        LeaderboardEntry {
            score,
            username: user_id.to_string(),
            user_id,
        }
    }

    #[test]
    fn leaderboard() {
        let mut leaderboard = Leaderboard::new();

        // Initial rankings
        for entry in [entry(1, 100), entry(2, 250), entry(3, 500)] {
            leaderboard.update(entry);
        }
        assert_eq!(
            leaderboard.rankings().collect::<Vec<_>>(),
            vec![entry(3, 500), entry(2, 250), entry(1, 100)]
        );

        // Insert a new entry
        leaderboard.update(entry(4, 400));
        assert_eq!(
            leaderboard.rankings().collect::<Vec<_>>(),
            vec![entry(3, 500), entry(4, 400), entry(2, 250), entry(1, 100)]
        );

        // Update an entry
        leaderboard.update(entry(1, 300));
        assert_eq!(
            leaderboard.rankings().collect::<Vec<_>>(),
            vec![entry(3, 500), entry(4, 400), entry(1, 300), entry(2, 250)]
        );
    }
}
