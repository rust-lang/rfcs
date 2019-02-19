- Feature Name: black_box
- Start Date: 2018-03-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC adds `core::hint::black_box`, a hint to disable certain compiler
optimizations.

# Motivation
[motivation]: #motivation

A hint to disable compiler optimizations is widely useful. One such application
is writing synthetic benchmarks where, due to the constrained nature of the
benchmark, the compiler is able to perform optimizations that wouldn't otherwise
trigger in practice.

There are currently no viable stable Rust alternatives for `black_box`. The
current nightly Rust implementations all rely on inline assembly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## `hint::black_box`

The hint:

```rust
pub fn black_box<T>(x: T) -> T;
```

behaves like the [identity function][identity_fn]: it just returns `x` and has
no effects. However, Rust implementations are _encouraged_ to assume that
`black_box` can use `x` in any possible valid way that Rust code is allowed to
without introducing undefined behavior in the calling code. That is,
implementations are encouraged to be maximally pessimistic in terms of
optimizations.

This property makes `black_box` useful for writing code in which certain
optimizations are not desired. However, disabling optimizations is not
guaranteed, which means that `black_box` is not a solution for programs that
rely on certain optimizations being disabled for correctness, like, for example,
constant time code.

### Example 1 - basics 

Example 1 ([`rust.godbolt.org`](https://godbolt.org/g/YP2GCJ)):

```rust
fn foo(x: i32) -> i32{ 
  hint::black_box(2 + x);
  3
}
let a = foo(2);
```

In this example, the compiler may simplify the expression `2 + x` down to `4`.
However, even though `4` is not read by anything afterwards, it must be computed
and materialized, for example, by storing it into memory, a register, etc.
because the current Rust implementation assumes that `black_box` could try to
read it.

### Example 2 - benchmarking `Vec::push`

The `hint::black_box` is useful for producing synthetic benchmarks that more
accurately represent the behavior of a real application. In the following
example, the function `bench` executes `Vec::push` 4 times in a loop:

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

This example allocates a `Vec`, pushes into it without growing its capacity, and
drops it, without ever using it for anything. The current Rust implementation
emits the following `x86_64` machine code (https://rust.godbolt.org/z/wDckJF):


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

LLVM is pretty amazing: it has optimized the `Vec` allocation and the calls to
`push_cap` away. In doing so, it has made our benchmark useless. It won't
measure the time it takes to perform the calls to `Vec::push` as we intended. 

In real applications, the program will use the vector for something, preventing
these optimizations. To produce a benchmark that takes that into account, we can
hint the compiler that the `Vec` is used for something
(https://rust.godbolt.org/z/CeXmxN):

```rust
fn push_cap(v: &mut Vec<i32>) {
    for i in 0..4 {
        black_box(v.as_ptr());
        v.push(black_box(i));
        black_box(v.as_ptr());
    }
}
```

Inspecting the machine code reveals that, for this particular Rust
implementation, `black_box` successfully prevents LLVM from performing the
optimization that removes the `Vec::push` calls that we wanted to measure.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The 

```rust
mod core::hint {
    /// Identity function that disables optimizations.
    pub fn black_box<T>(x: T) -> T;
}
```

is a `NOP` that returns `x`, that is, its operational semantics are equivalent
to the [identity function][identity_fn].


Implementations are encouraged, _but not required_, to treat `black_box` as an
_unknown_ function that can perform any valid operation on `x` that Rust is
allowed to perform without introducing undefined behavior in the calling code.
That is, to optimize `black_box` under the pessimistic assumption that it might
do anything with the data it got.

[identity_fn]: https://doc.rust-lang.org/nightly/std/convert/fn.identity.html

# Drawbacks
[drawbacks]: #drawbacks

Slightly increases the surface complexity of `libcore`.

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

@Centril asked whether `black_box` should be a `const fn`. The current
implementation uses inline assembly. It is unclear at this point whether
`black_box` should be a `const fn`, and if it should, how exactly would we go
about it. We do not have to resolve this issue before stabilization since we can
always make it a `const fn` later, but we should not forget about it either.
