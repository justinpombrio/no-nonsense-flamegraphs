The interface to this crate is purposefully very simple: annotate some functions with `span!`, and
get `flamegraph.svg`. There is currently no configuration, but it would be sensible to add some for:

- A directory to save `flamegraph.svg` in, besides the current directory.
- Various `inferno` configuration options. (Though not all of them! Many do not make sense for this
  crate.)

## To Test

Run the three examples in `examples/` with `cargo run --example NAME`, and check by hand that the
flamegraphs they produce look good.
