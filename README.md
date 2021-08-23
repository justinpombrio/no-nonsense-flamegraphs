# No Nonsense Flamegraphs

** This is a modification/simplification of [Commure/Stencil](https://github.com/commure/stencil).
If you need more features, look there. **

---

1. Mark which functions you want to trace in your Rust code.
2. Run your code. Flamegraph saved in `flamegraph.svg`. Bosh. Done.

This saves flame graphs, not flame charts. "But," you ask...

## "What is the difference betweeen a flame graph and a flame chart?"

A flame chart shows the precise timing of each call. If function A calls function B 100 times,
it will show up in the picture as 100 separate calls. This is both a blessing (it shows the
order of events, and has a very clear interpretation), and a curse (it scales poorly for
measuring frequent but brief calls).

In a flame graph, on the other hand, if A calls B 100 times, all the calls to B will be lumped
together into one span showing their total time.

There is a good overview of flame graphs in this readme for a different project:
https://github.com/flamegraph-rs/flamegraph#systems-performance-work-guided-by-flamegraphs

If you _do_ want a flame graph, your next question should be...

## "How do I use this crate?"

It's really really simple. Mark each of the functions you want to trace with `outln!("LABEL")`:

```
use no_nonsense_flamegraphs::outln;

fn myExistingFunctionRelatedToKittens() {
  outln!("kittens"); // measures the span from now until it's dropped
  // kitten related functionality
}
```

Then run your program, and a flame graph marked with your LABELs will be saved at `flamegraph.svg`
in your repo root.

It's safe to trace pretty hot loops. The overhead of a call to `outln!` in `--release` mode is only
about 70ns on my laptop.

However, _do not trace deeply recursive functions_, as they are liable to make large and unweildy
flame graphs, and possibly even cause panics while this library tries to convert the trace to an
SVG.
