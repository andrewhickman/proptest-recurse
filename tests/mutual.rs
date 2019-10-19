use proptest::collection::vec;
use proptest::strategy::{Just, SBoxedStrategy};
use proptest::{prelude::*, proptest};

use proptest_recurse::{StrategyExt, StrategySet};

#[derive(Clone, Debug)]
enum First {
    Zero,
    Second(Vec<Second>),
}

#[derive(Clone, Debug)]
enum Second {
    Zero,
    First(First),
}

impl First {
    fn depth(&self) -> u32 {
        match self {
            First::Zero => 0,
            First::Second(s) => match s.iter().map(Second::depth).max() {
                Some(depth) => depth + 1,
                None => 0,
            },
        }
    }
}

impl Second {
    fn depth(&self) -> u32 {
        match self {
            Second::Zero => 0,
            Second::First(f) => f.depth() + 1,
        }
    }
}

fn arb_first(set: &mut StrategySet) -> SBoxedStrategy<First> {
    Just(First::Zero).prop_mutually_recursive(5, 32, 8, set, |set| {
        vec(set.get::<Second, _>(arb_second), 0..8)
            .prop_map(First::Second)
            .sboxed()
    })
}

fn arb_second(set: &mut StrategySet) -> SBoxedStrategy<Second> {
    Just(Second::Zero)
        .prop_mutually_recursive(3, 32, 1, set, |set| {
            set.get::<First, _>(arb_first)
                .prop_map(Second::First)
                .sboxed()
        })
        .sboxed()
}

proptest! {
    #[test]
    fn create_first(x in arb_first(&mut Default::default())) {
        assert!(x.depth() <= 8);
    }

    #[test]
    fn create_second(x in arb_second(&mut Default::default())) {
        assert!(x.depth() <= 8);
    }
}
