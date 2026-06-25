//! The `suggest` command: the inverse of evaluation.
//!
//! Evaluation takes a metric plus an expected value and produces a pass or fail. Suggestion
//! goes the other way: it reads a file's current properties and proposes assertions, ready to
//! write to an assertions file the `run` command can consume. The proposals are heuristic
//! guesses from the file's current state, not verified expectations, hence "suggest". None of
//! the executor or dispatch code is involved; this layer reuses the same property functions the
//! executors call.
//!
//! The pieces: a [`Suggestion`] is one structured output line; a [`SuggestionProvider`] emits a
//! family's default set for a file (one provider per resource family, see [`providers`]); and
//! [`suggest`] orchestrates the providers and renders the assertions-file body.

mod orchestrator;
mod provider;
pub mod providers;
pub mod suggestion;

pub use orchestrator::{SuggestResult, suggest};
pub use provider::{SuggestionProvider, providers};
pub use suggestion::{Suggestion, band};
