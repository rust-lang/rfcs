- Feature Name: `bench_black_box`
- Start Date: 2018-03-12
- RFC PR: [rust-lang/rfcs#2360](https://github.com/rust-lang/rfcs/pull/2360)
- Rust Issue: [rust-lang/rust#64102](https://github.com/rust-lang/rust/issues/64102)

## Summary
[summary]: #summary

This RFC adds `core::hint::bench_black_box` (see [black box]), an identity function
that hints the compiler to be maximally pessimistic in terms of the assumptions
about what `bench_black_box` could do.

[black box]: https://en.wikipedia.org/wiki/black_box

## Motivation
[motivation]: #motivation

Due to the constrained nature of synthetic benchmarks, the compiler is often
able to perform optimizations that wouldn't otherwise trigger in practice, like
completely removing a benchmark if it has no side-effects. 

Currently, stable Rust users need to introduce expensive operations into their
programs to prevent these optimizations. Examples thereof are volatile loads and
stores, or calling unknown functions via C FFI. These operations incur overheads
that often would not be present in the application the synthetic benchmark is
trying to model.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

### `hint::bench_black_box`

The hint:

```rust
pub fn bench_black_box<T>(x: T) -> T;
```

behaves like the [identity function][identity_fn]: it just returns `x` and has
no effects. However, Rust implementations are _encouraged_ to assume that
`bench_black_box` can use `x` in any possible valid way that Rust code is allowed to
without introducing undefined behavior in the calling code. That is,
implementations are encouraged to be maximally pessimistic in terms of
optimizations.

This property makes `bench_black_box` useful for writing code in which certain
optimizations are not desired, but too unreliable when disabling these
optimizations is required for correctness.

#### Example 1 - basics 

Example 1 ([`rust.godbolt.org`](https://godbolt.org/g/YP2GCJ)):

```rust
fn foo(x: i32) -> i32 { 
  hint::bench_black_box(2 + x);
  3
}
let a = foo(2);
```

In this example, the compiler may simplify the expression `2 + x` down to `4`.
However, even though `4` is not read by anything afterwards, it must be computed
and materialized, for example, by storing it into memory, a register, etc.
because the current Rust implementation assumes that `bench_black_box` could try to
read it.

#### Example 2 - benchmarking `Vec::push`

The `hint::bench_black_box` is useful for producing synthetic benchmarks that more
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
        bench_black_box(v.as_ptr());
        v.push(bench_black_box(i));
        bench_black_box(v.as_ptr());
    }
}
```

Inspecting the machine code reveals that, for this particular Rust
implementation, `bench_black_box` successfully prevents LLVM from performing the
optimization that removes the `Vec::push` calls that we wanted to measure.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The 

```rust
mod core::hint {
    /// Identity function that disables optimizations.
    pub fn bench_black_box<T>(x: T) -> T;
}
```

is a `NOP` that returns `x`, that is, its operational semantics are equivalent
to the [identity function][identity_fn].


Implementations are encouraged, _but not required_, to treat `bench_black_box` as an
_unknown_ function that can perform any valid operation on `x` that Rust is
allowed to perform without introducing undefined behavior in the calling code.
That is, to optimize `bench_black_box` under the pessimistic assumption that it might
do anything with the data it got, even though it actually does nothing.

[identity_fn]: https://doc.rust-lang.org/nightly/std/convert/fn.identity.html

## Drawbacks
[drawbacks]: #drawbacks

Slightly increases the surface complexity of `libcore`.

## Rationale and alternatives
[alternatives]: #alternatives

Further rationale influencing this design is available in
https://github.com/nikomatsakis/rust-memory-model/issues/45

### `clobber`

A previous version of this RFC also provided a `clobber` function:

```rust
/// Flushes all pending writes to memory. 
pub fn clobber() -> ();
```

In https://github.com/nikomatsakis/rust-memory-model/issues/45 it was realized
that such a function cannot work properly within Rust's memory model.

### `value_fence` / `evaluate_and_drop`

An alternative design was proposed during the discussion on
[rust-lang/rfcs/issues/1484](https://github.com/rust-lang/rfcs/issues/1484), in
which the following two functions are provided instead:

```rust
#[inline(always)]
pub fn value_fence<T>(x: T) -> T {
    let y = unsafe { (&x as *T).read_volatile() };
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
  reads and stores aren't no ops, but the proposed `bench_black_box` and `clobber`
  functions are.
* are implementable on stable Rust: while we could add them to `std` they do not
  necessarily need to be there.

### `bench_input` / `bench_outpu`

@eddyb proposed
[here](https://github.com/rust-lang/rfcs/pull/2360#issuecomment-463594450) (and
the discussion that followed) to add two other hints instead:

* `bench_input`: `fn(T) -> T` (identity-like) may prevent some optimizations
  from seeing through the valid `T` value, more specifically, things like
  const/load-folding and range-analysis miri would still check the argument, and
  so it couldn't be e.g. uninitialized the argument computation can be
  optimized-out (unlike `bench_output`) mostly implementable today with the same
  strategy as `black_box`.

* `bench_output`: `fn(T) -> ()` (drop-like) may prevent some optimizations from
  optimizing out the computation of its argument the argument is not treated as
  "escaping into unknown code", i.e., you can't implement `bench_output(x)` as
  `{ bench_input(&mut x); x }`. What that would likely prevent is placing `x`
  into a register instead of memory, but optimizations might still see the old
  value of `x`, as if it couldn't have been mutated potentially implementable
  like `black_box` but `readonly`/`readnone` in LLVM.

From the RFC discussion there was consensus that we might want to add these
benchmarking hints in the future as well because their are easier to specify and
provide stronger guarantees than `bench_black_box`.

Right now, however, it is unclear whether these two hints can be implemented
strictly in LLVM. The comment thread shows that the best we can actually do
ends up implementing both of these as `bench_black_box` with the same effects.

Without a strict implementation, it is unclear which value these two intrinsics
would add, and more importantly, since their difference in semantics cannot be
shown, it is also unclear how we could teach users to use them correctly.

If we ever able to implement these correctly, we might want to consider
deprecating `bench_black_box` at that point, but whether it will be worth
deprecating is not clear either.

## Prior art
[prior-art]: #prior-art

Similar functionality is provided in the [`Google
Benchmark`](https://github.com/google/benchmark) C++ library: are called
[`DoNotOptimize`](https://github.com/google/benchmark/blob/61497236ddc0d797a47ef612831fb6ab34dc5c9d/include/benchmark/benchmark.h#L306)
(`bench_black_box`) and
[`ClobberMemory`](https://github.com/google/benchmark/blob/61497236ddc0d797a47ef612831fb6ab34dc5c9d/include/benchmark/benchmark.h#L317).
The `black_box` function with slightly different semantics is provided by the
`test` crate:
[`test::black_box`](https://github.com/rust-lang/rust/blob/master/src/libtest/lib.rs#L1551).

## Unresolved questions
[unresolved]: #unresolved-questions

* `const fn`: it is unclear whether `bench_black_box` should be a `const fn`. If it
  were, that would hint that it cannot have any side-effects, or that it cannot
  do anything that `const fn`s cannot do. 

* Naming: during the RFC discussion it was unclear whether `black_box` is the
  right name for this primitive but we settled on `bench_black_box` for the time
  being. We should resolve the naming before stabilization.
  
  Also, we might want to add other benchmarking hints in the future, like
  `bench_input` and `bench_output`, so we might want to put all of this
  into a `bench` sub-module within the `core::hint` module. That might
  be a good place to explain how the benchmarking hints should be used 
  holistically.
  
  Some arguments in favor or against using "black box" are that:
     * pro: [black box] is a common term in computer programming, that conveys
       that nothing can be assumed about it except for its inputs and outputs.
       con: [black box] often hints that the function has no side-effects, but
       this is not something that can be assumed about this API.
     * con: `_box` has nothing to do with `Box` or `box`-syntax, which might be confusing
 
  Alternative names suggested: `pessimize`, `unoptimize`, `unprocessed`, `unknown`,
  `do_not_optimize` (Google Benchmark).
