mod support;

use support::CapsuleFns;
use trellis_examples::market_desk::{
    available_bug_capsules, market_lifecycle_showcase_trace, run_all_bug_capsules, run_bug_capsule,
};

fn main() {
    support::run_with_capsules(
        "market-lifecycle",
        market_lifecycle_showcase_trace,
        CapsuleFns {
            list: available_bug_capsules,
            all: run_all_bug_capsules,
            one: run_bug_capsule,
        },
    );
}
