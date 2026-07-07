mod support;

use support::CapsuleFns;
use trellis_examples::fleetpulse::{
    available_bug_capsules, revoke_permission_showcase_trace, run_all_bug_capsules, run_bug_capsule,
};

fn main() {
    support::run_with_capsules(
        "revoke-permission",
        revoke_permission_showcase_trace,
        CapsuleFns {
            list: available_bug_capsules,
            all: run_all_bug_capsules,
            one: run_bug_capsule,
        },
    );
}
