#![allow(clippy::unreadable_literal, clippy::inconsistent_digit_grouping, clippy::precedence)]

#[macro_use] pub mod html;
pub mod game;
pub mod moves;
pub mod infoset;
pub mod ai_interface;
pub mod distr;
pub mod eval;
pub mod greedy;
#[cfg(feature = "heavy")] pub mod api;
#[cfg(feature = "heavy")] pub mod history;
#[cfg(feature = "heavy")] pub mod history_db;
pub mod logger;
pub mod stats;
pub mod fast;