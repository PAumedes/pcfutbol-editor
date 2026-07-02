//! Pointer rules shared by every crate that names or slices DBC records
//! (PLAN.md §4.2). Keep this the only place that knows the arithmetic.

use std::ops::RangeInclusive;

use crate::TeamIndex;

const PLAYERS_PER_TEAM: u16 = 50;

/// Team override file name: `EQ97` + 4-digit decimal team pointer.
///
/// e.g. team pointer `9013` -> `EQ979013.DBC`.
pub fn team_filename(team_pointer: u16) -> String {
    format!("EQ97{team_pointer:04}.DBC")
}

/// The player pointer block owned by team `load_order` (1-indexed in load
/// order): `(k-1)*50 + 1 ..= k*50`. Barcelona (k=1) -> 1..=50, the second
/// team (k=2) -> 51..=100, etc.
///
/// Panics if `load_order` is 0; callers index teams starting at 1.
pub fn player_block_for_load_order(load_order: u32) -> RangeInclusive<u16> {
    assert!(load_order >= 1, "load_order is 1-indexed");
    let start = ((load_order - 1) * PLAYERS_PER_TEAM as u32 + 1) as u16;
    let end = (load_order * PLAYERS_PER_TEAM as u32) as u16;
    start..=end
}

/// Result of resolving a team's player block against the loaded PKF index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerBlockLookup {
    pub range: RangeInclusive<u16>,
    /// True when the team pointer wasn't found in the index and we fell
    /// back to block `1..=50` (mirrors the reference editor's behaviour;
    /// callers should surface this to the user).
    pub used_fallback: bool,
}

/// On open: look up the file's team pointer in the loaded PKF index to
/// recover its player block. Falls back to block `1..=50` if not found.
pub fn resolve_player_block(team_pointer: u16, index: &TeamIndex) -> PlayerBlockLookup {
    match index.iter().position(|entry| entry.pointer == team_pointer) {
        Some(zero_based_pos) => PlayerBlockLookup {
            range: player_block_for_load_order((zero_based_pos + 1) as u32),
            used_fallback: false,
        },
        None => PlayerBlockLookup {
            range: 1..=PLAYERS_PER_TEAM,
            used_fallback: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TeamIndexEntry;

    #[test]
    fn team_filename_formats_boca() {
        assert_eq!(team_filename(9013), "EQ979013.DBC");
    }

    #[test]
    fn team_filename_zero_pads_small_pointers() {
        assert_eq!(team_filename(1), "EQ970001.DBC");
    }

    #[test]
    fn first_team_owns_first_fifty_pointers() {
        assert_eq!(player_block_for_load_order(1), 1..=50);
    }

    #[test]
    fn second_team_owns_next_fifty_pointers() {
        assert_eq!(player_block_for_load_order(2), 51..=100);
    }

    #[test]
    #[should_panic(expected = "1-indexed")]
    fn load_order_zero_panics() {
        player_block_for_load_order(0);
    }

    #[test]
    fn resolve_finds_team_by_load_order_position() {
        let index: TeamIndex = vec![
            TeamIndexEntry {
                pointer: 9013,
                short_name: "BOCA".into(),
                country: 1,
            },
            TeamIndexEntry {
                pointer: 9014,
                short_name: "RIVER".into(),
                country: 1,
            },
        ];
        let result = resolve_player_block(9014, &index);
        assert_eq!(result.range, 51..=100);
        assert!(!result.used_fallback);
    }

    #[test]
    fn resolve_falls_back_when_pointer_missing() {
        let index: TeamIndex = vec![TeamIndexEntry {
            pointer: 9013,
            short_name: "BOCA".into(),
            country: 1,
        }];
        let result = resolve_player_block(4242, &index);
        assert_eq!(result.range, 1..=50);
        assert!(result.used_fallback);
    }
}
