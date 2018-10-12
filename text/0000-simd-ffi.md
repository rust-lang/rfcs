- Feature Name: `simd_ffi`
- Start Date: 2018-10-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC allows using SIMD types in C FFI.

# Motivation
[motivation]: #motivation

The architecture-specific SIMD types provided in [`core::arch`] cannot currently
be used in C FFI. That is, Rust programs cannot interface with C libraries that
use these in their APIs.

One notable example would be calling into vectorized [`libm`] implementations
like [`sleef`], [`libmvec`], or Intel's [`SVML`]. The [`packed_simd`] crate
relies on C FFI with these fundamental libraries to offer competitive
performance.

[`core::arch`]: https://doc.rust-lang.org/stable/core/arch/index.html
[`libm`]: https://sourceware.org/glibc/wiki/libm
[`sleef`]: https://sleef.org/
[`libmvec`]: https://sourceware.org/glibc/wiki/libm
[`SVML`]: https://software.intel.com/en-us/node/524289
[`packed_simd`]: https://github.com/rust-lang-nursery/packed_simd

## Why is using SIMD vectors in C FFI currently disallowed?

Consider the following example
([playground](https://play.rust-lang.org/?gist=b8cfb63bb4e7fb00bb293f6e27061c52&version=nightly&mode=debug&edition=2015)):

```rust
extern "C" fn foo(x: __m256);

fn main() {
    unsafe { 
        union U { v: __m256, a: [u64; 4] }
        foo(U { a: [0; 4] }.v);
    }
}
```

In this example, a 256-bit wide vector type, `__m256`, is passed to an `extern
"C"` function via C FFI. Is the behavior of passing `__m256` to the C function
defined?

That depends on both the platform and how the Rust program was compiled!

First, let's make the platform concrete and assume that it follows the [x64 SysV
ABI][sysv_abi] which states:

> **3.2.1 Registers and the Stack Frame**
>
> Intel AVX (Advanced Vector Extensions) provides 16 256-bit wide AVX registers
> (`%ymm0` - `%ymm15`). The lower 128-bits of `%ymm0` - `%ymm15` are aliased to
> the respective 128b-bit SSE registers (`%xmm0` - `%xmm15`). For purposes of
> parameter passing and function return, `%xmmN` and `%ymmN` refer to the same
> register. Only one of them can be used at the same time.
> 
> **3.2.3 Parameter Passing**
>
> **SSE** The class consists of types that fit into a vector register.
>
> **SSEUP** The class consists of types that fit into a vector register and can
> be passed and returned in the upper bytes of it.

[sysv_abi]: https://www.uclibc.org/docs/psABI-x86_64.pdf

Second, in `C`, the `__m256` type is only available if the current translation
unit is being compiled with `AVX` enabled.

Back to the example: `__m256` is a 256-bit wide vector type, that is, wider than
128-bit, but it can be passed through a vector register using the lower and
upper 128-bits of a 256-bit wide register, and in C, if `__m256` can be used,
these registers are always available.

That is, the C ABI requires two things: 

* that Rust passes `__m256` via a 256-bit wide register
* that `foo` has the `#[target_feature(enable = "avx")]` attribute !

And this is where things went wrong: in Rust, `__m256` is always available
independently of whether `AVX` is available or not<sup>[1](#layout_unspecified)</sup>, 
but we haven't specified how we are actually compiling our Rust program above:

* if we compile it with `AVX` globally enabled, e.g., via `-C
  target-feature=+avx`, then the behavior of calling `foo` is defined because
  `__m256` will be passed to C in a single 256-bit wide register, which is what
  the C ABI requires.
  
* if we compile our program without `AVX` enabled, then the Rust program cannot
  use 256-bit wide registers because they are not available, so independently of
  how `__m256` will be passed to C, it won't be passed in a 256-bit wide
  register, and the behavior is undefined because of an ABI mismatch.

<a name="layout_unspecified">1</a>: its layout is currently unspecified but that
is not relevant for this issue since if 256-bit registers are not available they
cannot be used anyways, which is what matters here.

So, first of all, is this a big deal? 

Currently, one cannot use SIMD types in C FFI in stable Rust, so technically,
nothing is broken yet, and no, this is not a big deal: stable Rust is still
safe! However, we would like to be able to call C FFI functions without
introducing undefined behavior independently of which `-C target-features` are
passed, so the example code shown above has to be rejected by the compiler.

Second, you might be wondering: why is `__m256` available even if `AVX` is not
available? That's a good question. We want to use `__m256` in some parts of
Rust's programs even if `AVX` is not globally enabled, and currently we don't
have great infrastructure for conditionally allowing it in some parts of the
program and not others.

Ideally, one should only be able to use `__m256` and operations on it if `AVX`
is available. Which leads to how can we fix this ? 

The most trivial solution would be to just always require
`#[target_feature(enable = X)]` in C FFI functions using SIMD types, where
"unblocking" the use of each type requires one or two particular feature to be
enabled, e.g., `avx` or `avx2` in the case of `__m256`.

That is, the compiler would reject the example above with an error: 

```
error[E1337]: `__m256` on C FFI requires `#[target_feature(enable = "avx")]`
 --> src/main.rs:7:15
  |
7 |     fn foo(x: __m25a6) -> __m256;
  |               ^^^^^^^
```

And the following program would always have defined behavior
([playground](https://play.rust-lang.org/?gist=db651d09441fd16172a5c94711b2ab97&version=nightly&mode=debug&edition=2015)):

```rust
#[target_feature(enable = "avx")]
extern "C" fn foo(x: __m256) -> __m256;

fn main() {
    unsafe { 
        union U { v: __m256, a: [u64; 4] }
        if is_x86_feature_detected!("avx") {
            foo(U { a: [0; 4] }.v);
        }
    }
}
```

Note here that:

* `extern "C" foo` is compiled with `AVX` enabled, so `foo` takes an `__m256`
  like the C ABI expects
* the call to `foo` is guarded with an `is_x86_feature_detected`, that is, `foo`
  will only be called if `AVX` is available at run-time
* if the Rust binary is compiled without `AVX`, Rust will insert shims in the
  call to `foo` to pass it as a 256-bit register. Rust already does this, and
  `#[target_feature]` is what allows it to do it. Without the
  `#[target_feature]` annotation, Rust does not know that C expects this. 

# Guide-level and reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Architecture-specific vector types require `#[target_feature]`s to be FFI safe.
That is, they are only safely usable as part of the signature of `extern`
functions if the function has certain `#[target_feature]`s enabled.

Which `#[target_feature]`s must be enabled depends on the vector types being
used.

For the stable architecture-specific vector types the following target features
must be enabled:

* `x86`/`x86_64`:
    * `__m128`, `__m128i`, `__m128d`: `"sse"`
    * `__m256`, `__m256i`, `__m256d`: `"avx"`


Future stabilizations of architecture-specific vector types must specify the
target features required to use them in `extern` functions.

# Drawbacks
[drawbacks]: #drawbacks

TBD.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

This is an adhoc solution to the problem. This RFC does not explore more general
mechanisms of dealing with, or abstracting over, these kind of problems.

## Future architecture-specific vector types

In the future, we might want to stabilize some of the following vector types.
This section explores which target features would they require:

* `x86`/`x86_64`:
  * `__m64`: `mmx`
  * `__m512`, `__m512i`, `__m512f`: "avx512f"
* `arm`: `neon`
* `aarch64`: `neon`
* `ppc64`: `altivec` / `vsx`
* `wasm32`: `simd128`

## Require the feature to be enabled globally for the binary

Instead of using `#[target_feature]` we could allow vector types on C FFI only
behind `#[cfg(target_feature)]`, e.g., via something like the portability check. 

This would not allow calling C FFI functions with vector types conditionally on,
e.g., run-time feature detection.

# Prior art
[prior-art]: #prior-art

In C, the architecture specific vector types are only available if the required
target features are enabled at compile-time.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Should it be possible to use, e.g., `__m128` on C FFI when the `avx` feature
  is enabled? Does that change the calling convention and make doing so unsafe ?
  We could extern this RFC to also require that to use certain types certain
  features must be disabled.
