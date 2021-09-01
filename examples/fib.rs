//! This ought to produce a flamegraph that looks like this:
//!
//!                                     [=== is_small (2 calls) ===]
//!    [=== is_small (1 calls) ===]  [=== fib (2 calls) ===========]
//! [=== fib (1 calls) ============================================]

use no_nonsense_flamegraphs::outln;

fn main() {
    // Expository purposes only. Don't ever `outln!` recursive functions!
    fn fib(n: usize) -> usize {
        outln!("fib");

        if is_small(n) {
            n
        } else {
            fib(n - 1) + fib(n - 2)
        }
    }

    fn is_small(n: usize) -> bool {
        outln!("is_small");

        n <= 2
    }

    fib(3);
}
