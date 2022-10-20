- Feature Name: infallible_lifetime_extension
- Start Date: 2020-11-08
- RFC PR: [rust-lang/rfcs#3027](https://github.com/rust-lang/rfcs/pull/3027)
- Rust Issue: [rust-lang/rust#80619](https://github.com/rust-lang/rust/issues/80619)

# Summary
[summary]: #summary

Restrict (implicit) [promotion][rfc1414], such as lifetime extension of rvalues, to infallible operations.

[rfc1414]: https://github.com/rust-lang/rfcs/blob/master/text/1414-rvalue_static_promotion.md

# Motivation
[motivation]: #motivation

## Background on promotion and lifetime extension 

Rvalue promotion (as it was originally called) describes the process of taking an rvalue that can be computed at compile-time, and "promoting" it to a constant, so that references to that rvalue can have `'static` lifetime.
It has been introduced by [RFC 1414][rfc1414].
The scope of what exactly is being promoted in which context has been extended over the years in an ad-hoc manner, and the underlying mechanism of promotion (to extract a part of a larger body of code into a separate constant) is now also used for purposes other than making references have `'static` lifetime.
To account for this, the const-eval WG [agreed on the following terminology][promotion-status]:
* Making references have `'static` lifetime is called "lifetime extension".
* The underlying mechanism of extracting part of some code into a constant is called "promotion".

Promotion is currently used for four compiler features:
* lifetime extension
* non-`Copy` array repeat expressions
* functions where some arguments must be known at compile-time (`#[rustc_args_required_const]`)
* `const` operands of `asm!`

These uses of promotion fall into two categories:
* *Explicit* promotion refers to promotion where not promoting is simply not an option: `#[rustc_args_required_const]` and `asm!` *require* the value of this expression to be known at compile-time.
* *Implicit* promotion refers to promotion that might not be required: a reference might not actually need to have `'static` lifetime, and an array repeat expression could be `Copy` (or the repeat count no larger than 1).

For more details, see the [const-eval WG writeup][promotion-status].

## The problem with implicit promotion

Explicit promotion is mostly fine as-is.
This RFC is concerned with implicit promotion.
The problem with implicit promotion is best demonstrated by the following example:

```rust
fn make_something() {
  if false { &(1/0) }
}
```

If the compiler decides to do implicit promotion here, the code is changed to something like

```rust
fn make_something() {
  if false {
    const VAL: &i32 = &(1/0);
    VAL
  }
}
```

However, this code would fail to compile!
When doing code generation for a function, all its constants have to be evaluated, including the ones in dead code, since in general we cannot know that we are compiling dead code.
(In fact, there is even code that [relies on failing constants stopping compilation](https://github.com/rust-lang/rust/issues/67191).)
When evaluating `VAL`, a panic is triggered due to division by zero, so any code that needs to know the value of `VAL` is stuck as there is no such value.

This is a problem because the original code (pre-promotion) works just fine: the division never actually happens.
It is only because the compiler decided to extract the division into a separately evaluated constant that it even becomes a problem.
Notice that this is a problem only for implicit promotion, because with explicit promotion, the value *has* to be known at compile-time -- so stopping compilation if the value cannot be determined is the right behavior.

To solve this problem, every part of the compiler that works with constants needs to be able to handle the case where the constant *has no defined value*, and continue in some correct way.
This is hard to get right, and has lead to a number of problems over the years:
* There has been at least one [soundness issue](https://github.com/rust-lang/rust/issues/50814).
* There are still outstanding [diagnostic issues](https://github.com/rust-lang/rust/issues/61821).
* Promotion needs a special [exception in const-value validation](https://github.com/rust-lang/rust/issues/67534).
* All code handling constants has to carry [extra complexity to support promotion](https://github.com/rust-lang/rust/issues/75461)

This RFC proposes to fix all these problems at once, by restricting implicit promotion to those expression whose evaluation cannot fail.
This is the last step in a series of changes that have been going on for quite some time, starting with the [introduction](https://github.com/rust-lang/rust/pull/53851) of the `#[rustc_promotable]` attribute to control which function calls may be subject to implicit promotion (the original RFC said that all calls to `const fn` should be promoted, but as user-defined `const fn` got closer and closer, that seemed less and less like a good idea, due to all the ways in which evaluating a `const fn` can fail).
Together with [some planned changes for evaluation of regular constants](https://github.com/rust-lang/rust/issues/71800), this means that all CTFE failures can be made hard errors, greatly simplifying the parts of the compiler that trigger evaluation of constants and handle the resulting value or error.

For more details, see [the MCP that preceded this RFC](https://github.com/rust-lang/lang-team/issues/58).

[promotion-status]: https://github.com/rust-lang/const-eval/blob/33053bb2c9a0c6a17acd3116dd47bbb360e060db/promotion.md

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

(Based on [RFC 1414][rfc1414])

Inside a function body's block:

- If a shared reference to a constexpr rvalue is taken. (`&<constexpr>`),
- And the constexpr does not contain a `UnsafeCell { ... }` constructor,
- And the constexpr only consists of operations that will definitely succeed to
  evaluate at compile-time,
- And the resulting value does not need dropping,
- Then instead of translating the value into a stack slot, translate
  it into a static memory location and give the resulting reference a
  `'static` lifetime.

Operations that definitely succeed at the time of writing the RFC include:
- literals of any kind
- constructors (struct/enum/union/tuple)
- struct/tuple field accesses
- arithmetic and logical operators that do not involve division: `+`/`-`/`*`, all bitwise and shift operators, all unary operators

Note that arithmetic overflow is not a problem: an addition in debug mode is compiled to a `CheckedAdd` MIR operation that never fails, which returns an `(<int>, bool)`, and is followed by a check of said `bool` to possibly raise a panic.
We only ever promote the `CheckedAdd`, so evaluation of the promoted will never fail, even if the operation overflows.
For example, `&(1 + u32::MAX)` turns into something like:
```rust
const C: (u32, bool) = CheckedAdd(1, u32::MAX); // evaluates to (0, true).
assert!(C.1 == false);
&C.0
```
See [this prior RFC](https://github.com/rust-lang/rfcs/blob/master/text/1211-mir.md#overflow-checking) for further details.

However, also note that operators being infallible is more subtle than it might seem.
In particular, it requires that all constants of integer type (and even all integer-typed fields of all constants) be proper integers, not pointers cast to integers.
The following code shows a problematic example:
```rust
const FOO: usize = &42 as *const i32 as usize;
let x: &usize = &(FOO * 3);
```
`FOO*3` cannot be evaluated during CTFE, so to ensure that multiplication is infallible, we need to ensure that all constants used in promotion are proper integers.
This is currently ensured by the "validity check" that is performed on the final value of each constant: the check recursively traverses the type of the constant and ensures that the data matches that type.

Operations that might fail include:
- `/`/`%`
- `panic!` (including the assertion that follows `Checked*` arithmetic to ensure that no overflow happened)
- array/slice indexing
- any unsafe operation
- `const fn` calls (as they might do any of the above)

Notably absent from *both* of the above list is dereferencing a reference.
This operation is, in principle, infallible---but due to the concern mentioned above about validity of consts, it is only infallible if the validity check in constants traverses through references.
Currently, the check stops when hitting a reference to a static, so currently, dereferencing a reference can *not* be considered an infallible operation for the purpose of promotion.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

See above for (hopefully) all the required details.
What exactly the rules will end up being for which operations can be promoted will depend on experimentation to avoid breaking too much existing code, as discussed below.

# Drawbacks
[drawbacks]: #drawbacks

The biggest drawback is that this will break some existing code.
Compared to the status quo, this means the following expressions are not implicitly promoted any more:
* Division, modulo, array/slice indexing
* `const fn` calls in `const`/`static` bodies (`const fn` are already not being implicitly promoted in `fn` and `const fn` bodies)

If code relies on implicit promotion of these operations, it will stop to compile.
Crater runs should be used all along the way to ensure that the fall-out is acceptable.
The language team will be involved (via FCP) in each breaking change to make this judgment call.
If too much code is broken, various ways to weaken this proposal (at the expense of more technical debt, sometimes across several parts of the compiler) are [described blow][rationale-and-alternatives].

The long-term plan is that such code can switch to [inline `const` expressions](2920-inline-const.md) instead.
However, inline `const` expressions are still in the process of being implemented, and for now are specified to not support code that depends on generic parameters in the context, which is a loss of expressivity when compared with implicit promotion.
More complex work-around are possible for this using associated `const`, but they can become quite tedious.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The rationale has been described with the motivation.

Unless we want to keep supporting fallible const-evaluation indefinitely, the main alternatives are devising more precise analyses to determine if some operation is infallible.
For example, we could still perform implicit promotion for division and modulo if the divisor is a non-zero literal.
We could also have `CheckedDiv` and `CheckedMod` operations that, similar to operations like `CheckedAdd`, always returns a result of the right type together with a `bool` saying if the result is valid.
We could still perform *array* indexing if the index is a constant and in-bounds.
For slices, we could have an analysis that predicts the (minimum) length of the slice.
Notice that promotion happens in generic code and can depend on associated constants, so we cannot, in general, *evaluate* the implicit promotion candidate to check if that causes any errors.

We could also decide to still perform implicit promotion of potentially fallible operations in the bodies of `const`s and `static`s.
(This would mean that the RFC only changes behavior of implicit promotion in `fn` and `const fn` bodies.)
This is possible because that code is not subject to code generation, it is only interpreted by the CTFE engine.
The engine will only evaluate the part of the code that is actually being run, and thus can avoid evaluating promoteds in dead code.
However, this means that all other consumers of this code (such as pretty-printing and optimizations) must *not* evaluate promoteds that they encounter, since that evaluation may fail.
This will incur technical debt in all of those places, as we need to carefully ensure not to eagerly evaluate all constants that we encounter.
We also need to be careful to still evaluate all user-defined constants even inside promoteds in dead code (because, remember, code may rely on the fact that compilation will fail if any constant that is syntactically used in a function fails to evaluated).
Note that this is *not* an option for code generation, i.e., for code in `fn` and `const fn`: all code needs to be translated to LLVM, even possibly dead code, so we have to evaluate all constants that we encounter.

If there are some standard library `const fn` that cannot fail to evaluate, and that form the bulk of the function calls being implicitly promoted, we could add the `#[rustc_promotable]` attribute to them to enable implicit promotion.
This will not help, however, if there is plenty of code relying on implicit promotion of user-defined `const fn`.

Conversely, if this plan all works out, one alternative proposal that goes even further is to restrict implicit promotion to expressions that would be permitted in a pattern.
This would avoid adding a new class of expression in between "patterns" and "const-evaluable".
On the other hand, it is much more restrictive (basically allowing only literals and constructors), and does not actually help simplify the compiler.

# Prior art
[prior-art]: #prior-art

A few changes have landed in the recent past that already move us, step-by-step, towards the goal outlined in this RFC:
* Treat `const fn` like `fn` for promotability: https://github.com/rust-lang/rust/pull/75502, https://github.com/rust-lang/rust/pull/76411
* Do not promote `union` field accesses: https://github.com/rust-lang/rust/pull/77526

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The main open question is to what extend existing code relies on lifetime extension of fallible operations, i.e., if we can get away with the plan outlined here.
(Lifetime extension is currently the only stable form of implicit promotion, and thus the only one relevant for backwards compatibility.)
In `fn` and `const fn`, only a few fallible operations remain: division, modulo, and slice/array indexing.
In `const` and `static`, we additionally promote calls to arbitrary `const fn`, which of course could fail in arbitrary ways -- crater experiments will have to show if code actually relies on this.
A fall-back plan in case this RFC would break too much code has been [described above][rationale-and-alternatives].

# Future possibilities
[future-possibilities]: #future-possibilities

A potential next step after this RFC could be to tackle the remaining main promotion "hack", the `#[rustc_promotable]` attribute.
We now know exactly what this attribute expresses: this `const fn` may never fail to evaluate (in particular, it may not panic).
This provides a theoretical path to stabilization of this attribute, backed by an analysis that ensures that the function indeed does not panic.
(However, once inline `const` expressions with generic parameters are stable, this does not actually grant any extra expressivity, just a slight increase in convenience.)
