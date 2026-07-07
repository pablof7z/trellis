mod support;

use support::CapsuleFns;
use trellis_examples::plugin_host::{
    available_bug_capsules, capability_lifecycle_showcase_trace, run_all_bug_capsules,
    run_bug_capsule,
};

fn main() {
    support::run_with_capsules(
        "capability-lifecycle",
        capability_lifecycle_showcase_trace,
        CapsuleFns {
            list: available_bug_capsules,
            all: run_all_bug_capsules,
            one: run_bug_capsule,
        },
    );
}
