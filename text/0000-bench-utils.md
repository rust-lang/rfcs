- Feature Name: black_box-and-clobber
- Start Date: 2018-03-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC adds two functions to `core::mem`: `black_box` and `clobber`, which are
mainly useful for writing benchmarks.

# Motivation
[motivation]: #motivation

The `black_box` and `clobber` functions are useful for writing synthetic
benchmarks where, due to the constrained nature of the benchmark, the compiler
is able to perform optimizations that wouldn't otherwise trigger in practice.

The implementation of these functions is backend-specific and requires inline
assembly. Such that if the standard library does not provide them, the users are
required to use brittle workarounds on nightly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation


## `mem::black_box`

The function:

```rust
pub fn black_box<T>(x: T) -> T;
```

prevents the value `x` from being optimized away and flushes pending reads/writes
to memory. It does not prevent optimizations on the expression generating the
value `x` nor on the return value of the function. For
example ([`rust.godbolt.org`](https://godbolt.org/g/YP2GCJ)):

```rust
fn foo(x: i32) -> i32{ 
  mem::black_box(2 + x);
  3
}
let a = foo(2);
```

Here, the compiler can simplify the expression `2 + x` into `2 + 2` and then
`4`, but it is not allowed to discard `4`. Instead, it must store `4` into a
register even though it is not used by anything afterwards.

## `mem::clobber`

The function

```rust
pub fn clobber() -> ();
```
 	 
flushes all pending writes to memory. Memory managed by block scope objects must
be "escaped" with `black_box` . 

Using `mem::{black_box, clobber}` we can benchmark `Vec::push` as follows:

```rust
fn bench_vec_push_back(bench: Bencher) -> BenchResult {
    let n = /* large enough number */;
    let mut v = Vec::with_capacity(n);
    bench.iter(|| {
        // Escape the vector pointer:
        mem::black_box(v.as_ptr());
        v.push(42_u8);
        // Flush the write of 42 back to memory:
        mem::clobber();
    })
}
```

To measure the cost of `Vec::push`, we pre-allocate the `Vec` to avoid
re-allocating memory during the iteration. Since we are allocating a vector,
writing values to it, and dropping it, LLVM is actually able of optimize code
like this away ([`rust.godbolt.org`](https://godbolt.org/g/QMs77J)). 

To make this a suitable benchmark, we use `mem::clobber()` to force LLVM to
write `42` back to memory. Note, however, that if we try this LLVM still manages
to optimize our benchmark away ([`rust.godbolt.org`](https://godbolt.org/g/r9K2Bk))!

The problem is that the memory of our vector is managed by an object in block
scope. That is, since we haven't shared this memory with anything, no other code
in our program can have a pointer to it, so LLVM does not need to schedule any
writes to this memory, and there are no pending memory writes to flush! 

What we must do is tell LLVM that something might also have a pointer to this
memory, and this is what we use `mem::black_box` for in this case
([`rust.godbolt.or`](https://godbolt.org/g/3wBxay)).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

* `mem::black_box(x)`: flushes all pending writes/read to memory and prevents
  `x` from being optimized away while still allowing optimizations on the
  expression that generates `x`.
* `mem::clobber`: flushes all pending writes to memory.

# Drawbacks
[drawbacks]: #drawbacks

TBD.

# Rationale and alternatives
[alternatives]: #alternatives

An alternative design was proposed during the discussion on
[rust-lang/rfcs/issues/1484](https://github.com/rust-lang/rfcs/issues/1484), in
which the following two functions are provided instead:

```rust
#[inline(always)]
pub fn value_fence<T>(x: T) -> T {
    let y = unsafe { (&x as *const T).read_volatile() };
    std::mem::forget(x);
    y
}

#[inline(always)]
pub fn evaluate_and_drop<T>(x: T) {
    unsafe {
        let mut y = std::mem::uninitialized();
        std::ptr::write_volatile(&mut y as *mut T, x);
        drop(y); // not necessary but for clarity
    }
}
```

This approach is not pursued in this RFC because these two functions:

* add overhead ([`rust.godbolt.org`](https://godbolt.org/g/aCpPfg)): `volatile`
  reads and stores aren't no ops, but the proposed `black_box` and `clobber`
  functions are.
* are implementable on stable Rust: while we could add them to `std` they do not
  necessarily need to be there.

# Prior art
[prior-art]: #prior-art

These two exact functions are provided in the [`Google
Benchmark`](https://github.com/google/benchmark) C++ library: are called
[`DoNotOptimize`](https://github.com/google/benchmark/blob/61497236ddc0d797a47ef612831fb6ab34dc5c9d/include/benchmark/benchmark.h#L306)
(`black_box`) and
[`ClobberMemory`](https://github.com/google/benchmark/blob/61497236ddc0d797a47ef612831fb6ab34dc5c9d/include/benchmark/benchmark.h#L317).
The `black_box` function with slightly different semantics is provided by the `test` crate:
[`test::black_box`](https://github.com/rust-lang/rust/blob/master/src/libtest/lib.rs#L1551).

# Unresolved questions
[unresolved]: #unresolved-questions

TBD.
