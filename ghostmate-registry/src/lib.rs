//! Pure-Rust registry logic for the GhostMate MatchRegistry.
//!
//! `no_std`-compatible (uses `alloc` for collections) so it can be embedded
//! in the Miden WASM contract if needed.  Tests compile with std via cfg_attr.
#![cfg_attr(not(test), no_std)]

#[cfg(not(test))]
extern crate alloc;

#[cfg(not(test))]
use alloc::vec::Vec;

// ── Status constants (shared with the Miden contract) ─────────────────────────

pub const CHALLENGE_OPEN:      u64 = 0;
pub const CHALLENGE_ACCEPTED:  u64 = 1;
pub const CHALLENGE_CANCELLED: u64 = 2;

pub const GAME_ACTIVE:     u64 = 0;
pub const GAME_WHITE_WINS: u64 = 1;
pub const GAME_BLACK_WINS: u64 = 2;
pub const GAME_DRAW:       u64 = 3;

// Sentinel stored in agents_stats[3] to distinguish registered from default-zero.
pub const REGISTERED: u64 = 1;

// ── Data types ────────────────────────────────────────────────────────────────

/// Per-agent statistics and identity.
///
/// On the Miden side these are stored as two `StorageMap<Word,Word>` entries:
///   - agents_name:  [prefix,0,0,0] → name (4 u64 packed)
///   - agents_stats: [prefix,0,0,0] → [wins, losses, draws, REGISTERED]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentStats {
    pub prefix: u64,
    pub name:   [u64; 4],
    pub wins:   u64,
    pub losses: u64,
    pub draws:  u64,
}

impl AgentStats {
    /// Classical tournament score: 1 point per win, ½ per draw (stored as 2×).
    pub fn score(&self) -> u64 {
        self.wins * 2 + self.draws
    }

    pub fn total_games(&self) -> u64 {
        self.wins + self.losses + self.draws
    }

    /// Return a copy updated for one game result.
    /// `is_white` — whether this agent played white in that game.
    pub fn apply_result(&self, is_white: bool, result: u64) -> Self {
        let mut s = self.clone();
        match result {
            GAME_WHITE_WINS => if is_white { s.wins += 1 } else { s.losses += 1 },
            GAME_BLACK_WINS => if !is_white { s.wins += 1 } else { s.losses += 1 },
            GAME_DRAW       => { s.draws += 1 }
            _               => {}
        }
        s
    }
}

/// An open or resolved challenge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChallengeRecord {
    pub id:                u64,
    pub challenger_prefix: u64,
    pub wager_amount:      u64,
    pub status:            u64,
}

impl ChallengeRecord {
    pub fn is_open(&self)      -> bool { self.status == CHALLENGE_OPEN }
    pub fn is_accepted(&self)  -> bool { self.status == CHALLENGE_ACCEPTED }
    pub fn is_cancelled(&self) -> bool { self.status == CHALLENGE_CANCELLED }
}

/// A game record created when a challenge is accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameRecord {
    pub id:                   u64,
    pub white_prefix:         u64,
    pub black_prefix:         u64,
    pub status:               u64,
    pub game_contract_prefix: u64,
}

impl GameRecord {
    pub fn is_active(&self) -> bool { self.status == GAME_ACTIVE }

    /// Returns the prefix of the winning agent, or `None` for draw/active.
    pub fn winner_prefix(&self) -> Option<u64> {
        match self.status {
            GAME_WHITE_WINS => Some(self.white_prefix),
            GAME_BLACK_WINS => Some(self.black_prefix),
            _               => None,
        }
    }
}

// ── Leaderboard logic ─────────────────────────────────────────────────────────

/// Sort agents for leaderboard display.
///
/// Primary:   wins  descending  (more wins = higher rank)
/// Secondary: losses ascending  (fewer losses breaks ties)
/// Tertiary:  draws descending  (more draws breaks further ties)
pub fn sort_leaderboard(agents: &mut Vec<AgentStats>) {
    agents.sort_by(|a, b| {
        b.wins.cmp(&a.wins)
            .then_with(|| a.losses.cmp(&b.losses))
            .then_with(|| b.draws.cmp(&a.draws))
    });
}

/// Return the 1-based leaderboard rank of `prefix`, or `None` if not found.
pub fn leaderboard_rank(agents: &[AgentStats], prefix: u64) -> Option<usize> {
    let mut sorted = agents.to_vec();
    sort_leaderboard(&mut sorted);
    sorted.iter().position(|a| a.prefix == prefix).map(|i| i + 1)
}

/// Apply a settled game result to the relevant agents in `stats`.
///
/// Returns the number of agent records that were updated (0, 1, or 2).
pub fn apply_game_result(
    stats:        &mut Vec<AgentStats>,
    white_prefix: u64,
    black_prefix: u64,
    result:       u64,
) -> usize {
    let mut updated = 0;
    for agent in stats.iter_mut() {
        if agent.prefix == white_prefix {
            *agent = agent.apply_result(true, result);
            updated += 1;
        } else if agent.prefix == black_prefix {
            *agent = agent.apply_result(false, result);
            updated += 1;
        }
    }
    updated
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(prefix: u64, wins: u64, losses: u64, draws: u64) -> AgentStats {
        AgentStats { prefix, name: [0; 4], wins, losses, draws }
    }

    // ── AgentStats ───────────────────────────────────────────────────────────

    #[test]
    fn score_is_2x_wins_plus_draws() {
        let a = make_agent(1, 3, 1, 2);
        assert_eq!(a.score(), 3 * 2 + 2);
    }

    #[test]
    fn total_games_sums_all_results() {
        let a = make_agent(1, 3, 1, 2);
        assert_eq!(a.total_games(), 6);
    }

    #[test]
    fn apply_result_win_as_white() {
        let a = make_agent(1, 0, 0, 0);
        let a2 = a.apply_result(true, GAME_WHITE_WINS);
        assert_eq!(a2.wins, 1);
        assert_eq!(a2.losses, 0);
    }

    #[test]
    fn apply_result_loss_as_white() {
        let a = make_agent(1, 0, 0, 0);
        let a2 = a.apply_result(true, GAME_BLACK_WINS);
        assert_eq!(a2.losses, 1);
        assert_eq!(a2.wins, 0);
    }

    #[test]
    fn apply_result_win_as_black() {
        let a = make_agent(1, 0, 0, 0);
        let a2 = a.apply_result(false, GAME_BLACK_WINS);
        assert_eq!(a2.wins, 1);
        assert_eq!(a2.losses, 0);
    }

    #[test]
    fn apply_result_draw_increments_draws() {
        let a = make_agent(1, 0, 0, 0);
        let a2 = a.apply_result(true, GAME_DRAW);
        assert_eq!(a2.draws, 1);
        assert_eq!(a2.wins,  0);
        assert_eq!(a2.losses,0);
    }

    #[test]
    fn apply_result_is_non_mutating() {
        let original = make_agent(1, 5, 2, 1);
        let _updated = original.apply_result(true, GAME_WHITE_WINS);
        assert_eq!(original.wins, 5, "original must not be mutated");
    }

    // ── ChallengeRecord ──────────────────────────────────────────────────────

    #[test]
    fn challenge_status_predicates() {
        let open  = ChallengeRecord { id: 0, challenger_prefix: 1, wager_amount: 100, status: CHALLENGE_OPEN };
        let accpt = ChallengeRecord { status: CHALLENGE_ACCEPTED, ..open.clone() };
        let cancl = ChallengeRecord { status: CHALLENGE_CANCELLED, ..open.clone() };
        assert!( open.is_open());
        assert!(!open.is_accepted());
        assert!(!open.is_cancelled());
        assert!( accpt.is_accepted());
        assert!( cancl.is_cancelled());
    }

    // ── GameRecord ───────────────────────────────────────────────────────────

    #[test]
    fn winner_prefix_returns_correct_player() {
        let base = GameRecord { id: 0, white_prefix: 10, black_prefix: 20, status: GAME_ACTIVE, game_contract_prefix: 99 };
        assert_eq!(base.winner_prefix(),  None);
        assert_eq!(GameRecord { status: GAME_WHITE_WINS, ..base.clone() }.winner_prefix(), Some(10));
        assert_eq!(GameRecord { status: GAME_BLACK_WINS, ..base.clone() }.winner_prefix(), Some(20));
        assert_eq!(GameRecord { status: GAME_DRAW,       ..base }.winner_prefix(),         None);
    }

    // ── Leaderboard sorting ──────────────────────────────────────────────────

    #[test]
    fn sort_leaderboard_orders_by_wins_descending() {
        let mut agents = vec![
            make_agent(1, 2, 1, 0),
            make_agent(2, 5, 0, 0),
            make_agent(3, 1, 2, 0),
        ];
        sort_leaderboard(&mut agents);
        assert_eq!(agents[0].prefix, 2); // 5 wins
        assert_eq!(agents[1].prefix, 1); // 2 wins
        assert_eq!(agents[2].prefix, 3); // 1 win
    }

    #[test]
    fn sort_leaderboard_breaks_win_ties_by_fewer_losses() {
        let mut agents = vec![
            make_agent(1, 3, 2, 0), // 3W 2L
            make_agent(2, 3, 0, 0), // 3W 0L  ← should rank higher
            make_agent(3, 3, 1, 0), // 3W 1L
        ];
        sort_leaderboard(&mut agents);
        assert_eq!(agents[0].prefix, 2);
        assert_eq!(agents[1].prefix, 3);
        assert_eq!(agents[2].prefix, 1);
    }

    #[test]
    fn sort_leaderboard_breaks_further_ties_by_more_draws() {
        let mut agents = vec![
            make_agent(1, 3, 0, 1), // 3W 0L 1D
            make_agent(2, 3, 0, 3), // 3W 0L 3D  ← should rank higher
        ];
        sort_leaderboard(&mut agents);
        assert_eq!(agents[0].prefix, 2);
        assert_eq!(agents[1].prefix, 1);
    }

    #[test]
    fn leaderboard_rank_returns_1_based_position() {
        let agents = vec![
            make_agent(10, 5, 0, 0),
            make_agent(20, 3, 1, 0),
            make_agent(30, 1, 2, 0),
        ];
        assert_eq!(leaderboard_rank(&agents, 10), Some(1));
        assert_eq!(leaderboard_rank(&agents, 20), Some(2));
        assert_eq!(leaderboard_rank(&agents, 30), Some(3));
    }

    #[test]
    fn leaderboard_rank_returns_none_for_unknown_prefix() {
        let agents = vec![make_agent(10, 5, 0, 0)];
        assert_eq!(leaderboard_rank(&agents, 99), None);
    }

    #[test]
    fn sort_leaderboard_handles_empty_input() {
        let mut agents: Vec<AgentStats> = vec![];
        sort_leaderboard(&mut agents); // must not panic
        assert!(agents.is_empty());
    }

    #[test]
    fn sort_leaderboard_handles_single_agent() {
        let mut agents = vec![make_agent(1, 0, 0, 0)];
        sort_leaderboard(&mut agents);
        assert_eq!(agents.len(), 1);
    }

    // ── apply_game_result ────────────────────────────────────────────────────

    #[test]
    fn apply_game_result_updates_both_players() {
        let mut stats = vec![make_agent(1, 0, 0, 0), make_agent(2, 0, 0, 0)];
        let updated = apply_game_result(&mut stats, 1, 2, GAME_WHITE_WINS);
        assert_eq!(updated, 2);
        let w = stats.iter().find(|a| a.prefix == 1).unwrap();
        let b = stats.iter().find(|a| a.prefix == 2).unwrap();
        assert_eq!(w.wins,   1);
        assert_eq!(b.losses, 1);
    }

    #[test]
    fn apply_game_result_draw_increments_both_draws() {
        let mut stats = vec![make_agent(1, 0, 0, 0), make_agent(2, 0, 0, 0)];
        apply_game_result(&mut stats, 1, 2, GAME_DRAW);
        for a in &stats { assert_eq!(a.draws, 1); }
    }

    #[test]
    fn apply_game_result_returns_zero_for_unknown_agents() {
        let mut stats = vec![make_agent(1, 0, 0, 0)];
        let updated = apply_game_result(&mut stats, 99, 100, GAME_WHITE_WINS);
        assert_eq!(updated, 0);
        assert_eq!(stats[0].wins, 0); // untouched
    }

    #[test]
    fn apply_game_result_updates_only_one_if_one_unregistered() {
        let mut stats = vec![make_agent(1, 0, 0, 0)];
        let updated = apply_game_result(&mut stats, 1, 99, GAME_WHITE_WINS);
        assert_eq!(updated, 1);
        assert_eq!(stats[0].wins, 1);
    }
}
