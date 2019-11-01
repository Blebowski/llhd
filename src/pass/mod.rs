// Copyright (c) 2017-2019 Fabian Schuiki

//! Optimization and analysis passes on LLHD IR.
//!
//! This module implements various passes that analyze or mutate an LLHD
//! intermediate representation.

pub mod cf;
pub mod dce;
pub mod gcse;

pub use cf::ConstFolding;
pub use dce::DeadCodeElim;
pub use gcse::GlobalCommonSubexprElim;
