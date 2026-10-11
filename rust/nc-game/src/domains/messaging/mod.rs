pub mod action;
mod controller;
mod inbox;
pub mod screens;
pub mod state;
pub mod update;

pub use action::MessagingAction;
pub(crate) use inbox::{delete_item_prompt, delete_prompt_choice};
pub use state::MessagingState;
pub mod views;
