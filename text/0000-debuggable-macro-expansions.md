- Feature Name: debuggable-macro-expansions
- Start Date: 2017-08-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

By default, annotate code expanded from a macro with debug location info
corresponding to the macro definition (i.e. the behavior that's currently
available on nightly via `-Zdebug-macros`). Add an annotation
`#[collapse_debuginfo]` to enable a particular macro definition to opt to have
the expansion annotated with debug location info corresponding to the macro
invocation (i.e. the current default behavior).

# Motivation
[motivation]: #motivation

Currently, the debug location info associated with code expanded from a macro
claims that the location of the expanded code is the location of the macro
invocation. This serves two purposes: First, it makes panic stacks point to
the location in the code where a panicking assertion was invoked as opposed to
pointing to the panic machinery internals. Second, it results in debuggers
skipping over print-like macros like `println!` that programmers virtually
always want to skip over when debugging and that due to current technical
limitations cannot be made skippable without claiming collapsed location info
for their expansion.

However, the approach of collapsing debug location information yields bad
results for macros that are not assert-like and not print-like but that expand
to substantial code. The current approach of collapsing debug information by
default makes non-assert-like, non-print-like macro usage result in code that
is on debuggable, that doesn't get useful panic/crash stacks in CI or in the
field (e.g. in the context of the Firefox crash reporter) at that doesn't get
useful profiling attribution.

Opting into non-collapsed location information using the `-Zdebug-macros` flag
is not a sufficient remedy because

1. It is nightly-only

2. For the panic/crash stack use case it makes the location information for
assert-like, so the panic/crash stack use case really needs to have it both
ways depending on the nature of the macro in the same build

3. Since Firefox supports profiling the same builds that are delivered to
users by the means of the [Gecko Profiler](https://developer.mozilla.org /en-
US/docs/Mozilla/Performance/Profiling_with_the_Built-in_Profiler), solutions
that address the profiling use case should not require a special build.

Changing the default behavior to the current behavior of `-Zdebug-macros` of
the location info reflecting the macro definition allows the debugging, crash
attribution and profiling attribution of code expanded from macros in the
general case, while allowing specific macros to opt into collapsed location
info reflecting the macro invocation site allows the use cases that motivate
the current default behavior of collapsed location info to continue to be
addressed.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

(Assume a guide section title like "Debug info for code expanded from
(macros".)

By default, debug location information for code expanded from macros reflects
the location of the corresponding source text, as one would expect. That is,
given a macro definition and invocation like this

```rust
macro_rules! outer {  //  1
    ($b:block) =>     //  2
    {                 //  3
        one();        //  4
        inner!();     //  5
        $b            //  6
    }                 //  7
}                     //  8
                      //  9
macro_rules! inner {  // 10
    () =>             // 11
    {                 // 12
        two();        // 13
    }                 // 14
}                     // 15
                      // 16
fn f() {              // 17
    outer!({          // 18
        three();      // 19
        four();       // 20
    });               // 21
}                     // 22

```

the expansion has line numbers like this

```rust
fn f() {              // 17
    {                 //  3
        one();        //  4
        {             // 12
            two();    // 13
        }             // 14
        {             // 18
            three();  // 19
            four();   // 20
        }             // 21
    }                 //  7
}                     // 22

```

This default behavior produces undesirable results if you create convenience
macros for assertions. For assertion-like macros, one generally once the panic
stack location information to point to the outermost invocation site of an
assert-like macro instead of pointing to the internals of the panicking
implementation. To address the different debug location information needs of
assert-like macros there is, there is the annotation `#[collapse_debuginfo]`.
When a macro definition is annotated with `#[collapse_debuginfo]`, the debug
location information for the code expanded from the macro is the location of
the invocation of the macro or, if the macro is itself invoked from another
macro annotated with `#[collapse_debuginfo]`, the first invocation location
upwards that doesn't itself reside in a macro annotated with
`#[collapse_debuginfo]`.

If the macros in the above example had been annotated with
`#[collapse_debuginfo]`, the expansion would instead have line numbers like
this

```rust
fn f() {              // 17
    {                 // 18
        one();        // 18
        {             // 18
            two();    // 18
        }             // 18
        {             // 18
            three();  // 18
            four();   // 18
        }             // 18
    }                 // 18
}                     // 22

```

Assert-like standard-library macros like `assert!`, `panic!`, etc. are
annotated with `#[collapse_debuginfo]`. If you build a domain-specific
assertion macro on top of them, e.g. if you were defined an
`assert_ready_state!` macro on top of `assert!` when writing document loading
code for a Web browser, you should annotate your domain-specific assertion
macro with `#[collapse_debuginfo]`.

Additionally, some non-assert-like macros in the standard library, such as
`println!` are annotated with `#[collapse_debuginfo]`, because programmers
almost always want to step over them in a debugger (as opposed to stepping
into the implementation), but current limitations in what debugging
information can represent require the collapsing of debug location information
in order to get the "step over" behavior in debuggers.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Implementation

When expanding a macro that isn't annotated with `#[collapse_debuginfo]`, use
the code that currently runs if `-Zdebug-macros` is set.

When expanding a macro that is annotated with `#[collapse_debuginfo]`, use the
code that currently runs by default.

Add `#[collapse_debuginfo]` to the macros in `src/libstd/macros.rs` as well as
macros whose name starts with `assert_` elsewhere in the standard library.


## Reference text

By default, debug location information corresponds to the source code location
of the line being run. For lines expanded from a macro, this means the source
code lines on which the body of the macro is defined. For lines in a block of
code past as an argument to a macro, this means a source code lines where the
argument block is defined.

As an exception, the debug location information for code expanded from macros
annotated with `#[collapse_debuginfo]` is the location of the macro invocation
(regardless of whether the expanded code came from the macro definition or
from macro arguments). The location of the invocation site the location
associated with the invocation site in the AST after the previously-performat
macro expansions. That is, if a macro annotated with `#[collapse_debuginfo]`
invokes another macro annotated with `#[collapse_debuginfo]`, the code
expanded from the inner macro gets the debug location of the invocation site
of the outer macro.

# Drawbacks
[drawbacks]: #drawbacks

While the distinction between assert-like and non-assert-like and the desired
behavior is clear and tied to the nature of the macro and, therefore, it's OK
to leave the decision on the debug info behavior to the macro definition site,
the desired results for println-like macros are somewhat more dependent on the
circumstances of the macro user, but this solution still leaves the decision
to the crate that defines a macro as opposed to the crate invoking the macro.

# Rationale and Alternatives
[alternatives]: #alternatives

The primary alternative would have been exposing `-Zdebug-macros` outside
nightly. The reason why that is not a sufficient remedy is that for the panic
stack use case, it is necessary to have different treatment for assert-like
and non-assert-like macros.

# Unresolved questions
[unresolved]: #unresolved-questions

Should `-Zdebug-macros` remain as a way to ignore `#[collapse_debuginfo]`?

Should the location info for code in a block passed as an argument to a macro
annotated with `#[collapse_debuginfo]` get the novel behavior (novel in the
sense of being neither the current default nor the `-Zdebug-macros` behavior)
of getting the location information for the source text where the argument
block is defined even if lines defined in the macro body get the location of
the macro invocation?
