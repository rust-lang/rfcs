- Feature Name: alloca
- Start Date: 2016-12-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a builtin `alloca!(type, number_of_elements)` macro that reserves space for the given number of elements of type
`T` on the stack and returns a slice over the reserved memory. The memories' lifetime is artifically restricted to the
current function's scope, so the borrow checker can ensure that the memory is no longer used when the method returns.

# Motivation
[motivation]: #motivation

Some algorithms (e.g. sorting, regular expression search) need a one-time backing store for a number of elements only
known at runtime. Reserving space on the heap always takes a performance hit, and the resulting deallocation can
increase memory fragmentation, possibly slightly degrading allocation performance further down the road.

If Rust included this zero-cost abstraction, more of these algorithms could run at full speed – and would be available
on systems without an allocator, e.g. embedded, soft-real-time systems. The option of using a fixed slice up to a
certain size and using a heap-allocated slice otherwise (as afforded by
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

There are a few constraints we have to keep: First, we want to allow for mostly free usage of the memory, while keeping
borrows to it limited to the current function's scope – this makes it possible to use it in a loop, increasing its
usefulness. The macro should include a check in debug mode to ensure the stack limit is not exceeded. Actually, it
should arguably check this in release mode, too (which would be feasible without giving up performance using stack
probes, which have not been available from LLVM despite being hailed LLVM's preferred solution to stack overflow
problems), but writing this within Rustc would duplicate work that LLVM is poised to do anyway.

This feature would be available via a builtin macro `stack!(..)` taking any of the following arguments:

- `stack![x; <num>]` reserves an area large enough for *num* (where num is an expression evaluating to a `usize`) `x`
instances on the stack, fills it with `x` and returns a slice to it; this requires that `x` be of a `Copy`able type

- `stack![x, y, z, ..]` (analogous to `vec![..]`). This is not actually needed as current arrays do mostly the same
thing, but will likely reduce the number of frustrated users

- `stack![for <iter>]` (where iter is an expression that returns an `std::iter::ExactSizeIterator`)

- `unsafe { stack![Ty * num] }` reserves an uninitialized area large enough for *num* elements of the given type `Ty`,
giving people seeking performance a cheap dynamically sized scratch space for their algorithms

All variants return a slice to the reserved stack space which will live until the end of the current function (same as
C's `alloca(..)` builtin). Because this is a compiler-builtin, we can make use of the type of the values in determining
the type of the expression, so we don't need to restate the type (unless it's not available, as in the unsafe version).

The macro will expand to a newly introduced `DynArray{ ty: Ty, num: Expr }` `rustc::hir::ExprKind` variant (plus some
exertions to put the values in the reserved space, depending on variant) that will be mapped to an `alloca` operation
in MIR and LLVM IR. The type of the expression will be rigged in HIR to have a lifetime until the function body ends.

Te iterator version will return a shorter slice than reserved if the iterator returns `None` early. SHould the iterator
panic, the macro will `forget(_)` all values inserted so far and re-raise the panic.

If the macro is invoked with unsuitable input (e.g. `stack![Ty]`, `stack![]`, etc., it should at least report an error
outlining the valid modes of operation. If we want to improve the ergonomics, we could try to guess which one the user
has actually attempted and offer a suggestion to that effect.

Translating the MIR to LLVM bytecode will produce the corresponding `alloca` operation with the given type and number
expression.

# How we teach this
[teaching]: #how-we-teach-this

The doc comments for the macro should contain text like the following:


```Rust
/// *** WARNING *** stay away from this feature unless you absolutely need it.
/// Using it will destroy your ability to statically reason about stack size.
/// 
/// Apart from that, this works much like an unboxed array, except the size is
/// determined at runtime. Since the memory resides on the stack, be careful
/// not to exceed the stack limit (which depends on your operating system),
/// otherwise the resulting stack overflow will at best kill your program. You
/// have been warned.
/// 
/// Valid uses for this is mostly within embedded system without heap allocation
/// to claim some scratch space for algorithms, e.g. in sorting, traversal, etc.
///
/// This macro has four modes of operation:
/// ..
```

The documentation should be sufficient to explain the use of the feature. Also the book should be extended with
examples of all modes of operation. Once stabilized, the release log should advertise the new feature. Blogs will rave
about it, trumpets will chime, and the world will be a little brighter than before.

# Drawbacks
[drawbacks]: #drawbacks

- Even more stack usage means the dreaded stack limit will probably be reached even sooner. Overflowing the stack space
leads to segfaults at best and undefined behavior at worst. On unices, the stack can usually be extended at runtime,
whereas on Windows stack size is set at link time (default to 1MB).

- With this functionality, we lose the ability to statically reason about stack space. Worse, since it can be used to
reserve space arbitrarily, it can blow past the guard page that operating systems usually employ to secure programs
against stack overflow. Hilarity ensues. However, it can be argued that static stack reservations (e.g.
`let _ = [0u64; 9999999999];` already suffices to do this. Perhaps someone should write a lint against this. It
certainly won't be allowed in MISRA Rust, if such a thing ever happens to come into existence.

- Adding this will increase implementation complexity and require support from possible alternative implementations /
backends (e.g. Cretonne, WebASM).

# Alternatives
[alternatives]: #alternatives

- Do nothing. Rust works well without it (there's the issue mentioned in the "Motivation" section though). `SmallVec`s
work well enough and have the added benefit of limiting stack usage. Except, no, they turn into hideous assembly that
makes you wonder if using a `Vec` wouldn't have been the better option.

- dynamically sized arrays are a potential solution, however, those would need to have a numerical type that is only
fully known at runtime, requiring complex type system gymnastics.

- use a function instead of a macro. This would be more complex for essentially no gain.

- mark the use of the macro as `unsafe` regardless of values given due to the potential stack overflowing problem.

- Copy the design from C `fn alloca()`, possibly wrapping it later. This doesn't work in Rust because the returned
slice could leave the scope, giving rise to unsoundness.

- Use escape analysis to determine which allocations could be moved to the stack. This could potentially benefit even
more programs, because they would benefit from increased allocation speed without the need for change. The deal-breaker
here is that we would also lose control to avoid the listed drawback, making programs crash without recourse. Also the
compiler would become somewhat more complex (though a simple incomplete escape analysis implementation is already in
[clippy](https://github.com/Manishearth/rust-clippy).

# Unresolved questions
[unresolved]: #unresolved-questions

- Is the feature as defined above ergonomic? Should it be?

- Bikeshedding: Can we find a better name?
