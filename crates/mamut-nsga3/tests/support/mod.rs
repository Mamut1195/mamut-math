//! Shared test support for the MATH-NSGA-004 conformance suite.
//!
//! This module is compiled into each conformance integration test crate. Items
//! not used by a particular test are still compiled, so the whole module is
//! `allow(dead_code)`.
#![allow(dead_code)]

pub mod dtlz;
pub mod harness;
pub mod json;
pub mod rng;
