use std::process::ExitCode;

use trellis_observatory_engine::{available_bug_capsules, run_all_bug_capsules, run_bug_capsule};

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => print_json(&available_bug_capsules(), true),
        [arg] if arg == "--list" => print_json(&available_bug_capsules(), true),
        [arg] if arg == "--all" => {
            let reports = run_all_bug_capsules();
            let ok = reports.iter().all(|report| report.status == "pass");
            print_json(&reports, ok)
        }
        [flag, name] if flag == "--capsule" => match run_bug_capsule(name) {
            Some(report) => {
                let ok = report.status == "pass";
                print_json(&report, ok)
            }
            None => {
                eprintln!("unknown capsule: {name}");
                print_usage();
                ExitCode::FAILURE
            }
        },
        _ => {
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn print_json<T: serde::Serialize>(value: &T, ok: bool) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(json) => {
            println!("{json}");
            if ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("failed to serialize eval capsule output: {error}");
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  cargo run -p trellis-observatory-engine --example eval_capsules -- --list");
    eprintln!("  cargo run -p trellis-observatory-engine --example eval_capsules -- --all");
    eprintln!(
        "  cargo run -p trellis-observatory-engine --example eval_capsules -- --capsule <name>"
    );
}
