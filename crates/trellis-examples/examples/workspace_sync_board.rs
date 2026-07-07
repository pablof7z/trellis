mod support;

use support::CapsuleFns;
use trellis_examples::workspace_sync_board::{
    available_bug_capsules, run_all_bug_capsules, run_bug_capsule, switch_workspace_showcase_trace,
};

fn main() {
    support::run_with_capsules(
        "switch-workspace",
        switch_workspace_showcase_trace,
        CapsuleFns {
            list: available_bug_capsules,
            all: run_all_bug_capsules,
            one: run_bug_capsule,
        },
    );
}
