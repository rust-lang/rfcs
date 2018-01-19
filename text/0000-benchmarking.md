- Feature Name: benchmarking
- Start Date: 2018-01-11
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This aims to stabilize basic benchmarking tools for a stable `cargo bench`

# Motivation
[motivation]: #motivation

Benchmarking is important for maintaining good libraries. They give us a clear idea of performance tradeoffs
and make it easier to pick the best library for the job. They also help people keep track of performance regressions,
and aid in finding and fixing performance bottlenecks.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

You can write benchmarks much like tests; using a `#[bench]` annotation in your library code or in a
dedicated file under `benches/`. You can also use `[[bench]]` entries in your `Cargo.toml` to place
it in a custom location.


A benchmarking function looks like this:

```rust
use std::test::Bencher;

#[bench]
fn my_benchmark(bench: &mut Bencher) {
    let x = do_some_setup();
    bench.iter(|| x.compute_thing());
    x.teardown();
}
```

`Bencher::iter` is where the actual code being benchmarked is placed. It will run the
test multiple times until it has a clear idea of what the average time taken is,
and the variance.

The benchmark can be run with `cargo bench`.

To ensure that the compiler doesn't optimize things away, use `test::black_box`.
The following code will show very little time taken because of optimizations, because
the optimizer knows the input at compile time and can do some of the computations beforehand.

```rust
use std::test::Bencher;

fn pow(x: u32, y: u32) -> u32 {
    if y == 0 {
        1
    } else {
        x * pow(x, y - 1)
    }
}

#[bench]
fn my_benchmark(bench: &mut Bencher) {
    bench.iter(|| pow(4, 30));
}
```

```
running 1 test
test my_benchmark ... bench:           4 ns/iter (+/- 0)

test result: ok. 0 passed; 0 failed; 0 ignored; 1 measured; 0 filtered out
```

However, via `mem::black_box`, we can blind the optimizer to the input values,
so that it does not attempt to use them to optimize the code:

```rust
#[bench]
fn my_benchmark(bench: Bencher) -> BenchResult {
    let x = mem::black_box(4);
    let y = mem::black_box(30);
    bench.iter(|| pow(x, y))
}
```

```
running 1 test
test my_benchmark ... bench:          11 ns/iter (+/- 2)

test result: ok. 0 passed; 0 failed; 0 ignored; 1 measured; 0 filtered out
```

Any result that is yielded from the callback for `Bencher::iter()` is also
black boxed; otherwise, the compiler might notice that the result is unused and
optimize out the entire computation.

In case you are generating unused values that do not get returned from the callback,
use `black_box()` on them as well:

```rust
#[bench]
fn my_benchmark(bench: &mut Bencher) {
    let x = mem::black_box(4);
    let y = mem::black_box(30);
    bench.iter(|| {
        black_box(pow(y, x));
        pow(x, y)
    });
}
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The bencher reports the median value and deviation (difference between min and max).
Samples are [winsorized], so extreme outliers get clamped.

Avoid calling `iter` multiple times in a benchmark; each call wipes out the previously
collected data.

`cargo bench` essentially takes the same flags as `cargo test`, except it has a `--bench foo`
flag to select a single benchmark target.


 [winsorized]: https://en.wikipedia.org/wiki/Winsorizing

# Drawbacks
[drawbacks]: #drawbacks

The reason we haven't stabilized this so far is basically because we're hoping to have a custom test
framework system, so that the bencher can be written as a crate. This is still an alternative, though
there has been no movement on this front in years.

# Rationale and alternatives
[alternatives]: #alternatives

This design works. It doesn't give you fine grained tools for analyzing results, but it's
a basic building block that lets one do most benchmarking tasks. The alternatives include
a custom test/bench framework, which is much more holistic, or exposing more
fundamental building blocks.

Another possible API would be one which implicitly handles the black boxing, something
like

```rust
let input1 = foo();
let input2 = bar();
bencher.iter(|(input1, input2)| baz(input1, input2), (input1, input2))
```

This has problems with the types not being Copy, and it feels a bit less flexible.

# Unresolved questions
[unresolved]: #unresolved-questions

- Should stuff be in `std::test` or a partially-stabilized `libtest`?
- Should we stabilize any other `Bencher` methods (like `run_once`)?
- Stable machine-readable output for this would be nice, but can be done in a separate RFC.

