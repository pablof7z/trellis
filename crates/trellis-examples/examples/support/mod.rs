use std::{env, process};

use trellis_examples::showcase_trace::{ShowcaseTrace, to_pretty_json};

pub(crate) fn run(expected_script: &str, build: fn() -> ShowcaseTrace) {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args != ["--script", expected_script] {
        eprintln!("usage: --script {expected_script}");
        process::exit(2);
    }

    match to_pretty_json(&build()) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("failed to serialize showcase trace: {error}");
            process::exit(1);
        }
    }
}
