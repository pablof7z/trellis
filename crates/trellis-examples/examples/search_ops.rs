mod support;

use support::CapsuleFns;
use trellis_examples::search_ops::{
    available_bug_capsules, run_all_bug_capsules, run_bug_capsule, search_lifecycle_showcase_trace,
};

fn main() {
    support::run_with_capsules(
        "search-lifecycle",
        search_lifecycle_showcase_trace,
        CapsuleFns {
            list: available_bug_capsules,
            all: run_all_bug_capsules,
            one: run_bug_capsule,
        },
    );
}
