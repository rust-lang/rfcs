- Feature Name: alloca
- Start Date: 2016-12-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add variable-length arrays to the language.

# Motivation
[motivation]: #motivation

Some algorithms (e.g. sorting, regular expression search) need a one-time backing store for a number of elements only
known at runtime. Reserving space on the heap always takes a performance hit, and the resulting deallocation can
increase memory fragmentation, possibly slightly degrading allocation performance further down the road.

If Rust included this zero-cost abstraction, more of these algorithms could run at full speed – and would be available
on systems without an allocator, e.g. embedded, soft- or hard-real-time systems. The option of using a fixed slice up
to a certain size and using a heap-allocated slice otherwise (as afforded by
[SmallVec](https://crates.io/crates/smallvec)-like classes) has the drawback of decreasing memory locality if only a
small part of the fixed-size allocation is used – and even those implementations could potentially benefit from the
increased memory locality.

As a (flawed) benchmark, consider the following C program:

```C
#include <stdlib.h>

int main(int argc, char **argv) {
    int n = argc > 1 ? atoi(argv[0]) : 1;
    int x = 1;
    char foo[n];
    foo[n - 1] = 1;
}
```

Running `time nice -n 20 ionice ./dynalloc 1` returns almost instantly (0.0001s), whereas using `time nice -n 20 ionice
./dynalloc 200000` takes 0.033 seconds. As such, it appears that just by forcing the second write further away from the
first slows down the program (this benchmark is actually completely unfair, because by reducing the process' priority,
we invite the kernel to swap in a different process instead, which is very probably the major cause of the slowdown
here).

Still, even with the flaws in this benchmark,
[The Myth of RAM](http://www.ilikebigbits.com/blog/2014/4/21/the-myth-of-ram-part-i) argues quite convincingly for the
benefits of memory frugality.

# Detailed design
[design]: #detailed-design

So far, the `[T]` type could not be constructed in valid Rust code. It will now represent compile-time unsized (also
known as "variable-length") arrays. The syntax to construct them could simply be `[t; n]` where `t` is a valid value of
the type (or `mem::uninitialized`) and `n` is an expression whose result is of type `usize`. Type ascription must be used
to disambiguate cases where the type could either be `[T]` or `[T; n]` for some value of `n`.

The AST for the unsized array will be simply `syntax::ast::ItemKind::Repeat(..)`, but removing the assumption that the
second expression is a constant value. The same applies to `rustc::hir::Expr_::Repeat(..)`.

Type inference must apply the sized type unless otherwise ascribed. We should implement traits like `IntoIterator` for
unsized arrays, which may allow us to improve the ergonomics of arrays in general.

Translating the MIR to LLVM bytecode will produce the corresponding `alloca` operation with the given type and number
expression. It will also require alignment inherent to the type (which is done via a third argument).

Because LLVM currently lacks the ability to insert stack probes, the safety of this feature cannot be guaranteed. It is
thus advisable to keep this feature unstable until Rust has a working stack probe implementation.

# How we teach this
[teaching]: #how-we-teach-this

We need to extend the book to cover the distinction between sized and unsized arrays and especially the cases where
type ascription is required. Having good error messages in case of type error around the sizedness of arrays will also
help people to learn the correct use of the feature.

While stack probes remain unimplemented on some platforms, the documentation for this feature should warn of possible
dire consequences of stack overflow.

# Drawbacks
[drawbacks]: #drawbacks

- Even more stack usage means the dreaded stack limit will probably be reached even sooner. Overflowing the stack space
leads to segfaults at best and undefined behavior at worst (at least until the aforementioned stack probes are in
place). On unices, the stack can usually be extended at runtime, whereas on Windows main thread stack size is set at
link time (default to 1MB). The `thread::Builder` API has a method to set the stack size for spawned threads, however.

- With this functionality, trying to statically reason about stack usage, even in an approximate way, gains a new
degree of complexity, as maximum stack depth now depends not only on control flow alone, which can sometimes be
predictable, but also on arbitrary computations. It certainly won't be allowed in MISRA Rust, if such a thing ever
happens to come into existence.

- Adding this will increase implementation complexity and require support from possible alternative implementations /
backends (e.g. MIRI, Cretonne, WebASM). However, as all of them have C frontend support, they'll need to implement such
a feature anyway.

# Alternatives
[alternatives]: #alternatives

- Do nothing. Rust works well without it (there's the issue mentioned in the "Motivation" section though). `SmallVec`s
work well enough and have the added benefit of limiting stack usage. Except, no, they turn into hideous assembly that
makes you wonder if using a `Vec` wouldn't have been the better option.

- make the result's lifetime function-scope bound (which is what C's `alloca()` does). This is mingling two concerns
together that should be handled separately. A `'fn` lifetime will be however suggested in a sibling RFC.

- use a special macro or function to initialize the arrays. Both seem like hacks compared to the suggested syntax.

- mark the use of unsized arrays as `unsafe` regardless of values given due to the potential stack overflowing problem.
The author of this RFC does not deem this necessary if the feature gate is documented with a stern warning.

- allow for some type inference with regards to sizedness. This is likely to lead to surprises when some value ends up
unsized when a sized one was expected.

- Copy the design from C `alloca()`, possibly wrapping it later. This doesn't work in Rust because the returned
slice could leave the scope, giving rise to unsoundness.

- Use escape analysis to determine which allocations could be moved to the stack. This could potentially benefit even
more programs, because they would benefit from increased allocation speed without the need for change. The deal-breaker
here is that we would also lose control to avoid the listed drawback, making programs crash without recourse. Also the
compiler would become somewhat more complex (though a simple incomplete escape analysis implementation is already in
[clippy](https://github.com/Manishearth/rust-clippy).

# Unresolved questions
[unresolved]: #unresolved-questions

- does the MIR need to distinguish between arrays of statically-known size and unsized arrays (apart from the type
information)?
