//! grimoire: a catalogue of the game library on the NAS.

mod application;
mod bootstrap;
mod domain;
mod infrastructure;
mod kernel;
mod presentation;

pub use bootstrap::run;
