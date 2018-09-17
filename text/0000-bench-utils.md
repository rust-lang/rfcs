- Feature Name: black_box
- Start Date: 2018-03-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC adds one function, `core::hint::black_box`, which is a hint to the
optimizer to disable certain compiler optimizations. 

# Motivation
[motivation]: #motivation

A tool for preventing compiler optimizations is widely useful. One application
is writing synthetic benchmarks, where, due to the constrained nature of the
benchmark, the compiler is able to perform optimizations that wouldn't otherwise
trigger in practice. Another application is writing constant time code, where it
is undesirable for the compiler to optimize certain operations depending on the
context in which they are executed.

The implementation of this function is backend-specific and currently requires
inline assembly. No viable alternative is available in stable Rust.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation


## `hint::black_box`

The function:

```rust
/// An _unknown_ function that returns `x`.
pub fn black_box<T>(x: T) -> T;
```

returns `x` and is an _unknown_ function, that is, a function that the compiler
cannot make any assumptions about. It can use `x` in any possible valid way that
Rust code is allowed to without introducing undefined behavior in the calling
code. This requires the compiler to be maximally pessimistic in terms of
optimizations, but the compiler is still allowed to optimize the expression
generating `x`. While the compiler must assume that `black_box` performs any
legal mutation of `x`, the programmer can rely on `black_box` not actually
having any effect (other than inhibiting optimizations).

For example ([`rust.godbolt.org`](https://godbolt.org/g/YP2GCJ)):

```rust
fn foo(x: i32) -> i32{ 
  hint::black_box(2 + x);
  3
}
let a = foo(2);
```

In the call to `foo(2)` the compiler is allowed to simplify the expression `2 +
x` down to `4`, but `4` must be materialized, for example, by storing it into
memory, a register, etc. because `black_box` could try to read it even though
`4` is not read by anything afterwards.

### Benchmarking `Vec::push`

The `hint::black_box` is useful for producing synthetic benchmarks that more
accurately represent the behavior of a real application. In the following
snippet, the function `bench` executes `Vec::push` 4 times in a loop:

```rust
fn push_cap(v: &mut Vec<i32>) {
    for i in 0..4 {
      v.push(i);
    }
}

pub fn bench_push() -> Duration { 
    let mut v = Vec::with_capacity(4);
    let now = Instant::now();
    push_cap(&mut v);
    now.elapsed()
}
```

Here, we allocate the `Vec`, push into it without growing its capacity, and drop
it, without ever using it for anything. If we look at the assembly
(https://rust.godbolt.org/z/wDckJF):


```asm
example::bench_push:
  sub rsp, 24
  call std::time::Instant::now@PLT
  mov qword ptr [rsp + 8], rax
  mov qword ptr [rsp + 16], rdx
  lea rdi, [rsp + 8]
  call std::time::Instant::elapsed@PLT
  add rsp, 24
  ret
```

it is pretty amazing: LLVM has actually managed to completely optimize the `Vec`
allocation and call to `push_cap` away! In our real application, we would
probably use the vector for something, preventing all of these optimizations
from triggering, but in this synthetic benchmark LLVM optimizations are
producing a benchmark that won't tell us anything about the cost of `Vec::push`.

We can use `hint::black_box` to create a more realistic synthetic benchmark
since the compiler has to assume that `black_box` observes and mutates its
argument it cannot optimize the whole benchmark away (https://rust.godbolt.org/z/CeXmxN):

```rust
fn push_cap(v: &mut Vec<i32>) {
    for i in 0..4 {
        black_box(v.as_ptr());
        v.push(black_box(i));
        black_box(v.as_ptr());
    }
}
```

that prevents LLVM from assuming anything about the vector across the calls to
`Vec::push`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The 

```
mod core::hint {
    /// An _unknown_ unsafe function that returns `x`.
    pub fn black_box<T>(x: T) -> T;
}
```

is an _unknown_ function that can perform any valid operation on `x` that Rust
is allowed to perform without introducing undefined behavior in the calling
code. You can rely on `black_box` being a `NOP` just returning `x`, but the
compiler will optimize under the pessimistic assumption that `black_box` might
do anything with the data it got.

# Drawbacks
[drawbacks]: #drawbacks

TBD.

# Rationale and alternatives
[alternatives]: #alternatives

Further rationale influencing this design is available in
https://github.com/nikomatsakis/rust-memory-model/issues/45

## `clobber`

A previous version of this RFC also provided a `clobber` function:

```rust
/// Flushes all pending writes to memory. 
pub fn clobber() -> ();
```

In https://github.com/nikomatsakis/rust-memory-model/issues/45 it was realized
that such a function cannot work properly within Rust's memory model.

## `value_fence` / `evaluate_and_drop`

An alternative design was proposed during the discussion on
[rust-lang/rfcs/issues/1484](https://github.com/rust-lang/rfcs/issues/1484), in
which the following two functions are provided instead:

```rust
#[inline(always)]
pub fn value_fence<T>(x: T) -> T {
    let y = unsafe { (&x as *const T).read_volatile() };
    std::hint::forget(x);
    y
}

#[inline(always)]
pub fn evaluate_and_drop<T>(x: T) {
    unsafe {
        let mut y = std::hint::uninitialized();
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

Similar functionality is provided in the [`Google
Benchmark`](https://github.com/google/benchmark) C++ library: are called
[`DoNotOptimize`](https://github.com/google/benchmark/blob/61497236ddc0d797a47ef612831fb6ab34dc5c9d/include/benchmark/benchmark.h#L306)
(`black_box`) and
[`ClobberMemory`](https://github.com/google/benchmark/blob/61497236ddc0d797a47ef612831fb6ab34dc5c9d/include/benchmark/benchmark.h#L317).
The `black_box` function with slightly different semantics is provided by the
`test` crate:
[`test::black_box`](https://github.com/rust-lang/rust/blob/master/src/libtest/lib.rs#L1551).

# Unresolved questions
[unresolved]: #unresolved-questions

TBD.
