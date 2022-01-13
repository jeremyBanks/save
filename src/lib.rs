#![doc = include_str!("../README.md")]
#![warn(
    missing_docs,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::missing_safety_doc,
    clippy::must_use_candidate,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cloned_instead_of_copied,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::cloned_instead_of_copied,
    clippy::from_iter_instead_of_collect,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::unreadable_literal,
    clippy::semicolon_if_nothing_returned,
    clippy::redundant_closure_for_method_calls,
    clippy::large_digit_groups,
    clippy::inefficient_to_string,
    clippy::unused_self
)]

pub mod cli;
pub mod git;
