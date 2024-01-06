- Feature Name: `re_export_symbols`
- Start Date: 2024-01-05
- RFC PR: [rust-lang/rfcs#3556](https://github.com/rust-lang/rfcs/pull/3556)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Allow users to build a cdylib rust crate which both:

 - links against a pre-built staticlib
 - re-exports specific symbols from that staticlib

# Motivation
[motivation]: #motivation

Currently the Rust compiler attempts to restrict visibility of symbols not intended for export. This is good practice when building cdylibs (`.so` on Linux) as it produces much smaller linked output.

When building a cdylib, one will typically link to native dependencies either as 1) a cdylib or 2) a staticlib. The latter case can be preferable when trying to make the final cdylib product 'freestanding', with minimal dependencies on the host system.

When linking a staticlib into a cdylib crate, there are some cases where it is useful to re-export symbols. There is extensive dicussion from [people wanting to do this](https://github.com/rust-lang/rfcs/issues/2771) (with a note that the Lang team discussed this in 2017!), as well as another issue from people attempting to [use `+whole-archive`](https://github.com/rust-lang/rust/issues/110624) to re-export _all_ symbols from a linked staticlib.

However, the *only* supported way to re-export a symbol today is manually write wrappers around the staticlib symbols, re-passing the arguments down. This is:
 - inconvenient - re-exporting as the same name requires putting the `extern` block and wrapper function in different modules due to name collision
 - error-prone - the user has to maintain the wrapper function with the correct arguments
 - sometimes impossible - variadic functions are [not stable yet](https://github.com/rust-lang/rust/issues/44930)

This tightly-scoped RFC adds support for re-exporting explicitly listed symbols from a linked staticlib in an extern block to the final produced cdylib. It does *not* add 'wildcard' support to support re-exporting all symbols from a staticlib, nor does it cover other staticlib/cdylib combinations.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When producing a cdylib from a Rust crate and linking to a staticlib, adding `#[no_mangle]` and `pub` to an item in an extern block in the top level crate as follows:

```
#[no_mangle]
pub fn foo();
```

will make the `foo` symbol from the static library (e.g. `libext.a` on Linux) visible in the final produced `libcrate.so`.

If the crate is not being build as a cdylib the `#[no_mangle]` annotation will be ignored - this includes when the crate is a dependency of a cdylib crate.

If `ext` is not _explicitly_ linked as a static library the result is unspecified - the symbol may or may not be visible depending on the behavior of the linker. A non-fatal warning will be emitted:

```
warning: `#[no_mangle]` with `pub` is only valid for re-export when the library is explicitly a `staticlib`
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This feature [works today](https://github.com/aidanhs/rust-re-export-lib/) (at least on Linux), with a single caveat - a warning from the compiler:

```
   Compiling x v0.1.0 (/tmp/tmp.Sig2rLZept/rust-re-export-lib)
warning: `#[no_mangle]` has no effect on a foreign function
 --> src/lib.rs:5:5
  |
5 |     #[no_mangle]
  |     ^^^^^^^^^^^^ help: remove this attribute
6 |     pub fn foo();
  |     ------------- foreign function
  |
  = warning: this was previously accepted by the compiler but is being phased out; it will become a hard error in a future release!
  = note: symbol names in extern blocks are not mangled
  = note: `#[warn(unused_attributes)]` on by default

warning: `x` (lib) generated 1 warning (run `cargo fix --lib -p x` to apply 1 suggestion)
    Finished dev [unoptimized + debuginfo] target(s) in 0.25s
```

This warning is incorrect - `#[no_mangle]` does indeed have an effect on a foreign function, and it's exactly what this RFC needs. The implementation strategy for this PR then becomes:

1. remove the above warning
2. document the new behavior
3. add a warning for re-exporting a symbol when the library is not explicitly marked as a staticlib

On point 3 - the goal is to catch cases where a) the user has specified non-staticlib link kind or b) the user has not specified any link kind. The semantics of case (a) are not defined in this RFC (see [future-possibilities](#future-possibilities)), and catching (b) is to avoid the user accidentally falling into case (a) where the linker selects a dylib automatically.

# Drawbacks
[drawbacks]: #drawbacks

None that the RFC author is aware of.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

There are a few messages [on Zulip](https://rust-lang.zulipchat.com/#narrow/stream/131828-t-compiler/topic/Replacing.20no_mangle.20as.20a.20way.20to.20re-export.20symbols) talking a little about design space and related things.

Possible other annotations are:

 - `#[re_export]`
 - re-use the [proposed `#[export]` annotation](https://github.com/rust-lang/rfcs/pull/3435)

Both of these cause breakage for people using the existing workaround for debatable naming clarity. `#[no_mangle]` is a very familiar incantation for people wanting to make symbols available today, and, though the literal meaning doesn't quite match, it is visually consistent when used alongside other `extern` functions defined in Rust.

`#[export]` could potentially create additional confusion. That RFC states:

> For functions, the `#[export]` attribute will make the function available from the dynamic library under a stable "mangled" symbol that uniquely represents its crate and module path and full signature.

The RFC author believes that re-using this annotation would give it two very different (contradictory) meanings.

# Prior art
[prior-art]: #prior-art

The RFC author believes (but is not 100% sure) that languages and build systems where users might want this level of control often just require the user to instruct the system linker appropriately. This typically requires more knowledge of platform executable formats, but also permits much more fine control over final produced artifacts.

For a number of reasons, Rust as a project has opted to avoid exposing too many platform intricacies, which sacrifices control in favor of more consistent cross-platform behavior.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

 - Are we happy with the stabilisation of this existing behavior, or is a new keyword sufficiently better to warrant more discussion? In particular, is it necessary to delay until the `#[export]` RFC comes to its conclusion?

# Future possibilities
[future-possibilities]: #future-possibilities

This RFC makes it possible to re-export all symbols from a linked staticlib by iterating over all the symbols in a build script and writing them into an extern block that gets `include!`d into the cdylib crate. A more ergonomic re-export of all symbols from a staticlib included via `+whole-archive` may be desirable. It's not clear to the RFC author if this should always be the behavior of `+whole-archive` when pulling a staticlib into a cdylib, or if an additional flag is appropriate - the [original RFC](https://rust-lang.github.io/rfcs/2951-native-link-modifiers.html) that introduced `+whole-archive` is not fully illuminating for this case.

Scenarios out of scope of this RFC include:

1. re-export from a cdylib linked into a final cdylib
2. re-export from a cdylib linked into a final staticlib
3. re-export from a cdylib linked into a final binary
4. re-export staticlib linked into a final staticlib
5. re-export from a staticlib linked into a final binary

For 1, 2 and 3, the author isn't sure what platforms they are sensical (or even possible) on. 4 can be pessimistically approximated with `+whole-archive`, though results in larger final staticlibs than strictly necessary. 5 is valid on Linux, but only sometimes (a dynamically linked PIE ELF can be used as a shared library), and support on other platforms is beyond the RFC author's knowledge.

In all of these supplementary cases, the use-cases and discussion around them seem to be more niche than the primary motivation of this RFC. The RFC author would gratefully receive any additional information about these other use cases.
