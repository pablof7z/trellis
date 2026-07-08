mod support;

use support::CapsuleFns;
use trellis_examples::pipeline_lab::{
    available_bug_capsules, pipeline_lifecycle_showcase_trace, run_all_bug_capsules,
    run_bug_capsule,
};

fn main() {
    support::run_with_capsules(
        "pipeline-lifecycle",
        pipeline_lifecycle_showcase_trace,
        CapsuleFns {
            list: available_bug_capsules,
            all: run_all_bug_capsules,
            one: run_bug_capsule,
        },
    );
}
