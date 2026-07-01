use std::hint::black_box;
use std::time::Instant;

use trellis_bench::smoke::BENCHES;

fn main() {
    for bench in BENCHES {
        measure(bench.name, bench.iterations, bench.run);
    }
}

fn measure(name: &str, iterations: usize, bench: fn() -> usize) {
    let start = Instant::now();
    let mut checksum = 0usize;
    for _ in 0..iterations {
        checksum ^= black_box(bench());
    }
    println!(
        "{name}: {:?} over {iterations} iterations ({checksum})",
        start.elapsed()
    );
}
