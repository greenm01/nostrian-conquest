//! t039 phase 5: transitional accessor methods on the model types, kept
//! only while call sites move to the model's fields and the free functions
//! that replace the accessors with logic. Each accessor goes when its last
//! caller does; this module goes with the last of them.

pub mod base;
pub mod conquest;
pub mod fleet;
pub mod ipbm;
pub mod planet;
pub mod player;
pub mod setup;
