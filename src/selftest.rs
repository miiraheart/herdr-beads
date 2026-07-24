//! Headless board dump. `herdr-beads --selftest` prints the grouped board as
//! plain text - no TUI - so CI (and you, before launching herdr) can confirm
//! the bd bridge parses and groups correctly.

use crate::bd;
use crate::model::{status_glyph, status_label, status_rank, Scope};
use anyhow::Result;

pub fn run(scope: Scope) -> Result<()> {
    let beads = bd::load(scope, true)?;
    println!(
        "herdr-beads selftest - {} beads · scope {}\n",
        beads.len(),
        scope.label()
    );

    let mut statuses: Vec<String> = beads.iter().map(|b| b.status.clone()).collect();
    statuses.sort();
    statuses.dedup();
    statuses.sort_by_key(|s| (status_rank(s), s.clone()));

    for status in statuses {
        let mut group: Vec<&_> = beads.iter().filter(|b| b.status == status).collect();
        group.sort_by_key(|a| a.priority);
        println!(
            "{} {} ({})",
            status_glyph(&status),
            status_label(&status),
            group.len()
        );
        for b in group {
            println!(
                "  P{} {:<14} {}",
                b.priority,
                b.id,
                b.title.chars().take(70).collect::<String>()
            );
        }
        println!();
    }
    Ok(())
}
