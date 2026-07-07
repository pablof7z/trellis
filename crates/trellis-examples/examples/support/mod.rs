use std::{env, process};

use trellis_examples::{
    seeded_bugs::{SeededBugCapsule, SeededBugReport},
    showcase_trace::ShowcaseTrace,
};

#[allow(dead_code)]
pub(crate) fn run(expected_script: &str, build: fn() -> ShowcaseTrace) {
    run_args(expected_script, build, None);
}

#[allow(dead_code)]
pub(crate) fn run_with_capsules(
    expected_script: &str,
    build: fn() -> ShowcaseTrace,
    capsules: CapsuleFns,
) {
    run_args(expected_script, build, Some(capsules));
}

pub(crate) struct CapsuleFns {
    pub(crate) list: fn() -> Vec<SeededBugCapsule>,
    pub(crate) all: fn() -> Vec<SeededBugReport>,
    pub(crate) one: fn(&str) -> Option<SeededBugReport>,
}

fn run_args(expected_script: &str, build: fn() -> ShowcaseTrace, capsules: Option<CapsuleFns>) {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [flag, script] if flag == "--script" && script == expected_script => {
            print_json(&build(), true)
        }
        [flag] if flag == "--list-capsules" => {
            let Some(capsules) = capsules else {
                usage(expected_script, false);
            };
            print_json(&(capsules.list)(), true);
        }
        [flag] if flag == "--capsules" => {
            let Some(capsules) = capsules else {
                usage(expected_script, false);
            };
            let reports = (capsules.all)();
            let ok = reports.iter().all(|report| report.status == "pass");
            print_json(&reports, ok);
        }
        [flag, name] if flag == "--capsule" => {
            let Some(capsules) = capsules else {
                usage(expected_script, false);
            };
            let Some(report) = (capsules.one)(name) else {
                eprintln!("unknown capsule: {name}");
                usage(expected_script, true);
            };
            let ok = report.status == "pass";
            print_json(&report, ok);
        }
        _ => usage(expected_script, capsules.is_some()),
    }
}

fn print_json(value: &impl serde::Serialize, ok: bool) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("failed to serialize showcase output: {error}");
            process::exit(1);
        }
    }
    if !ok {
        process::exit(1);
    }
}

fn usage(expected_script: &str, has_capsules: bool) -> ! {
    eprintln!("usage: --script {expected_script}");
    if has_capsules {
        eprintln!("       --list-capsules");
        eprintln!("       --capsules");
        eprintln!("       --capsule <name>");
    }
    process::exit(2);
}
