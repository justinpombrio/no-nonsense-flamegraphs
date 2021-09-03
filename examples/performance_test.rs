//! Remember to run with --release.
//!
//! A big/complex flame graph had the following performance:
//! - 1ms to construct a very large flame graph. (800 lines, avg. of 15 calls, 60kb.) As far as I
//!   can tell, this time is spent formatting strings.
//! - 70ns/call to span!, assuming that 100% of time was spent in tracing. This is for 5.4*10^6
//!   calls to span!.
//!
//! A reasonably sized flame graph (60 lines) had the following performance:
//! - 120μs to construct a reasonably sized flame graph (60 lines, average line has 15 calls).
//! - 70ns/call to span!, assuming that 100% of time was spent in tracing. This is for 5.4*10^6
//!   calls to span!.

use no_nonsense_flamegraphs::span;
use std::time::Instant;

fn main() {
    // Expository purposes only. Don't ever `span!` recursive functions!
    fn fib(n: usize) -> usize {
        span!("fib");
        if is_small(n) {
            2
        } else {
            fib(n - 1) + fib(n - 2) + 2
        }
    }

    fn is_small(n: usize) -> bool {
        span!("is_small");
        n <= 2
    }

    fn fan(n: usize) -> usize {
        let name: &'static str = Box::leak(Box::new(format!("f{}", n)));
        span!(name);
        fib(n) + 1
    }

    fn fanout(n: usize) -> usize {
        span!("fanout");
        let mut sum = 1;
        for i in 0..n {
            sum += fan(i);
        }
        sum
    }

    let now = Instant::now();
    let num_calls = fanout(30);
    println!("Total num calls: {}", num_calls);
    let elapsed_ms = now.elapsed().as_millis();
    println!(
        "Time taken to construct and save flame graph: {}ms",
        elapsed_ms
    );
    println!(
        "Time per call (upper bound on overhead): {}ns",
        1000000 * elapsed_ms / num_calls as u128
    );
}
