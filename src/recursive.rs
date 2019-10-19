use std::fmt;
use std::sync::Arc;

use proptest::strategy::{float_to_weight, NewTree, ValueTree};
use proptest::test_runner::*;
use proptest::{prelude::*, prop_oneof};

pub(crate) struct Recursive<T> {
    base: SBoxedStrategy<T>,
    recurse: Arc<dyn Fn(SBoxedStrategy<T>) -> SBoxedStrategy<T> + Send + Sync>,
    depth: u32,
    desired_size: u32,
    expected_branch_size: u32,
}

impl<T: fmt::Debug> fmt::Debug for Recursive<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Recursive")
            .field("base", &self.base)
            .field("recurse", &"<function>")
            .field("depth", &self.depth)
            .field("desired_size", &self.desired_size)
            .field("expected_branch_size", &self.expected_branch_size)
            .finish()
    }
}

impl<T> Clone for Recursive<T> {
    fn clone(&self) -> Self {
        Recursive {
            base: self.base.clone(),
            recurse: Arc::clone(&self.recurse),
            depth: self.depth,
            desired_size: self.desired_size,
            expected_branch_size: self.expected_branch_size,
        }
    }
}

impl<T: fmt::Debug + 'static> Recursive<T> {
    pub(crate) fn new(
        base: SBoxedStrategy<T>,
        depth: u32,
        desired_size: u32,
        expected_branch_size: u32,
        recurse: impl Fn(SBoxedStrategy<T>) -> SBoxedStrategy<T> + Send + Sync + 'static,
    ) -> Self {
        Self {
            base: base.sboxed(),
            recurse: Arc::new(recurse),
            depth,
            desired_size,
            expected_branch_size,
        }
    }
}

impl<T: fmt::Debug + 'static> Strategy for Recursive<T> {
    type Tree = Box<dyn ValueTree<Value = T>>;
    type Value = T;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        // copied from https://github.com/AltSysrq/proptest/blob/ee53956395492c8172a6d437cb0d2962f6077572/src/strategy/recursive.rs#L76

        let mut branch_probabilities = Vec::new();
        let mut k2 = u64::from(self.expected_branch_size) * 2;
        for _ in 0..self.depth {
            branch_probabilities.push(f64::from(self.desired_size) / k2 as f64);
            k2 = k2.saturating_mul(u64::from(self.expected_branch_size) * 2);
        }

        let mut strat = self.base.clone();
        while let Some(branch_probability) = branch_probabilities.pop() {
            let recursed = (self.recurse)(strat.clone());
            let recursive_choice = recursed.sboxed();
            let non_recursive_choice = strat;
            // Clamp the maximum branch probability to 0.9 to ensure we can
            // generate non-recursive cases reasonably often.
            let branch_probability = branch_probability.min(0.9);
            let (weight_branch, weight_leaf) = float_to_weight(branch_probability);
            let branch = prop_oneof![
                weight_leaf => non_recursive_choice,
                weight_branch => recursive_choice,
            ];
            strat = branch.sboxed();
        }

        strat.new_tree(runner)
    }
}
