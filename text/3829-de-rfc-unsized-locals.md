- Feature Name: `unsized_locals`
- Start Date: 2025-06-02
- RFC PR: [rust-lang/rfcs#3829](https://github.com/rust-lang/rfcs/pull/3829)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


_Following the great success of the [type ascription de-RFC], we now present the next one in the series!_

# Summary
[summary]: #summary

Unsized locals ([RFC 1909], called "unsized rvalues" originally)
has been a merged RFC for eight years with no clear path to stabilization.

There is still a very large gap in the implementation in rustc that hasn't been addressed for many years and there are several language problems with this feature. 

This RFC intends to advocate for the feature being removed entirely with a fresh RFC being necessary to add a similar feature again.

Note that the acceptance of this RFC should not be taken as judgement on a future RFC. A fresh RFC with a new design would be required, and this RFC may provide input on design constraints for such an RFC, but this should not be taken as pre-rejecting such an RFC.

# Demotivation
[demotivation]: #demotivation

The `unsized_locals` feature is simple to explain on the surface: local variables no longer have to be `Sized`.

```rust
#![feature(unsized_locals)]
fn main() {
    let x = *Box::from("hello, world!");
}
```

This will dynamically allocate space on the stack for the string.
C has a similar feature, [`alloca`] and variable length arrays (VLA), the latter having been made optional to implement in C11.

## Lack of Proper Implementation

This feature has never been properly implemented in rustc.
The variable length array form proposed in the RFC still doesn't exist at all, only the unsized local variable does.
It is implemented in the type checker and codegen, but [lacking MIR semantics](https://github.com/rust-lang/rust/issues/48055#issuecomment-1837424794) and therefore unimplemented in the compile time function evaluator.
This is very significant, as MIR semantics govern how the feature should behave precisely in the first place.
Without them, they cannot work in `const` and optimizations are likely broken around them.
Because of this lack of implementation quality, the `unsized_locals` feature was already accepted for removal from rustc two years ago in [MCP 630].
This removal hasn't yet been implemented.

Dynamic stack allocation as currently implemented interacts especially poorly with loops.
Allocations are not freed until the function returns, so the following example overflows the stack:

```rust
#![feature(unsized_locals)]
fn main() {
    let s = "A".repeat(1000);
    for i in 0..1000000 {
        let x: str = *Box::from(s.as_str());
        std::hint::black_box(&s);
    }
}
```
There are ways around this (rewinding the stack pointer at the end of the loop to free up the memory), but they are not currently implemented.

## Implicit Danger

While lack of implementation quality is a sign of lack of interest for the feature, it is not the primary reason for this RFC,
which is purely about the language design of the feature.

The original RFC was very short, and especially short on motivation and rationale for the design.

Dynamic stack allocation in general has a rather significant downside: it makes it easy to accidentally overflow the stack if you allocate a lot.
Stacks are usually rather small (on the order of few megabytes, depending on the platform), which means that dynamically allocating user-controlled input on the stack is often rather dangerous.
While stack overflows are not considered memory unsafe by Rust, they still cause crashes which can lead to denial of service vulnerabilities and unreliability in general.

Dynamic stack allocation also has its upsides.
It is generally faster than heap allocation and can therefore improve performance in cases where the previously mentioned downsides are not a concern.
Therefore, this RFC is not necessarily a rejection of the idea of dynamic stack allocation, but merely the way the `unsized_locals` feature exposes it.

Which is where we get to the major argument of this RFC: The `unsized_locals` feature integrates far too naturally into Rust.
This makes the feature very **easy to use**, and especially **easy to use accidentally**.

As previously mentioned, dynamic stack allocation can be dangerous and should not be used lightly.
It's an advanced optimization feature that is best left untouched by default.
As such, it behaves similarly to `unsafe` (but is not actually `unsafe`).
With `unsized_locals`, the use of dynamic stack allocation is completely implicit.
When you create an unsized local, it is often not obvious that dynamic stack allocation is happening.
In the example from the start, we need to be aware of all the involved types (which are often inferred in practice) to know that this is a potentially problematic unsized local that we have to audit more carefully instead of a normal sized local.
Especially around strings, which are often user-controlled, this easily leads to accidentally-dangerous situations.

Rust's strings, reference types, and `Sized` are a part of the language that can often be hard to understand for beginners coming from garbage-collected languages.
By allowing people to create a dynamic stack allocation without being aware of what is happening, we open the doors for people, especially new Rust programmers who are not intimately familiar with the tradeoffs of dynamic stack allocation, to shoot themselves in the foot and become vulnerable to unexpected crashes.

As a _dangerous_ feature, dynamic stack allocation must be explicit and obvious - and `unsized_locals` makes it implicit _by design_, a major downside of this feature that has not been addressed in prior RFCs.

`unsized_locals` is not the only feature that can cause unbounded stack allocation and eventually lead to stack overflow in Rust, one has to look no further than Dijkstra's favorite: recursion.
A program with unbounded recursion is easily found and fixed, but especially recursive parsers suffer from similar problems where user input can cause stack overflows.
What makes recursion different from dynamic stack allocation?
The main difference here is that recursion is a lot harder to do on accident.
When recursion is used, it is usually used on purpose, and while sometimes the potential for stack overflows is overlooked, the general feature is usually used on purpose.
That said, recursion can certainly be dangerous in some contexts, but prior existing features are not a good reason to introduce more ways to blow the stack.

The Linux kernel has spent a lot of time on getting VLAs (C's cousin to `unsized_locals`) [removed from the codebase](https://www.phoronix.com/news/Linux-Kills-The-VLA).

# Guide-level obfuscation
[guide-level-obfuscation]: #guide-level-obfuscation

The `unsized_locals` feature is removed from the compiler and [RFC 1909] is officially unaccepted.

If someone wants to bring dynamic stack allocation into Rust again, a new design will have to be designed from scratch, considering all the problems laid out in this RFC.
Alternatively, the existing design of `unsized_locals` *could* be RFCed again, if such an RFC was able to work around all the problems.
This RFC merely unaccepts the previous `unsized_locals` RFC, it does not necessarily banish `unsized_locals` from existence forever.

This does not have a negative effect on features that feature unsized values in function signatures like `unsized_fn_params`.
Their behavior is much more clear and they are implemented differently.
In fact, `unsized_fn_params` is currently needed in the standard library to implement `Box<dyn FnOnce()> as FnOnce()>`.

# Drawforwards
[drawforwards]: #drawforwards

This feature has the previously mentioned performance upsides that users could profit from if it was stabilized.
But this benefit applies to other ways to expose dynamic stack allocation too, and other ways are likely to be easier to implement correctly and stabilize.

# Irrationale and alternatives
[irrationale-and-alternatives]: #irrationale-and-alternatives

If nothing is done on the language side, the feature will likely still be removed from the compiler.
This puts the feature into a really bad position, but it may be readded in the future if someone desires.
With this RFC, the fate of `unsized_locals` is sealed and it becomes clear to anyone what the state of the feature is.

The intent of this RFC is not to proposed an alternative way to solve dynamic stack allocation,
but there are some listed alternatives here that may be considered in the future if desired.
While this RFC doesn't explicitly encourage people to revisit this topic, it may result in new activity around dynamic stack allocation in Rust.

[RFC 1808] proposed an `alloca` function, which was rejected because `alloca` does not really behave like a function.

[RFC 1808] then changed to propose the the VLA syntax instead.
It was rejected in favor of more general unsized values, which culminated in [RFC 1909].

The [`alloca` crate](https://crates.io/crates/alloca) implements dynamic stack allocation via a closure indirection and FFI with C.

The `unsized_fn_params` feature doesn't suffer from the same problems as `unsized_locals` and will still be kept around.
It is independent of this RFC.

# Posterior art
[posterior-art]: #posterior-

The best prior art for this removal is of course the inspiration for the de-RFC format, the [type ascription de-RFC].
[MCP 630] can also be seen as prior art.

# Unresolved answers
[unresolved-answers]: #unresolved-answers


None

# Future probabilities
[future-probabilities]: #future-probabilities

In the future, dynamic stack allocation may be re-added to Rust via some other feature that solves the explicitness problems outlined in the motivation.

Alternatively, it could be decided, explicitly or implicitly through inaction, that dynamic stack allocation is not a fit for Rust and will not be added.

[type ascription de-RFC]: https://rust-lang.github.io/rfcs/3307-de-rfc-type-ascription.html
[`alloca`]: https://man7.org/linux/man-pages/man3/alloca.3.html
[MCP 630]: https://github.com/rust-lang/compiler-team/issues/630
[RFC 1808]: https://github.com/rust-lang/rfcs/pull/1808
[RFC 1909]: https://rust-lang.github.io/rfcs/1909-unsized-rvalues.html
