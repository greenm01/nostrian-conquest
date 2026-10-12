pub mod admin;
pub mod commands;
pub mod config;
pub mod dispatch;
pub mod game;
pub mod invite;
pub mod lobby;
pub mod map_archive;
pub mod status;
pub mod supervisor;
pub mod support;

pub fn run_cli(args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    dispatch::run_args(args)
}
