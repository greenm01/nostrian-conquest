//! The classic ECMAINT encounter loop (`docs/dev/ec-encounter-spec.md`).

#[path = "encounter/support.rs"]
mod support;

#[path = "encounter/classic.rs"]
mod classic;

#[path = "encounter/decisions.rs"]
mod decisions;

#[path = "encounter/odds.rs"]
mod odds;

#[path = "encounter/aborts.rs"]
mod aborts;

#[path = "encounter/memory.rs"]
mod memory;
