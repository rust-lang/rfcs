- Feature Name: `patchable_function_entry`
- Start Date: 2023-12-12
- RFC PR: [rust-lang/rfcs#3543](https://github.com/rust-lang/rfcs/pull/3543)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes support for `patchable-function-entry` as present in [`clang`](https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fpatchable-function-entry) and [`gcc`](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html#index-fpatchable-function-entry). This feature is generally used to allow hotpatching and instrumentation of code.

# Motivation
[motivation]: #motivation

The Linux kernel uses `-fpatchable-function-entry` heavily, including for [`ftrace`](https://www.kernel.org/doc/html/v6.6/trace/ftrace.html) and [`FINEIBT` for x86](https://github.com/torvalds/linux/blob/26aff849438cebcd05f1a647390c4aa700d5c0f1/arch/x86/Kconfig#L2464). Today, enabling these features alongside Rust will lead to confusing or broken behavior (`ftrace` will fail to trace Rust functions when developing, `FINEIBT` will conflict with the `kcfi` sanitizer, etc.). It also uses the `clang` and `gcc` attribute `patchable_function_entry` to disable this padding on fragile functions or those used for instrumentation.

Integrating Rust code into this and other large projects which expect all native code to have these nop buffers will be made easier by allowing them to request the same treatment of native functions they get in C and C++.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`patchable-function-entry` provides configurable nop padding before function symbols and after function symbols but before any generated code. We refer to the former as `prefix` padding and the latter as `entry` padding. For example, if we had a function `f` with `prefix` set to 3 and `entry` to 2, we'd expect to see:

```
nop
nop
nop
f:
nop
nop
// Code goes here
```

To set this for all functions in a crate, use `-C patchable-function-entry=nop_count,offset` where `nop_count = prefix + entry`, and `offset = prefix`. Usually, you'll want to copy this value from a corresponding `-fpatchable-function-entry=` being passed to the C compiler in your project.

To set this for a specific function, use `#[patchable_function_entry(prefix(m), entry(n))]` to pad with m nops before the symbol and n after the symbol, but before the prelude. This will override the flag value.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`patchable-function-entry` provides configurable nop padding before function symbols and after function symbols but before any generated code. We refer to the former as `prefix` padding and the latter as `entry` padding. For example, if we had a function `f` with `prefix` set to 3 and `entry` to 2, we'd expect to see:

```
f_pad:
nop
nop
nop
f:
nop
nop
// Code goes here
```

Nop padding may not be supported on all architectures. As of the time of writing, support includes:

- aarch64
- aarch64\_be
- loongarch32
- loongarch64
- riscv32
- riscv64
- i686
- x86\_64

`f_pad` addresses for every padded symbol are aggregated in the `__patchable_function_entries` section of the resulting object.
This is not a real symbol, just a collected location.

## Compiler flag `-C patchable-function-entry`

This flag comes in two forms:

- `-C patchable-function-entry=nop_count,offset`
- `-C patchable-function-entry=nop_count`

In the latter, offset is assumed to be zero. `nop_count` must be greater than or equal to `offset`, or it will be rejected.

If unspecified, the current behavior is maintained, which is equivalent to `=0` here.

This flag sets the default nop padding for all functions in the crate. Notably, this default *only applies to codegenned functions*. If a function is monomorphized during the compilation of another crate or any similar scenario, it will use the default from that crate's compilation. In most cases, all crates in a compilation should use the same value of `-C patchable-function-entry` to reduce confusion.

`prefix` is calculated as `offset`. `entry` is calculated as `nop_count - offset`. This unusual mode of specification is intended to mimic the compiler flags of `clang` and `gcc` for ease of build system integration.

Specifying the compiler flag for a backend or architecture which does not support this feature will result in an error.

## Attribute `#[patchable_function_entry]`

This attribute allows specification of either the `prefix` or `entry` values or both, using the format `#[patchable_function_entry(prefix(n), entry(n))]`. If either is left unspecified, it overrides them to a default value of 0.

As this is specified via an attribute, it will persist across crate boundaries unlike the compiler flag.

Specifying any amount of padding other than 0 in an attribute will result in an error on backends or architectures which do not support this feature.

# Drawbacks
[drawbacks]: #drawbacks

Not currently aware of any other than the complexity that comes from adding anything.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Implementation Levels
### Status Quo
If we keep to the status quo, we need to go through the Linux kernel making Rust support disable a variety of features which depend on this codegen feature. While I have not taken a complete inventory, this includes debugging features (e.g. `ftrace`) and hardening features (e.g. `FINEIBT`).

This alternative runs the risk of the Rust-for-Linux experiment not leaving experiment status, and similar systems with introspection considering Rust unsuitable.

The primary advantage of this design is that it does not require us to do anything.

### Only compiler flag
In this design, we only add the `-C patchable-function-entry` flag and not the attribute. This is enough for today - it would allow Rust to participate in these schemes, and in the event that a user *deeply* needed an uninstrumented function, they could build it as a separate crate.

This design has two drawbacks:

- It requires users to artificially structure their code as a form of annotation.
- The caveats around polymorphic functions using their codegen environment's flags could be tricky or surprising.

The primary advantage of this design is that it is purely a compiler feature, with no change to the language.

## Compiler flag and no-padding attribute
In this design, we add the compiler flag and an attribute that zeroes out padding for a function. This covers all the use cases I see in the Linux kernel today, so the only real downside is missing the opportunity to match `gcc` and `clang`'s capabilities with only a small bit more code.

Some other project might use explicit padding configuration per-function, but a quick search across github only finds the `patchable_function_entry` attribute set to `(0, 0)` other than in compiler tests.

## Everything (proposed design)
The only real downside I see here is the complexity of adding one more thing to the language.

# Argument style

There are two basic ways being used today to specify this nop padding:

- `nop_count`,`offset`, used by the attributes and flags in `gcc` and `clang`.
- `prefix`, `entry`, used by the *LLVM* attributes after translation from the language level attributes and flags.

The primary advantage of the first format is that it is used in `gcc` and `clang`. This means that existing documentation will not mislead users and tooling will have an easier time feeding the correct flag to Rust.

The advantage of the second style is that `prefix` and `entry` don't have validity constraints (`nop_count` must be greater than `offset`) and it's more obvious what the user is asking for.

## Copy `gcc`/`clang` everywhere

This approach has the advantage of matching all existing docs and programmers coming over not being confused.

## Use LLVM-style everywhere

This format doesn't require validation and is likely easier to understand for users not already exposed to this concept.

## Use `gcc`/`clang` for flags, LLVM-style for arguments (proposed)

Build systems tend to interact with our flag interface, and they already have `nop_count,offset` format flags constructed for their C compilers, so this is likely the easiest way for them to interface.

Users are unlikely to be directly copying code with a manual attribute, and usually are just going to be disabling padding per a github search for the attribute. Setting padding to `(0, 0)` is compatible across both styles, and setting `prefix` and `entry` manually is likely to be more understandable for a new user.

## Use `gcc`/`clang` for flags, Support both styles for arguments

Our attribute system is more powerful than `clang` and `gcc`, so we have the option to support:

- `prefix(n)`
- `entry(n)`
- `nop_count(n)`
- `offset(n)`

as modifiers to the attribute. We could make `prefix`/`entry` vs `nop_count`/`offset` an exclusive choice, and support both. This would provide the advantage of allowing users copying from or familiar with the other specification system to continue using it. The disadvantages would be more complex attribute parsing and potential confusion for people reading code.

## Support both styles for flags and arguments

In addition to supporting `nop_count`/`offset` for attributes, we could support this on the command line as well. This would have three forms:

- `-C patchable-function-entry=m` (`nop_count=m`, `offset=0`, compat format)
- `-C patchable-function-entry=m,n` (`nop_count=m`, `offset=n`, compat format)
- `-C patchable-function-entry=nop_count=m,offset=n` (`nop_count=m`, `offset=n`, modern format, offset optional)
- `-C patchable-function-entry=prefix=m,entry=n` (`prefix=m`, `entry=n`, modern format, either optional)

This would have the benefit of making it more clear what's being specified and allowing users to employ the simpler format on the command line if not integrating with an existing build.

The primary disadvantage of this is having many ways to say the same thing.

## Use LLVM-style for flags, `gcc`/`clang` for arguments

I'm not sure why we would do this.

# Prior art
[prior-art]: #prior-art

- Linux uses this flag and attribute extensively
- `clang` [implements the flag](https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fpatchable-function-entry)
- `clang` [implements the attribute](https://clang.llvm.org/docs/AttributeReference.html#patchable-function-entry)
- `gcc` [implements the flag](https://gcc.gnu.org/onlinedocs/gcc/Instrumentation-Options.html#index-fpatchable-function-entry)
- `gcc` [implements the attribute](https://gcc.gnu.org/onlinedocs/gcc/Common-Function-Attributes.html#index-patchable_005ffunction_005fentry-function-attribute)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should we use LLVM or `gcc`/`clang` style for a per-function attribute? Should we support both styles?
- Should we support a more explicit command line argument style?
- Should we reject linking crates with different default padding configurations?

# Future possibilities
[future-possibilities]: #future-possibilities

We could potentially use these for dynamic tracing of rust programs, similar to `#[instrument]` in the `tracing` today, but with more configurable behavior and even lower overhead (since there will be no conditionals to check, just a nop sled to go down).
