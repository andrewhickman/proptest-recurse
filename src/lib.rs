#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

//! This crate provides a helper struct for defining mutually recursive strategies with
//! [`proptest`](https://crates.io/crates/proptest). The `prop_recursive` combinator is useful for
//! defining simple recursive strategies but for two or more mutually recursive strategies it
//! becomes cumbersome to use. `StrategySet` aims to solve this problem.
//!
//! # Examples
//!
//! Suppose we have the following mutually recursive types `First` and `Second`
//!
//! ```no_run
//! #[derive(Clone, Debug)]
//! enum First {
//!     Zero,
//!     Second(Vec<Second>),
//! }
//!
//! #[derive(Clone, Debug)]
//! enum Second {
//!     Zero,
//!     First(First),
//! }
//! ```
//! We can define strategies for each using a `StrategySet`
//! ```no_run
//! # use proptest::collection::vec;
//! # use proptest::prelude::*;
//! # use proptest::strategy::{SBoxedStrategy, Just};
//! #
//! # #[derive(Clone, Debug)]
//! # enum First {
//! #     Zero,
//! #     Second(Vec<Second>),
//! # }
//! #
//! # #[derive(Clone, Debug)]
//! # enum Second {
//! #     Zero,
//! #     First(First),
//! # }
//! #
//! use proptest_recurse::{StrategySet, StrategyExt};
//!
//! fn arb_first(set: &mut StrategySet) -> SBoxedStrategy<First> {
//!     Just(First::Zero).prop_mutually_recursive(5, 32, 8, set, |set| {
//!         vec(set.get::<Second, _>(arb_second), 0..8)
//!             .prop_map(First::Second)
//!             .sboxed()
//!     })
//! }
//!
//! fn arb_second(set: &mut StrategySet) -> SBoxedStrategy<Second> {
//!     Just(Second::Zero)
//!         .prop_mutually_recursive(3, 32, 1, set, |set| {
//!             set.get::<First, _>(arb_first)
//!                 .prop_map(Second::First)
//!                 .sboxed()
//!         }).sboxed()
//! }
//! #
//! # fn main() {}
//! ```
//! To use these strategies, simply pass in an empty `StrategySet`
//! ```no_run
//! # use proptest::collection::vec;
//! # use proptest::{prelude::*, proptest};
//! # use proptest::strategy::{SBoxedStrategy, Just};
//! #  
//! # use proptest_recurse::{StrategySet, StrategyExt};
//! #
//! # #[derive(Clone, Debug)]
//! # enum First {
//! #     Zero,
//! #     Second(Vec<Second>),
//! # }
//! #
//! # #[derive(Clone, Debug)]
//! # enum Second {
//! #     Zero,
//! #     First(First),
//! # }
//! #
//! #
//! # fn arb_first(set: &mut StrategySet) -> SBoxedStrategy<First> {
//! #     Just(First::Zero).prop_mutually_recursive(5, 32, 8, set, |set| {
//! #         vec(set.get::<Second, _>(arb_second), 0..8)
//! #             .prop_map(First::Second)
//! #             .sboxed()
//! #     })
//! # }
//! #
//! # fn arb_second(set: &mut StrategySet) -> SBoxedStrategy<Second> {
//! #     Just(Second::Zero)
//! #         .prop_mutually_recursive(3, 32, 1, set, |set| {
//! #             set.get::<First, _>(arb_first)
//! #                 .prop_map(Second::First)
//! #                 .sboxed()
//! #         }).sboxed()
//! # }
//! #
//! # fn main() {}
//! #
//! proptest! {
//!     #[test]
//!     fn create(_ in arb_first(&mut Default::default())) {}
//! }
//! ```

mod recursive;

use std::any::{Any, TypeId};
use std::sync::Arc;

use im::HashMap;
use proptest::strategy::{SBoxedStrategy, Strategy};

use crate::recursive::Recursive;

/// A collection of strategies that depend on each other. This type is cheap to clone.
#[derive(Clone, Default, Debug)]
pub struct StrategySet {
    inner: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl StrategySet {
    /// Returns a strategy for `T`. If a strategy does not exist, it is created and inserted using
    /// `f`.
    pub fn get<T, F>(&mut self, f: F) -> SBoxedStrategy<T>
    where
        T: Any,
        F: FnOnce(&mut Self) -> SBoxedStrategy<T>,
    {
        let mut this = self.clone();
        self.inner
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Arc::new(f(&mut this)))
            .downcast_ref::<SBoxedStrategy<T>>()
            .unwrap()
            .clone()
    }
}

/// Extension methods for strategies.
pub trait StrategyExt: Strategy {
    /// A variant of `prop_recursive` for mutually recursive strategies. Instead of taking a single
    /// strategy, the branch function takes a set of strategies of various nesting depths. Note that
    /// the parameters `depth`, `desired_size`, and `expected_branch_size` apply only to values from
    /// this strategy.
    fn prop_mutually_recursive<F>(
        self,
        depth: u32,
        desired_size: u32,
        expected_branch_size: u32,
        set: &StrategySet,
        recurse: F,
    ) -> SBoxedStrategy<Self::Value>
    where
        Self::Value: Any,
        F: Fn(&mut StrategySet) -> SBoxedStrategy<Self::Value> + Send + Sync + 'static;
}

impl<T: Strategy + Send + Sync + 'static> StrategyExt for T {
    fn prop_mutually_recursive<F>(
        self,
        depth: u32,
        desired_size: u32,
        expected_branch_size: u32,
        set: &StrategySet,
        branch: F,
    ) -> SBoxedStrategy<Self::Value>
    where
        Self::Value: Any,
        F: Fn(&mut StrategySet) -> SBoxedStrategy<Self::Value> + Send + Sync + 'static,
    {
        let set = set.inner.clone();
        Recursive::new(
            self.sboxed(),
            depth,
            desired_size,
            expected_branch_size,
            move |nested| {
                branch(&mut StrategySet {
                    inner: set.update(TypeId::of::<Self::Value>(), Arc::new(nested)),
                })
            },
        )
        .sboxed()
    }
}

#[test]
fn strategy_set_send_sync() {
    fn send<T: Send>() {}
    fn sync<T: Sync>() {}

    send::<StrategySet>();
    sync::<StrategySet>();
}
