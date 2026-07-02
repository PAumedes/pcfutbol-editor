//! Sixth-pass investigator (PKF_FORMAT.md §5 Q2): tries to pin down the
//! ~270-335 byte region between `Team.president` (ends at blob offset 147)
//! and the coach marker (offset 482) against the override format's
//! `budget`+`affiliate1`+`affiliate2`+`league_history[10]`+`stats`+`jornada`
//! (92 bytes)+`palmares` (34 bytes)+`Tactics` (formation_blob + 7-byte tail)
//! field order (`dbc.rs::read_team`/`read_tactics`).
//!
//! Investigation-only; does not feed `container.rs`.
//!
//! Usage: cargo run -p pcf-codec --example investigate_tactics_block -- [path]

use std::fs;

struct Cur<'a> {
    data: &'a [u8],
    pos: usize,
}
impl<'a> Cur<'a> {
    fn u8(&mut self) -> u8 {
        let v = self.data[self.pos];
        self.pos += 1;
        v
    }
    fn take(&mut self, n: usize) -> &'a [u8] {
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        s
    }
    fn u16_le(&mut self) -> u16 {
        let b = self.take(2);
        u16::from_le_bytes([b[0], b[1]])
    }
    fn u24_le(&mut self) -> u32 {
        let b = self.take(3);
        u32::from_le_bytes([b[0], b[1], b[2], 0])
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "fixtures/golden/real_river_9001_container_blob.raw".to_string());
    let bytes = fs::read(&path).expect("read blob");
    println!("file: {path} ({} bytes)\n", bytes.len());

    // Cursor position 147 == right after `president`, per PKF_FORMAT.md §6.2.
    let start = 147usize;
    let coach_marker = 482usize; // confirmed §6.5
    println!(
        "region under study: [{start:#x}..{coach_marker:#x}) = {} bytes\n",
        coach_marker - start
    );

    println!("--- raw hex of the whole region, 16 bytes/line, offsets absolute ---");
    let region = &bytes[start..coach_marker];
    for (i, chunk) in region.chunks(16).enumerate() {
        let off = start + i * 16;
        let hex: String = chunk.iter().map(|b| format!("{b:02x} ")).collect();
        println!("{off:#06x}: {hex}");
    }

    println!("\n--- attempt: override field order (budget/affiliate/league_history/stats) ---");
    let mut c = Cur {
        data: &bytes,
        pos: start,
    };
    let budget = c.u24_le();
    let affiliate1 = c.u16_le();
    let affiliate2 = c.u16_le();
    println!(
        "budget(u24)={budget} affiliate1(u16)={affiliate1:#06x} affiliate2(u16)={affiliate2:#06x}  cursor={:#x}",
        c.pos
    );

    println!("league_history[10] (position,division) pairs:");
    let mut league = Vec::new();
    for _ in 0..10 {
        let position = c.u8();
        let division = c.u8();
        league.push((position, division));
    }
    println!("  {league:?}  cursor={:#x}", c.pos);

    let stats_start = c.pos;
    let stats_bytes = c.take(14).to_vec();
    println!("stats[14] raw = {stats_bytes:02x?}  cursor={:#x}", c.pos);
    // TeamStatsRaw: 6x u16 LE (played,won,drawn,gf,ga,points) + champion(u8) + runner_up(u8)
    let played = u16::from_le_bytes([stats_bytes[0], stats_bytes[1]]);
    let won = u16::from_le_bytes([stats_bytes[2], stats_bytes[3]]);
    let drawn = u16::from_le_bytes([stats_bytes[4], stats_bytes[5]]);
    let gf = u16::from_le_bytes([stats_bytes[6], stats_bytes[7]]);
    let ga = u16::from_le_bytes([stats_bytes[8], stats_bytes[9]]);
    let points = u16::from_le_bytes([stats_bytes[10], stats_bytes[11]]);
    println!(
        "  decoded: played={played} won={won} drawn={drawn} gf={gf} ga={ga} points={points} champion={} runner_up={}",
        stats_bytes[12], stats_bytes[13]
    );
    println!("stats_start={stats_start:#x}");

    let after_stats = c.pos;
    let remaining_to_coach = coach_marker - after_stats;
    println!(
        "\nbytes remaining between end-of-stats ({after_stats:#x}) and coach marker ({coach_marker:#x}): {remaining_to_coach}"
    );
    println!(
        "override needs jornada(92)+palmares(34)=126 for Team, then Tactics = 2(len)+formation_blob+7(tail)"
    );
    if remaining_to_coach >= 126 {
        println!(
            "  => {} bytes would remain for Tactics after a full 126-byte jornada+palmares",
            remaining_to_coach - 126
        );
    } else {
        println!(
            "  => NOT ENOUGH ROOM for a full 126-byte jornada+palmares before the coach marker"
        );
    }

    // Try candidate literal jornada(92)+palmares(34) read and dump what's left.
    if remaining_to_coach >= 126 {
        let jornada_start = after_stats;
        let jornada = &bytes[jornada_start..jornada_start + 92];
        let palmares_start = jornada_start + 92;
        let palmares = &bytes[palmares_start..palmares_start + 34];
        let tactics_start = palmares_start + 34;
        println!("\njornada[92]  @ {jornada_start:#x}: {jornada:02x?}");
        println!("\npalmares[34] @ {palmares_start:#x}: {palmares:02x?}");
        println!(
            "\n--- candidate Tactics region [{tactics_start:#x}..{coach_marker:#x}), {} bytes ---",
            coach_marker - tactics_start
        );
        let tactics_region = &bytes[tactics_start..coach_marker];
        for (i, chunk) in tactics_region.chunks(16).enumerate() {
            let off = tactics_start + i * 16;
            let hex: String = chunk.iter().map(|b| format!("{b:02x} ")).collect();
            println!("{off:#06x}: {hex}");
        }
        // If Tactics is [u16 len][formation_blob][7-byte tail], the len
        // prefix should be (coach_marker - tactics_start - 2 - 7).
        let expected_len = (coach_marker - tactics_start).saturating_sub(2 + 7);
        let actual_len_here = u16::from_le_bytes([tactics_region[0], tactics_region[1]]) as usize;
        println!(
            "\nif formation_blob len-prefixed here: expected_len(for exact fit)={expected_len}, actual u16 read at start={actual_len_here}"
        );
        println!(
            "last 7 bytes before coach marker (candidate TacticsFixedRaw tail): {:02x?}",
            &bytes[coach_marker - 7..coach_marker]
        );
    }

    // Independent of the jornada/palmares hypothesis: scan the WHOLE region
    // [start..coach_marker) for any 2-byte LE value V such that some run of
    // V bytes starting right after it stays inside the region, AND ALSO
    // scan for V matching the count of bytes remaining up to some other
    // fixed points (194 possible sub-lengths): report any u16 LE value in
    // 1..=300 found anywhere in the region together with what byte offset
    // "start+len+2" would land on, to eyeball candidates by hand.
    println!("\n--- scan: every u16 LE value 1..=300 found anywhere in the region (candidate length prefixes) ---");
    for i in start..coach_marker.saturating_sub(1) {
        let v = u16::from_le_bytes([bytes[i], bytes[i + 1]]) as usize;
        if (1..=300).contains(&v) {
            let end = i + 2 + v;
            let lands_on_coach = end == coach_marker;
            let marker = if lands_on_coach {
                "  <<== LANDS EXACTLY ON COACH MARKER"
            } else {
                ""
            };
            println!(
                "  offset={i:#06x} u16={v:<4} -> if length-prefix, string end at {end:#06x}{marker}"
            );
        }
    }
}
