- Feature Name: `optimize_attr`
- Start Date: 2018-03-26
- RFC PR: [rust-lang/rfcs#2412](https://github.com/rust-lang/rfcs/pull/2412)
- Rust Issue: [rust-lang/rust#54882](https://github.com/rust-lang/rust/issues/54882)

## Summary
[summary]: #summary

This RFC introduces the `#[optimize]` attribute for controlling optimization level on a per-item
basis.

## Motivation
[motivation]: #motivation

Currently, rustc has only a small number of optimization options that apply globally to the
crate. With LTO and RLIB-only crates these options become applicable to a whole-program, which
reduces the ability to control optimization even further.

For applications such as embedded, it is critical, that they satisfy the size constraints. This
means, that code must consciously pick one or the other optimization level. Absence of a method to
selectively optimize different parts of a program in different ways precludes users from utilising
the hardware they have to the greatest degree.

With a C toolchain selective optimization is fairly easy to achieve by compiling the relevant
codegen units (objects) with different options. In Rust ecosystem, where the concept of such units
does not exist, an alternate solution is necessary.

With the `#[optimize]` attribute it is possible to annotate the optimization level of separate
items, so that they are optimized differently from the global optimization option.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### `#[optimize(size)]`

Sometimes, optimizations are a trade-off between execution time and the code size. Some
optimizations, such as loop unrolling increase code size many times on average (compared to
original function size) for marginal performance benefits. In case such optimization is not
desirable…

```rust
#[optimize(size)]
fn banana() {
    // code
}
```

…will instruct rustc to consider this trade-off more carefully and avoid optimising in a way that
would result in larger code rather than a smaller one. It may also have effect on what instructions
are selected to appear in the final binary.

Note that `#[optimize(size)]` is a hint, rather than a hard requirement and compiler may still,
while optimising, take decisions that increase function size compared to an entirely unoptimized
result.

Using this attribute is recommended when inspection of generated code reveals unnecessarily large
function or functions, but use of `-O` is still preferable over `-C opt-level=s` or `-C
opt-level=z`.

### `#[optimize(speed)]`

Conversely, when one of the global optimization options for code size is used (`-Copt-level=s` or
`-Copt-level=z`), profiling might reveal some functions that are unnecessarily “hot”. In that case,
those functions may be annotated with the `#[optimize(speed)]` to make the compiler make its best
effort to produce faster code.

```rust
#[optimize(speed)]
fn banana() {
    // code
}
```

Much like with `#[optimize(size)]`, the `speed` counterpart is also a hint and will likely not
yield the same results as using the global optimization option for speed.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `#[optimize(size)]` attribute applied to an item or expression will instruct the optimization
pipeline to avoid applying optimizations that could result in a size increase and machine code
generator to generate code that’s smaller rather than faster.

The `#[optimize(speed)]` attribute applied to an item or expression will instruct the optimization
pipeline to apply optimizations that are likely to yield performance wins and machine code
generator to generate code that’s faster rather than smaller.

The `#[optimize]` attributes are just a hint to the compiler and are not guaranteed to result in
any different code.

If an `#[optimize]` attribute is applied to some grouping item (such as `mod` or a crate), it
propagates transitively to all items defined within the grouping item. Note, that a function is
also a “grouping” item for the purposes of this RFC, and `#[optimize]` attribute applied to a
function will propagate to other functions or closures defined within the body of the function.

`#[optimize]` attribute may also be applied to a closure expression using the currently unstable
`stmt_expr_attributes` feature.

It is an error to specify multiple incompatible `#[optimize]` options to a single item or
expression at once.  A more explicit `#[optimize]` attribute overrides a propagated attribute.

`#[optimize(speed)]` is a no-op when a global optimization for speed option is set (i.e. `-C
opt-level=1-3`). Similarly `#[optimize(size)]` is a no-op when a global optimization for size
option is set (i.e. `-C opt-level=s/z`). `#[optimize]` attributes are no-op when no optimizations
are done globally (i.e. `-C opt-level=0`). In all other cases the *exact* interaction of the
`#[optimize]` attribute with the global optimization level is not specified and is left up to
implementation to decide.

`#[optimize]` attribute applied to non function-like items (such as `struct`) or non function-like
expressions (i.e. not closures) is considered “unused” as of this RFC and should fire the
`unused_attribute` lint (unless the same attribute was used for a function-like item or expression,
via e.g.  propagation). Some future RFC may assign some behaviour to this attribute with respect to
such definitions.

## Implementation approach

For the LLVM backend, these attributes may be implemented in a following manner:

`#[optimize(size)]` – explicit function attributes exist at LLVM level. Items with
`optimize(size)` would simply apply the LLVM attributes to the functions.

`#[optimize(speed)]` in conjunction with `-C opt-level=s/z` – use a global optimization level of
`-C opt-level=2/3` and apply the equivalent LLVM function attribute (`optsize`, `minsize`) to all
items which do not have an `#[optimize(speed)]` attribute.

## Drawbacks
[drawbacks]: #drawbacks

* Not all of the alternative codegen backends may be able to express such a request, hence the
“this is a hint” note on the `#[optimize]` attribute.
    * As a fallback, this attribute may be implemented in terms of more specific optimization hints
      (such as `inline(never)`, the future `unroll(never)` etc).

## Rationale and alternatives
[alternatives]: #alternatives

Proposed is a very semantic solution (describes the desired result, instead of behaviour) to the
problem of needing to sometimes inhibit some of the trade-off optimizations such as loop unrolling.

Alternative, of course, would be to add attributes controlling such optimizations, such as
`#[unroll(no)]` on top of a loop statement. There’s already precedent for this in the `#[inline]`
annotations.

The author would like to argue that we should eventually have *both*, the `#[optimize]` for
people who look at generated code but are not willing to dig for exact reasons, and the targeted
attributes for people who know *why* the code is not satisfactory.

Furthermore, currently `optimize` is able to do more than any possible combination of targeted
attributes would be able to such as influencing the instruction selection or switch codegen
strategy (jump table, if chain, etc.) This makes the attribute useful even in presence of all the
targeted optimization knobs we might have in the future.

## Prior art
[prior-art]: #prior-art

* LLVM: `optsize`, `optnone`, `minsize` function attributes (exposed in Clang in some way);
* GCC: `__attribute__((optimize))` function attribute which allows setting the optimization level
and using certain(?) `-f` flags for each function;
* IAR: Optimizations have a check box for “No size constraints”, which allows compiler to go out of
its way to optimize without considering the size trade-off. Can only be applied on a
per-compilation-unit basis. Enabled by default, as is appropriate for a compiler targeting
embedded use-cases.

## Unresolved questions
[unresolved]: #unresolved-questions

* Should we also implement `optimize(always)`? `optimize(level=x)`?
    * Left for future discussion, but should make sure such extension is possible.
* Should there be any way to specify what global optimization for speed level is used in
  conjunction with the optimization for speed option (e.g. `-Copt-level=s3` could be equivalent to
  `-Copt-level=3` and `#[optimize(size)]` on the crate item);
    * This may matter for users of `#[optimize(speed)]`.
* Are the propagation and `unused_attr` approaches right?
