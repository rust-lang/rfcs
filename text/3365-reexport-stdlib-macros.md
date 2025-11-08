- Feature Name: (fill me in with a unique ident, `reexport-stdlib-macros`)
- Start Date: (fill me in with today's date, 2023-01-04)
- RFC PR: [rust-lang/rfcs#3365](https://github.com/rust-lang/rfcs/pull/3365)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposed we start re-exporting macros currently exposed in `core::*`
from submodules. We define a mapping of which macros to re-export from which
submodules, though in some cases it makes most sense to keep the macros exported
from `core`.

This RFC does not yet propose we deprecate or migrate any code,
that is left up to future RFCs.

# Motivation
[motivation]: #motivation

Right now the Rust stdlib exports over 70 macros, most of which exist only in
the crate root - despite providing a wide range of functionality. This has
negative consequences for the stdlib's root: not only are there countless
submodules, there are also an overwhelming number of macros doing an assortment
of things. But it also has negative consequences for the individual submodules:
they will often only tell part of a story, and often leave out crucial
information on how they should actually be used.

Take for example the [`std::panic`
submodule](https://doc.rust-lang.org/std/panic/index.html). It includes various
methods to inspect, catch, and even modify panic behavior. But it includes no
facilities to actually trigger panics. Experienced rustaceans will know that
this can be done using `panic!`, `todo!`, `unimplemented!` and the like. But for
someone new to Rust, this information is not made readily available.

When macros are available from submodules we can begin to paint a more complete
picture of how those submodules are intended to be used. Up until recently it
wasn't possible to expose macros from submodules, but now that it is we should
take the opportunity to start making use of it.

# Mapping re-exports

The following table covers which macro we're talking about, which sub-module it
should be made available from, whether that's a new sub-module, and whether the
macro is unstable. Macros which are unstable can be _moved_ rather than just
re-exported.

| __Macro name__                         | __Proposed mod__   | __New exports?__ | __Unstable?__ |
|----------------------------------------|--------------------|------------------|---------------|
| `assert`                               | `core::panic`       | ✅                | ❌             |
| `assert_eq`                            | `core::panic`       | ✅                | ❌             |
| `assert_matches::assert_matches`       | `core::panic`       | ✅                | ✅             |
| `assert_matches::debug_assert_matches` | `core::panic`       | ✅                | ✅             |
| `assert_ne`                            | `core::panic`       | ✅                | ❌             |
| `cfg`                                  | `core`              | ❌                | ❌             |
| `clone::Clone`                         | `core::clone`       | ❌                | ❌             |
| `cmp::Eq`                              | `core::cmp`         | ❌                | ❌             |
| `cmp::Ord`                             | `core::cmp`         | ❌                | ❌             |
| `cmp::PartialEq`                       | `core::cmp`         | ❌                | ❌             |
| `cmp::PartialOrd`                      | `core::cmp`         | ❌                | ❌             |
| `column`                               | `core`              | ❌                | ❌             |
| `compile_error`                        | `core`              | ❌                | ❌             |
| `concat`                               | `core`              | ❌                | ❌             |
| `concat_bytes`                         | `core`              | ❌                | ✅             |
| `concat_idents`                        | `core`              | ❌                | ✅             |
| `const_format_args`                    | `core::fmt`         | ✅                | ✅             |
| `dbg`                                  | `core::io`          | ✅                | ❌             |
| `debug_assert`                         | `core::panic`       | ✅                | ❌             |
| `debug_assert_eq`                      | `core::panic`       | ✅                | ❌             |
| `debug_assert_ne`                      | `core::panic`       | ✅                | ❌             |
| `default::Default`                     | `core::default`     | ❌                | ❌             |
| `env`                                  | `core`              | ❌                | ❌             |
| `eprint`                               | `core::io`          | ✅                | ❌             |
| `eprintln`                             | `core::io`          | ✅                | ❌             |
| `file`                                 | `core`              | ❌                | ❌             |
| `fmt::Debug`                           | `core::fmt`         | ❌                | ❌             |
| `format`                               | `core::fmt`         | ✅                | ❌             |
| `format_args`                          | `core::fmt`         | ✅                | ❌             |
| `format_args_nl`                       | `core::fmt`         | ✅                | ✅             |
| `future::join`                         | `core`              | ❌                | ✅             |
| `hash::Hash`                           | `core::hash::Hash`  | ❌                | ❌             |
| `include`                              | `core`              | ❌                | ❌             |
| `include_bytes`                        | `core`              | ❌                | ❌             |
| `include_str`                          | `core`              | ❌                | ❌             |
| `is_aarch64_feature_detected`          | `core::arch`        | ✅                | ✅             |
| `is_arm_feature_detected`              | `core::arch`        | ✅                | ✅             |
| `is_mips64_feature_detected`           | `core::arch`        | ✅                | ✅             |
| `is_mips_feature_detected`             | `core::arch`        | ✅                | ✅             |
| `is_powerpc64_feature_detected`        | `core::arch`        | ✅                | ✅             |
| `is_powerpc_feature_detected`          | `core::arch`        | ✅                | ✅             |
| `is_riscv_feature_detected`            | `core::arch`        | ✅                | ✅             |
| `is_x86_feature_detected`              | `core::arch`        | ✅                | ✅             |
| `line`                                 | `core`              | ❌                | ❌             |
| `llvm_asm`                             | `core`              | ❌                | ✅             |
| `log_syntax`                           | `core`              | ❌                | ✅             |
| `marker::Copy`                         | `core::marker`      | ❌                | ❌             |
| `matches`                              | `core`              | ❌                | ❌             |
| `module_path`                          | `core`              | ❌                | ❌             |
| `option_env`                           | `core`              | ❌                | ❌             |
| `panic`                                | `core::panic`       | ✅                | ❌             |
| `prelude::v1::bench`                   | `core::prelude::v1` | ❌                | ✅             |
| `prelude::v1::cfg_accessible`          | `core::prelude::v1` | ❌                | ❌             |
| `prelude::v1::cfg_eval`                | `core::prelude::v1` | ❌                | ✅             |
| `prelude::v1::derive`                  | `core::prelude::v1` | ❌                | ❌             |
| `prelude::v1::global_allocator`        | `core::prelude::v1` | ❌                | ❌             |
| `prelude::v1::test`                    | `core::prelude::v1` | ❌                | ❌             |
| `prelude::v1::test_case`               | `core::prelude::v1` | ❌                | ✅             |
| `print`                                | `core::io`          | ✅                | ❌             |
| `println`                              | `core::io`          | ✅                | ❌             |
| `ptr::addr_of`                         | `core::ptr`         | ❌                | ❌             |
| `ptr::addr_of_mut`                     | `core::ptr`         | ❌                | ❌             |
| `simd::simd_swizzle`                   | `core::simd`        | ❌                | ✅             |
| `stringify`                            | `core`              | ❌                | ❌             |
| `task::ready`                          | `core::task`        | ❌                | ❌             |
| `thread_local`                         | `core::thread`      | ✅                | ❌             |
| `todo`                                 | `core::panic`       | ✅                | ❌             |
| `trace_macros`                         | `core`              | ❌                | ✅             |
| `try`                                  | `core`              | ❌                | ❌             |
| `unimplemented`                        | `core::panic`       | ✅                | ❌             |
| `unreachable`                          | `core::panic`       | ✅                | ❌             |
| `vec`                                  | `core::vec`         | ✅                | ❌             |
| `write`                                | `core`              | ❌                | ❌             |
| `writeln`                              | `core`              | ❌                | ❌             |

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The implementation of this RFC should be no more than adding a re-export from
the submodule to the existing macro found in the crate root. Say we're re-exporting
`core::assert` from `core::panic::assert`, we could imagine it being done along
these lines:

```rust
pub mod core {
    /// Panics the current thread.
    #[macro_export]
    macro_rules! panic { ... }

    pub mod panic {
        pub use crate::panic;
    }
}
```

Some of these macros such as `panic!` will be built-ins, meaning that changing
their implementations might have implications for the compiler. Because this RFC
only proposes we re-export macros and not _migrate_ macros (see "future
possibilities"), simply creating an alias for the macro from the submodule is enough.

# Prior art
[prior-art]: #prior-art

It was only up to recently that it wasn't possible to export macros from
submodules. New macros being added to the stdlib are already available from
submodules (e.g. `std::ptr::addr_of`, `std::task::ready`). And derive-macros
have always been available from submodules (e.g.  `std::clone::Clone`,
`std::hash::Hash`).

Now that the technical restriction has been lifted, we can finally look
at the existing macros and start to re-export them from their logical
submodules.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## `write/writeln`

Both the `write` and `writeln` macros call a method named `write` on a type, as
exists in the stdlib as
[`std::fmt::Write`](https://doc.rust-lang.org/std/fmt/trait.Write.html) and
[`std::io::Write`](https://doc.rust-lang.org/std/io/trait.Write.html).

Because `write/writeln` both need to be available from `core`, they would need
to _at least_ be available from `core::fmt` - there is no `core::io`. But when
considering the `std` docs it might make sense to expose them from both. One
point in favor of doing it from both locations is that it would enable both the
`fmt` and `io` docs to be more self-contained. But it's unclear what the best
option would be. What should we do here?

# Future possibilities
[future-possibilities]: #future-possibilities

## Deprecate macros in stdlib root

Once we have the macros re-exported from their respective submodules, we can
start looking at what to do with the macros currently still in the stdlib's
root. Should we deprecate them over an edition? Should we change the way they're
shown in the docs? Because having 50 or so deprecated items in the crate root
seems like a lot.

It's likely we'll want to do _something_ here, but it's unclear what exactly.
Therefor we're leaving this as an open question which should be explored in the
future, but for now is out of the scope of this RFC.

## API Guidlines

It might be helpful for the stdlib's API guidelines to include a section
explaining when to export macros from existing submodules, when to export them
from newly created submodules, and when to export them from the crate root.
