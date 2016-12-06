- Feature Name: alloca
- Start Date: 2016-12-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Add a builtin `fn core::mem::reserve<'a, T>(elements: usize) -> StackSlice<'a, T>` that reserves space for the given
number of elements on the stack and returns a `StackSlice<'a, T>` to it which derefs to `&'a [T]`.

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

The standard library function can simply `panic!(..)` within the `reserve(_)` method, as it will be replaced when
translating to MIR. The `StackSlice` type can be implemented as follows:

```Rust
/// A slice of data on the stack
pub struct StackSlice<'a, T: 'a> {
    slice: &'a [T],
}

impl<'a, T: 'a> Deref for StackSlice<'a, T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        return self.slice;
    }
}
```

`StackSlice`'s embedded lifetime ensures that the stack allocation may never leave its scope. Thus the borrow checker
can uphold the contract that LLVM's `alloca` requires.

MIR Level: We need a way to represent the dynamic stack `alloca`tion with both the number of elements and the concrete
type of elements. Then while building the MIR, we need to replace the `Calls` from HIR with it.

Low-level: LLVM has the `alloca` instruction to allocate memory on the stack. We simply need to extend trans to emit it
with a dynamic `<NumElements>` argument when encountering the aforementioned MIR.

With a LLVM extension to un-allocate the stack slice we could even restrict the stack space reservation to the lifetime
of the allocated value, thus increasing locality over C code that uses alloca (which so far is suboptimally implemented
by some compilers, especially with regards to inlining).

# How to teach this

Add the following documentation to libcore:

```
*** WARNING *** stay away from this feature unless you absolutely need it.
Using it will destroy your ability to statically reason about stack size.

Apart from that, this works much like an unboxed array, except the size is
determined at runtime. Since the memory resides on the stack, be careful
not to exceed the stack limit (which depends on your operating system),
otherwise the resulting stack overflow will at best kill your program. You
have been warned.

Valid uses for this is mostly within embedded system without heap allocation.
```

Also add an example (perhaps a sort algorithm that uses some scratch space that will be heap-allocacted with `std` and
stack-allocated with `#[no_std]` (noting that the function would not be available on no-std systems at all were it not
for this feature).

Do not `pub use` it from `std::mem` to drive the point home.

# Drawbacks
[drawbacks]: #drawbacks

- Even more stack usage means the dreaded stack limit will probably be reached even sooner. Overflowing the stack space
leads to segfaults at best and undefined behavior at worst. On unices, the stack can usually be extended at runtime,
whereas on Windows stack size is set at link time (default to 1MB).

- Adding this will increase implementation complexity and require support from possible alternative implementations /
backends (e.g. Cretonne, WebASM).

# Alternatives
[alternatives]: #alternatives

- Do nothing. Rust works well without it (there's the issue mentioned in the "Motivation" section though). `SmallVec`s
work well enough and have the added benefit of limiting stack usage.

-  `mem::with_alloc<T, F: Fn([T]) -> U>(elems: usize, code: F) -> U` This has the benefit of reducing API surface, and
introducing rightwards drift, which makes it more unlikely to be used too much. However, it also needs to be
monomorphized for each function (instead of only for each target type), which will increase compile times.

- dynamically sized arrays are a potential solution, however, those would need to have a numerical type that is only
fully known at runtime, requiring complex type system gymnastics.

- use a macro instead of a function (analogous to `print!(..)`), which could insert the LLVM alloca builtin.

- mark the function as `unsafe` due to the potential stack overflowing problem.

- Copy the design from C `fn alloca()`, possibly wrapping it later.

- Use escape analysis to determine which allocations could be moved to the stack. This could potentially benefit even
more programs, because they would benefit from increased allocation speed without the need for change. The deal-breaker
here is that we would also lose control to avoid the listed drawback, making programs crash without recourse. Also the
compiler would become somewhat more complex (though a simple incomplete escape analysis implementation is already in
[clippy](https://github.com/Manishearth/rust-clippy).

# Unresolved questions
[unresolved]: #unresolved-questions

- Could we return the slice directly (reducing visible complexity)?

- Bikeshedding: Can we find a better name?
