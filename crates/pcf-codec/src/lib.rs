//! DBC parse/write, character codec, pointer assignment. Owned by Agent A.
//!
//! See PLAN.md Appendix A for the byte-level format this module implements,
//! and each submodule's docs for what's verified vs. synthetic/placeholder
//! pending real fixtures from the user (PLAN.md §9 risks 1-3).

pub mod charmap;
pub mod container;
pub mod container_bridge;
pub mod cursor;
pub mod dbc;
pub mod layout;
pub mod synthetic;

pub use charmap::CharMap;
pub use container::{
    find_domestic_team_records, parse_pkf_container, parse_pkf_container_verbose,
    parse_team_record, ContainerCoachStub, ContainerTeamRecord, TeamParseOutcome,
};
pub use container_bridge::container_team_to_dbc;
pub use dbc::DbcCodec;
