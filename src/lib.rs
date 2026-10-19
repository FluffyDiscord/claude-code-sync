#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbosityLevel {
    Quiet,
    Normal,
    Verbose,
}

pub mod artifacts;

pub mod config;

pub mod conflict;

pub mod interactive_conflict;

pub mod filter;

pub mod git;

pub mod history;

pub mod logger;

pub mod merge;

pub mod merge_tool;

pub mod later_timestamps;

pub mod onboarding;

pub mod parser;

pub mod project_map;

pub mod purge;

pub mod progress;

pub mod report;

pub mod self_update;

pub mod sync;

pub mod undo;

pub mod handlers;
