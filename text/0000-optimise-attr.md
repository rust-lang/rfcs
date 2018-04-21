- Feature Name: optimise_attr
- Start Date: 2018-03-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC introduces the `#[optimise]` attribute, specifically its `#[optimise(size)]` variant for
controlling optimisation level on a per-item basis.

# Motivation
[motivation]: #motivation

Currently, rustc has only a small number of optimisation options that apply globally to the
crate. With LTO and RLIB-only crates these options become applicable to a whole-program, which
reduces the ability to control optimisation even further.

For applications such as embedded, it is critical, that they satisfy the size constraints. This
means, that code must consciously pick one or the other optimisation level. However, since
optimisation level is increasingly applied program-wide, options like `-Copt-level=3` or
`-Copt-level=s` are less and less useful – it is no longer feasible (and never was feasible with
cargo) to use the former one for code where performance matters and the latter everywhere else.

With a C toolchain this is fairly easy to achieve by compiling the relevant objects with different
options. In Rust ecosystem, however, where this concept does not exist, an alternate solution is
necessary.

With `#[optimise(size)]` it is possible to annotate separate functions, so that they are optimised
for size in a project otherwise optimised for speed (which is the default for `cargo --release`).

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Sometimes, optimisations are a tradeoff between execution time and the code size. Some
optimisations, such as loop unrolling increase code size many times on average (compared to
original function size).

```rust
#[optimise(size)]
fn banana() {
    // code
}
```

Will instruct rustc to consider this tradeoff more carefully and avoid optimising in a way that
would result in larger code rather than a smaller one. It may also have effect on what instructions
are selected to appear in the final binary.

Note that `#[optimise(size)]` is a hint, rather than a hard requirement and compiler may still,
while optimising, take decisions that increase function size compared to an entirely unoptimised
result.

Using this attribute is recommended when inspection of generated code reveals unnecessarily large
function or functions, but use of `-O` is still preferable over `-C opt-level=s` or `-C
opt-level=z`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `#[optimise(size)]` attribute applied to a function definition will instruct the optimisation
engine to avoid applying optimisations that could result in a size increase and machine code
generator to generate code that’s smaller rather than larger.

Note that the `#[optimise(size)]` attribute is just a hint and is not guaranteed to result in any
different or smaller code.

Since `#[optimise(size)]` instructs optimisations to behave in a certain way, this means that this
attribute has no effect when no optimisations are run (such as is the case when `-Copt-level=0`).
Interaction of this attribute with the `-Copt-level=s` and `-Copt-level=z` flags is not specified
and is left up to implementation to decide.

# Drawbacks
[drawbacks]: #drawbacks

* Not all of the alternative codegen backends may be able to express such a request, hence the
“this is an optimisation hint” note on the `#[optimise(size)]` attribute.
    * As a fallback, this attribute may be implemented in terms of more specific optimisation hints
      (such as `inline(never)`, the future `unroll(never)` etc).

# Rationale and alternatives
[alternatives]: #alternatives

Proposed is a very semantic solution (describes the desired result, instead of behaviour) to the
problem of needing to sometimes inhibit some of the trade-off optimisations such as loop unrolling.

Alternative, of course, would be to add attributes controlling such optimisations, such as
`#[unroll(no)]` on top of a a loop statement. There’s already precedent for this in the `#[inline]`
annotations.

The author would like to argue that we should eventually have *both*, the `#[optimise(size)]` for
people who look at generated code and decide that it is too large, and the targetted attributes for
people who know *why* the code is too large.

Furthermore, currently `optimise(size)` is able to do more than any possible combination of
targetted attributes would be able to such as influencing the instruction selection or switch
codegen strategy (jump table, if chain, etc.) This makes the attribute useful even in presence of
all the targetted optimisation knobs we might have in the future.

---

Alternative: `optimize` (American English) instead of `optimise`… or both?

# Prior art
[prior-art]: #prior-art

* LLVM: `optsize`, `optnone`, `minsize` function attributes (exposed in Clang in some way);
* GCC: `__attribute__((optimize))` function attribute which allows setting the optimisation level
and using certain(?) `-f` flags for each function;
* IAR: Optimisations have a checkbox for “No size constraints”, which allows compiler to go out of
its way to optimise without considering the size tradeoff. Can only be applied on a
per-compilation-unit basis. Enabled by default, as is appropriate for a compiler targetting
embedded use-cases.

# Unresolved questions
[unresolved]: #unresolved-questions

* Should we support such an attribute at module-level? Crate-level?
    * If yes, should we also implement `optimise(always)`? `optimise(level=x)`?
        * Left for future discussion, but should make sure such extension is possible.
