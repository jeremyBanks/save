#![doc = include_str!("../README.md")]
#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(unsafe_code)]
#![warn(
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    missing_copy_implementations,
    missing_debug_implementations,
    non_ascii_idents,
    noop_method_call,
    single_use_lifetimes,
    unreachable_pub,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_ref_ptr,
    clippy::cloned_instead_of_copied,
    clippy::doc_markdown,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::from_iter_instead_of_collect,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inefficient_to_string,
    clippy::large_digit_groups,
    clippy::manual_filter_map,
    clippy::match_same_arms,
    clippy::missing_const_for_fn,
    clippy::missing_enforced_import_renames,
    clippy::module_name_repetitions,
    clippy::multiple_crate_versions,
    clippy::multiple_inherent_impl,
    clippy::must_use_candidate,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::panicking_unwrap,
    clippy::redundant_closure_for_method_calls,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::todo,
    clippy::unicode_not_nfc,
    clippy::unimplemented,
    clippy::unnecessary_unwrap,
    clippy::unreadable_literal,
    clippy::unseparated_literal_suffix,
    clippy::unused_self,
    clippy::use_self,
    clippy::useless_transmute
)]

pub mod cli;
pub mod git_ext;
pub mod hex;
pub mod testing;
pub mod zigzag;
