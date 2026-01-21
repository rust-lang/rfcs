- Feature Name: `export_visibility`
- Start Date: 2025-06-12
- RFC PR: [rust-lang/rfcs#3834](https://github.com/rust-lang/rfcs/pull/3834)
- Rust Issue:[rust-lang/rust#151425](https://github.com/rust-lang/rust/issues/151425)

# Summary
[summary]: #summary

Documentation of
[`#[no_mangle]`](https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute)
points out that by default a `#[no_mangle]` function (or `static`)
will be "publicly exported from the produced library or object file".
This RFC proposes a new `#[export_visibility = ...]` attribute
to override this behavior.
This means that if the same `#[no_mangle]` function is also
decorated with `#[export_visibility = "target_default"]`,
then it will instead use the default visibility of the target platform
(which can be overriden with the
[`-Zdefault-visibility=...`](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/default-visibility.html)
command-line flag).

# Motivation
[motivation]: #motivation

## Context: Enabling non-mangled, non-exported symbols

Rust items (functions or `static`s) decorated with
[`#[no_mangle]`](https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute)
or
[`#[export_name = ...]`](https://doc.rust-lang.org/reference/abi.html#the-export_name-attribute)
are by default publicly exported.
https://github.com/rust-lang/rust/issues/98449 points out that this means that
"it is not possible to define an un-mangled and un-exported symbol in Rust".
The new attribute proposed by this RFC would make this possible - this in turn
may realize the benefits described in the subsections below.

## Context: Impact on FFI tooling

`#[no_mangle]` and `#[export_name = ...]` are the only way to specify
an exact symbol name that can be used outside of Rust (e.g. from C/C++)
to refer to an item (a function or a `static`) defined in Rust.
This means that FFI libraries and tools can't really avoid problems
caused by unintentional public exports.
This ties this RFC with one of `rust-project-goals`:
https://github.com/rust-lang/rust-project-goals/issues/253.
Adopting this RFC should improve this aspect of cross-language interop.

## Benefit: Smaller binaries

One undesirable consequence of unnecessary public exports is binary size bloat.
In particular, https://github.com/rust-lang/rust/issues/73958 points out
that:

> [...] cross-language LTO is supposed to inline the FFI functions into their
> callers.  However, having them exported means also keeping those copies
> around. Also, unused FFI functions can't be eliminated as dead code.

## Benefit: Faster loading

Unnecessarily big tables of dynamically-loaded symbols
have negative impact on runtime performance.
For example, GCC wiki
[points out](https://gcc.gnu.org/wiki/Visibility)
that hiding unnecessary exports
"very substantially improves load times of your DSO (Dynamic Shared Object)".

## Benefit: Prevent misuses of internal functions

A shared library implemented in a mix of Rust and some other languages may use
`#[export_name = ...]` or `#[no_mangle]` to enable calling Rust functions from
those other languages.  Some of those functions will be internal implementation
details of the library.  Using `#[export_visibility = ...]` to hide those
functions will prevent other code from depending on those internal details.

## Benefit: Parity with C++

C++ developers can control visibility of their symbols with:

* `-fvisibility=...` command-line flag can be used in
  [Clang](https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fvisibility)
  or
  [GCC](https://gcc.gnu.org/onlinedocs/gcc/Code-Gen-Options.html#index-fvisibility)
* Per-item `__attribute__ ((visibility ("default")))` is recognized by
  [Clang](https://clang.llvm.org/docs/AttributeReference.html#visibility)
  and
  [GCC](https://gcc.gnu.org/onlinedocs/gcc/Common-Function-Attributes.html#index-visibility-function-attribute)

Rust has an equivalent command-line flag (
[`-Zdefault-visibility=...`](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/default-visibility.html),
tracked in https://github.com/rust-lang/rust/issues/131090).
OTOH, Rust doesn't have an equivalent attribute.
Adopting this RFC would be a step toward closing this gap.

## Context: Undefined behavior caused by naming collisions
[ub-intro]: #context-undefined-behavior-caused-by-naming-collisions

The subsections below attempt to provide details about the risk of undefined
behavior (UB) caused by duplicate symbol definitions.

### Presence of UB risk

The fact that naming collisions may cause UB is documented in the documentation
of [`#[export_name =
...]`](https://doc.rust-lang.org/reference/abi.html#the-export_name-attribute)
which points out that "this attribute is `unsafe` as a symbol with a custom name
may collide with another symbol with the same name (or with a well-known
symbol), leading to undefined behavior".  Similar UB risk is documented for the
[`#[no_mangle]`](https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute)
attribute.

### Scope of UB risk
[scope-of-naming-collision-risk]: #scope-of-ub-risk

The risk of name collisions is caused by two separate behaviors of
`#[export_name = ...]` and `#[no_mangle]`:

* Turning-off mangling (e.g. see
  [here](https://github.com/rust-lang/rust/blob/3d8c1c1fc077d04658de63261d8ce2903546db13/compiler/rustc_symbol_mangling/src/lib.rs#L240-L243))
  introduces the _possibility_ of naming collisions.
* Exporting the symbol with public visibility (e.g. see
  [here](https://github.com/rust-lang/rust/blob/8111a2d6da405e9684a8a83c2c9d69036bf23f12/compiler/rustc_monomorphize/src/partitioning.rs#L930-L937))
  increases the _scope_ of possible naming collisions (covering all DSOs).

### Origins of UB

It is out of scope for this RFC to identify and/or explain the exact origin
and/or mechanisms of the UB.  Nevertheless, discussions related to this RFC may
benefit from outlining at a high-level how the UB may happen, so this topic
is explored underneath the folded details section below.

<details>

The author of this RFC is not aware of a more authoratitative source that would
explain the mechanisms that can lead to the UB in presence of naming collisions.
The author speculates that:

* Compilers may assume that each symbol is defined only once (and that breaking
  this assumption can lead to UB).  Examples of such assumption:
    - C++ documents [One Definition Rule
      (ODR)](https://en.cppreference.com/w/cpp/language/definition.html#One_Definition_Rule).
      This rule necessarily extends to binaries that link C++ compilation
      artifacts with `rustc` artifacts (even if official Rust documentation
      doesn't AFAIK talk about this rule).
    - LLVM optimization passes assume that if they see a definition of a symbol,
      then this is the definition that will be actually used (for symbols with
      normal linkage - not weak, odr, etc.).  LLVM supports suppressing this
      assumption with [the SemanticInterposition
      feature](https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fsemantic-interposition),
      but `rustc` doesn't use this LLVM feature (e.g. see
      [here](https://github.com/rust-lang/rfcs/pull/3834#discussion_r2395618137)).
      Special thanks to @jyknight for pointing this out.
* Linkers don't define which exact definition will be used when multiple
  definitions are present
    - LLVM explicitly
      [documents](https://llvm.org/docs/LangRef.html#linkage-types) this for
      `linkonce_odr` and `weak_odr` saying that "one of the definitions is
      _non-deterministically_ chosen to run" (_emphasis_ mine).
    - It seems likely that dynamic linking may also be non-deterministic when
      multiple definitions are present.  For example, it seems that the
      choice of a definition may depend on the order in which DSOs are linked
      (and it seems fair to treat this order as non-deterministic, or at least
      outside the immediate control of the code author).

</details>

## Benefit: Avoiding undefined behavior

Using `#[export_visibility = ...]` to reduce symbol visibility can be used to
reduce or eliminate the risk of undefined behavior (UB) described in the
previous [ub-intro] section.

UB caused by high symbol visibility is not just a hypothetical risk - this risk
has actually caused difficult to diagnose symptoms that are captured in
https://crbug.com/418073233.  More information about this bug can be found in
the folded details section below.

<details>

In the smaller repro from
https://crrev.com/c/6580611/1 we see the following:

* Without this RFC the [`cxx`](https://cxx.rs) library cannot avoid publicly
  exporting symbols that are called from C++.  In particular, the following
  two symbols are publicly exported from a static library called `rust_lib`:
    - `rust_lib$cxxbridge1$get_string` (generated by `#[cxx::bridge]` proc macro
      to generate an FFI/C-ABI-friendly thunk for
      [`rust_lib::get_string`](https://chromium-review.googlesource.com/c/chromium/src/+/6580611/1/build/rust/tests/test_unexpected_so_hop_418073233/src/lib.rs)
    - `cxxbridge1$string$drop` exported from
      [`cxx/src/symbols/rust_string.rs`](https://github.com/dtolnay/cxx/blob/07d2bca38b7bfbbe366a9e844d3d66b80820d339/src/symbols/rust_string.rs#L83C18-L86)
* In the repro case, `rust_lib` is statically linked into the main test
  executable **and** into an `.so`.  This results in the naming collision,
  because now `rust_lib$cxxbridge1$get_string` and `cxxbridge1$string$drop` both
  have two definitions - one definition in the test executable and one in the
  `.so`.
* The test executable calls `rust_lib$cxxbridge1$get_string` and then
  `cxxbridge1$string$drop`.
    - Side-note: The `.so` statically links `rust_lib`, but doesn't actually use
      it.  (In the original repro the `.so` used a small part of a bigger
      statically linked "base" library and also didn't actually use Rust's
      `cxx`-related symbols.  See https://crrev.com/c/6504932 which removes the
      `.so`'s dependency on the "base" library as a workaround for this
      problem.)
* Because naming collisions lead to UB (see the [ub-intro] section above),
  it is non-deterministic whether calling `rust_lib$cxxbridge1$get_string` will
  end up calling the definition in the test executable VS in the `.so`.  Similar
  UB exists for calls to `cxxbridge1$string$drop`.
* The UB from the previous item leads to memory unsafety when:
    - The call from test executable to `rust_lib$cxxbridge1$get_string`
      ends up calling the definition in the `.so`, rather than in the
      executable.  This means that allocations made by `get_string` use **one**
      set of the allocator's global symbols - the copy within the `.so`.
    - The call from test executable to `cxxbridge1$string$drop` ends up
      calling the definition in the executable, rather than in the `.so`.
      This means that freeing the previous allocation uses **other**
      set of allocator's global symbols - the ones in the executable.
    - Using wrong global symbols means that the executable's allocator tries to
      free an allocation that it doesn't know anything about (because this
      allocation has been make by the allocator from the `.so`).  In debug
      builds this is caught by an assertion.  In release builds this would lead
      to memory unsafety.

</details>

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## The `export_visibility` attribute

[`#[no_mangle]`](https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute)
or
[`#[export_name = ...]`](https://doc.rust-lang.org/reference/abi.html#the-export_name-attribute)
attribute may be used to export
a Rust
[function](https://doc.rust-lang.org/reference/items/functions.html)
or
[static](https://doc.rust-lang.org/reference/items/static-items.html).
The `#[export_visibility = ...]` attribute overrides visibility
of such an exported symbol.

The `#[export_visibility = ...]` attribute uses the
[MetaNameValueStr](https://doc.rust-lang.org/reference/attributes.html#meta-item-attribute-syntax)
syntax to specify the desired visibility.  The following sections describe
string values that may be used.

### Default target platform visibility

`#[export_visibility = "target_default"]` uses
the default visibility of the target platform.

Note: the nightly version of the `rustc` compiler
supports overriding the target platform's visibility with the
[`-Zdefault-visibility=...`](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/default-visibility.html)
command-line flag.

#### End-to-end example

Consider the following example code:

    ```
    #![feature(export_visibility)]

    #[unsafe(export_name = "test_fn_no_attr")]
    unsafe extern "C" fn test_fn_with_no_attr() -> u32 {
        line!()  // `line!()` avoids identical code folding (ICF)
    }

    #[unsafe(export_name = "test_fn_target_default")]
    #[export_visibility = "target_default"]
    unsafe extern "C" fn test_fn_asks_for_target_default() -> u32 {
        line!()  // `line!()` avoids identical code folding (ICF)
    }
    ```

When the code above is built into a DSO,
then `-Zdefault-visiblity=hidden` will affect visibility of the 2nd function
and prevent it from getting exported from the DSO.
See below for an example of how this may be observed on a Linux system:

    ```
    $ rustc ~/scratch/export_visibility_end_to_end_test.rs \
        --crate-type=cdylib \
        -o ~/scratch/export_visibility_end_to_end_test_with_hidden_default_visibility.so \
        -Zdefault-visibility=hidden

    $ readelf \
            --dyn-syms \
            --demangle \
            ~/scratch/export_visibility_end_to_end_test_with_hidden_default_visibility.so \
        | grep test_fn

        55: 0000000000035920     6 FUNC    GLOBAL DEFAULT   15 test_fn_no_attr
    ```

#### LLVM-level example

`tests/codegen-llvm/export-visibility.rs` proposed in
[a prototype associated with this RFC](https://github.com/rust-lang/rust/commit/1e1924bdac60b3b522ecffefbedfef94e4aa79d5#diff-25436b0328a03fca2c8be8a36152e30d58272315d690d9b3b6b5f0b5ebf35269)
has the following expectations for the test functions from the example in the
previous section (with `-Zdefault-visibility=hidden`):

```
// HIDDEN: define noundef i32 @test_fn_no_attr
// HIDDEN: define hidden noundef i32 @test_fn_target_default
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Edits to reference documentation for `#[no_mangle]`

If this RFC is adopted (and stabilized) then
https://doc.rust-lang.org/reference/abi.html#r-abi.no_mangle.publicly-exported
should be edited.

Old text:

    > Additionally, the item will be publicly exported from the produced library
    > or object file, similar to the used attribute.

New text:

    > Unless overridden by `#[export_visibility = …]`, the item will be publicly
    > exported from the produced library or object file, similar to the used
    > attribute.

## Edits to reference documentation for `#[export_name]`

If this RFC is adopted (and stabilized) then
https://doc.rust-lang.org/reference/abi.html#r-abi.export_name should be edited.

Old text doesn’t mention symbol visibility, or exporting.

New text / paragraph:

    > Unless overridden by `#[export_visibility = …]`, the item will be publicly
    > exported from the produced library or object file, similar to the used
    > attribute.

## New section for `#[export_visibility = …]`

If this RFC is adopted (and stabilized) then
https://doc.rust-lang.org/reference/abi.html should get a new section:

    > # The `export_visibility` attribute
    >
    > Intro-tag: The _`export_visibility` attribute_ overrides if or how the
    > item is exported from the produced library or object file.
    > The `export_visibility` attribute can only be applied to
    > items with `#[no_mangle]` or `#[export_name = ...]` attributes.
    >
    > Syntax-tag: The export_visibility attribute uses the MetaNameValueStr
    > syntax to specify the symbol name.
    >
    > Target-default-tag: Currently only `#[export_visibility =
    > “target_default”]` is supported.  When used, it means that the item will
    > be exported with the default visibility of the target platform (which may
    > be overridden by the unstable `-Zdefault-visibility=...` command-line
    > flag.

Note that the applicability wording proposed above
is based on the following factors:

* Desire to only apply the `#[export_visibility = ...]` attribute to items
  for which
  [`contains_extern_indicator`](https://github.com/rust-lang/rust/blob/3bc767e1a215c4bf8f099b32e84edb85780591b1/compiler/rustc_middle/src/middle/codegen_fn_attrs.rs#L174-L184)
  is `true`.  Today this covers:
    - All items that use the `#[no_mangle]` attribute
    - All items that use the `#[export_name = ...]` attribute
    - All items that use the `#[rustc_std_internal_symbol]` attribute
    - Some items that use `#[linkage = ...]`
      (note that this attribute has not yet been
      [stabilized](https://doc.rust-lang.org/beta/unstable-book/language-features/linkage.html?highlight=linkage#linkage)
      and this is why it is not yet mentioned in the proposed reference text
      above)
- Desire to forbid applying the `#[export_visibility = ...]` attribute
  in cases where doing so may increase an item visibility.
    - This is why `#[rustc_std_internal_symbol]` is intentionally omitted
      and why the RFC proposes that using `#[export_visibility = ...]` for
      `#[rustc_std_internal_symbol]` items should be an error.  See also
      the [why-new-attr-cant-increase-visibility] section below.

## Other details

Other details (probably not important enough to include in the official
reference documentation for Rust):

* The proposal in this RFC has been prototyped in
  https://github.com/anforowicz/rust/tree/export-visibility



# Drawbacks
[drawbacks]: #drawbacks

No drawbacks have been identified at this point.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Context: why the new attribute cannot increase visibility
[why-new-attr-cant-increase-visibility]: #context-why-the-new-attribute-cannot-increase-visibility

The `#[export_visibility = ...]` attribute may only be applied to item
definitions with an "extern" indicator as checked by [`fn
contains_extern_indicator`](https://github.com/rust-lang/rust/blob/3bc767e1a215c4bf8f099b32e84edb85780591b1/compiler/rustc_middle/src/middle/codegen_fn_attrs.rs#L174-L184).

Based on the above, the `#[export_visibility = ...]` attribute may never
_increase_ visibility of a symbol.  This is because:

* `#[no_mangle]` and `#[export_name = ...]` force the
  _maximum_ possible visiblity.  See
  [here](https://github.com/rust-lang/rust/blob/8111a2d6da405e9684a8a83c2c9d69036bf23f12/compiler/rustc_monomorphize/src/partitioning.rs#L930-L937)
* It seems that `#[linkage = ...]` should have no impact on symbol visibility
* One known exception is `#[rustc_std_internal_symbol]` - see
  [here](https://github.com/rust-lang/rust/blob/8111a2d6da405e9684a8a83c2c9d69036bf23f12/compiler/rustc_codegen_ssa/src/back/symbol_export.rs#L527-L542).
  The RFC avoids this exception by disallowing using
  `#[export_visibility = ...]` with `#[rustc_std_internal_symbol]`.

## Rationale for not supporting `interposable` visibility

The [why-new-attr-cant-increase-visibility] section above means that
`#[export_visibility = "interposable"]` would be a no-op.  Because of this, the
`"interposable"` visibility value is not supported by the
`#[export_visibility = ...]` attribute.

> Side-note: The "interposable" visibility is sometimes called
> "default" [linker] visibility (see [the LLVM documentation
> here](https://llvm.org/docs/LangRef.html#visibility-styles)),
> or "public" or "exported" visibility.

Lack of support for the `"interposable"` visibility means that this RFC avoids
potential open questions about interaction with `__declspec(dllexport)` and/or
whether `rustc` would have to enable the [the LLVM SemanticInterposition
feature](https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fsemantic-interposition).

## Rationale for not requiring `unsafe` when using the new attribute

### Naming collisions

The risk of naming collisions is introduced by lack of mangling
(e.g. caused by the presence of `#[no_mangle]` or `#[export_name = ...]`
attributes).  Presence of `#[export_visibility = ...]` does not
_introduce_ this risk.

The [scope-of-naming-collision-risk] section above points out that symbol
visiblity affects the _scope_ of the risk of undefined behavior (UB) stemming
from naming collisions.  `#[export_visibility = ...]` never increases this risk,
because the [why-new-attr-cant-increase-visibility] section above shows that
`#[export_visibility = ...]` can never _increase_ visibility of a symbol.

### Missing symbols

The [hidden-vs-dylibs] section below points out that using
`#[export_visibility = ...]` may break `dylib`s.  This concern
is tracked as an open question, but this kind of breakage
is well-defined and doesn't lead to undefined behavior.

In particular, it is understood that hiding symbols from `dylib` may result in
linking failures (symbol X not found).  This risk is quite similar to the risk
of forgetting to write `pub mod` instead of `mod` (and we don't require writing
`unsafe mod`).

## Alternative: `#[rust_symbol_export_level]`

The `#[export_visibility = ...]` proposed in this RFC supports directly
controlling an exact visibility level of a symbol.  One alternative is
to control the visibility indirectly, levereging the fact that `#[no_mangle]`
and `#[export_name = ...]` symbols are currently public only because:

* Such symbols are treated as `SymbolExportLevel::C`:
  https://github.com/rust-lang/rust/blob/3048886e59c94470e726ecaaf2add7242510ac11/compiler/rustc_codegen_ssa/src/back/symbol_export.rs#L593-L605
* `SymbolExportLevel::C` translates into public visibility, but
  visibility of `SymbolExportLevel::Rust` may be controlled via
  `-Zdefault-visibility=...`:
  https://github.com/rust-lang/rust/blob/3048886e59c94470e726ecaaf2add7242510ac11/compiler/rustc_monomorphize/src/partitioning.rs#L941-L948

Special thanks to @bjorn3 for proposing this alternative in
https://github.com/rust-lang/rfcs/pull/3834#issuecomment-2978073435

This alternative has been prototyped in
https://github.com/rust-lang/rust/commit/9dd4d3f6b2beecb85ff4220502a8c7f61edca839
and tested to verify that it also addresses https://crbug.com/418073233
(with similar test/repro steps as in #comment12 of that bug, but using
https://crrev.com/c/6580611/3).

Other notes:

* Pros:
    - It is simpler
      (`#[rust_symbol_export_level]` vs `#[export_visibility = "<some value>"]`)
      both for users and for implementation.
    - It avoids some problems and open questions associated with
      `#[export_visibility = ...]`:
        - No `dylib` trouble (see [hidden-vs-dylibs])
        - No need to define behavior of specific visibilities - this question
          is punted to `-Zdefault-visibility=...`.
          See also [cross-platform-behavior].
* Cons:
    - Doesn't give the same level of control as C++ attributes
* Open questions:
    - The name of the attribute proposed in this alternative is subject to
      change if a better name is proposed.
* Possible follow-ups (but probably out-of-scope for this RFC):
    - @chorman0773 pointed out in
      https://github.com/rust-lang/rfcs/pull/3834#issuecomment-2981459636
      that an inverse attribute may also be desirable in some scenarios
      (e.g. `c_symbol_export_level`).

## Alternative: version scripts

Using
[linker version scripts](https://sourceware.org/binutils/docs/ld/VERSION.html)
has been proposed as a way to control visibility of Rust-defined symbols
(e.g. this is a workaround pointed out in
https://github.com/rust-lang/rust/issues/18541).
In particular, using version scripts is indeed a feasible way of avoiding
undefined behavior from https://crbug.com/418073233.

Using a version script has the following downsides compared to
the `#[export_visibility = ...]`-based approach proposed in this RFC:

* Using the attribute allows the compiler to optimize the code a bit more than
  when using a version script
  (based on
  [this Stack Overflow answer](https://stackoverflow.com/a/58527480/24042981))
* Using a version script means that visibility of a symbol is defined in a
  centralized location, far away from the source code of the symbol.
    - Copying symbol definitions from `.rs` files into a new library is not
      sufficient for preserving symbol visibility (for that the version script
      has to be replicated as well).
    - There is a risk that version script and the symbol definition may diverge
      (e.g. after renaming symbol name in an `.rs` file, one has to remember
      to check if a version script also needs to be updated).
* Version scripts don't work on all target platforms.  In particular,
  they work in GNU `ld` and LLVM `lld`, but the native Microsoft Visual C++
  linker (`link.exe`) does not directly support GNU-style version scripts.
  Instead, MSVC uses `.def` (module-definition) files to control symbol export
  and other aspects of DLL creation.  Having to use
  [a `.def` file](https://learn.microsoft.com/en-us/cpp/build/reference/exports?view=msvc-170)
  has a few extra downsides compared to a version script:
    - Having to support both formats
    - Lack of support for wildcards means that it is impossible to hide
      all symbols matching a pattern like `*cxxbridge*` used by `cxx` in
      auto-generated FFI thunks.
* Using a version script is one way of fixing https://crbug.com/418073233.
  This fix approach requires that authors of each future shared library know
  about the problem and use a version script.  This is in contrast to using
  `-Zdefault-visibility=hidden` and `#[export_visibility = "target_default"]`
  for `cxx` symbols, which has to be done only once to centrally, automatically
  avoid the problem for all `cxx`-dependent libraries in a given build
  environment.  (In fairness, using the command-line flag also requires
  awareness and opt-in, but it seems easier to append
  `-Zdefault-visibility=hidden` to default `rustflags` in globally-applicable
  build settings than it is to modify build tools to require a linker script for
  all shared libraries.  In fact, Chromium
  [already builds with the `-Zdefault-visibility=...` flag](https://source.chromium.org/chromium/chromium/src/+/main:build/config/gcc/BUILD.gn;l=34-35;drc=ee3900fd57b3c580aefff15c64052904d81b7760).)

## Alternative: introduce `-Zdefault-visibility-for-c-exports=...`

Introducing and using `-Zdefault-visibility-for-c-export=hidden`
can realize most benefits outlined in the "Motivation" section
(except C/C++ parity).
In particular this is a feasible way of avoiding undefined behavior from
https://crbug.com/418073233.

The main downside, is that it doesn't support making a subset of Rust-defined
symbols public, while hiding another subset.  This may still be achievable,
but would require reaching out for C/C++ to export some symbols
(i.e. defining `foo_hidden` in Rust, and then calling it from a publicly
exported `foo` defined in C/C++).

## Alternative: change behavior of `#[no_mangle]` in future language edition

Some past proposals suggested changing the behavior of `#[no_mangle]`
(and `#[export_name = ...]`) attribute in a future Rust language edition.
For example, this is what has been proposed in
https://github.com/rust-lang/rust/issues/73958#issuecomment-2889126604
(although it seems that this proposal wouldn't help with
https://crbug.com/418073233, because it seems to only affect scenarios where
linking is driven by `rustc`).
Other edition-boundary changes may also be considered - for example
just changing the default effect of `#[no_mangle]` from
(pseudo-code) `#[export_visibility = "interposable"]` to
`#[export_visibility = "target_default"]`
(which combined with `-Zdefault-visibility=hidden` should address
https://crbug.com/418073233).

It seems that the new edition behaviors proposed above would still benefit from
having a way to diverge from the default visibility behavior of `#[no_mangle]`
symbols.  And therefore it seems that the `#[export_visibility = ...]` attribute
proposed in this RFC would be useful not only in the current Rust edition,
but also in the hypothetical future Rust edition.

## Rationale: Okay to have no impact on Rust standard library

This RFC treats visibility of Rust standard library symbols as out of scope.
`-Zdefault-visibility=...` remains the only way to control symbol visibility
of the Rust standard library (assumming that it can be rebuilt with this
command-line flag).  This is ok - the RFC is beneficial even with this limited
scope.

More details about symbols exported from the Rust standard library can be
found in the folded details section below:

<details>

### Symbols exported from Rust standard library

Currently Rust standard library may end up exporting two kinds of symbols.
One kind is symbols using `#[rustc_std_internal_symbol]` attribute
(similar to `#[no_mangle]` so in theory `#[export_visibility = ...]`
attribute could be applied to such symbols).
An example can be found below:

```
$ git clone git@github.com:guidance-ai/llguidance.git
$ cd llguidance/parser
$ cargo rustc -- --crate-type=staticlib
...
$ nm --demangle --defined-only ../target/debug/libllguidance.a 2>/dev/null | grep __rustc::
0000000000000000 T __rustc::__rust_alloc
0000000000000000 T __rustc::__rust_dealloc
0000000000000000 T __rustc::__rust_realloc
0000000000000000 T __rustc::__rust_alloc_zeroed
0000000000000000 T __rustc::__rust_alloc_error_handler
0000000000000000 B __rustc::__rust_alloc_error_handler_should_panic
00000000 T __rustc::__rust_probestack
```

But non-`#[rustc_std_internal_symbol]` symbols (e.g.
[`String::new`](https://github.com/rust-lang/rust/blob/9c4ff566babe632af5e30281a822d1ae9972873b/library/alloc/src/string.rs#L439-L446))
can also end up publicly exported:

```
$ nm --demangle --defined-only ../target/debug/libllguidance.a 2>/dev/null \
    | grep alloc::string::String::new
0000000000000000 T alloc::string::String::new
0000000000000000 T alloc::string::String::new
0000000000000000 T alloc::string::String::new
0000000000000000 t alloc::string::String::new
0000000000000000 T alloc::string::String::new
0000000000000000 T alloc::string::String::new
0000000000000000 T alloc::string::String::new
```

> **Disclaimer**: The example above could be illustrated with other crates.
> It uses `llguidance` because:
>
> 1. it exposes C API
>    (and therefore it is potentially useful to build it as a `staticlib`)
> 2. it happens to be used by Chromium so the RFC author is somewhat familiar
>    with the crate
> 3. the RFC author had trouble building `rustc-demangle-capi` in this way
>    (hitting `#[panic_handler]`-related errors).

### Justification for relying on `-Zdefault-visibility=...`

Symbols can be hidden by rebuilding Rust standard library with
`-Zdefault-visibility=hidden`.

There are other valid reasons
for rebuilding the standard library when building a given project.
For example this is a way to use globally consistent `-C` options
like `-Cpanic=abort`,
[`-Clto=no`](https://source.chromium.org/chromium/chromium/src/+/main:build/config/compiler/BUILD.gn;l=1115-1118;drc=26d51346374a0d16b0ba2243ef83c015a944d975),
etc.

Rebuilding the standard library is possible,
although it is currently supported as an **unstable**
[`-Zbuild-std`](https://doc.rust-lang.org/cargo/reference/unstable.html#build-std)
command-line flag of `cargo`.
FWIW Chromium currently does rebuild the standard library
(using automated
[tooling](https://source.chromium.org/chromium/chromium/src/+/main:tools/rust/gnrt_stdlib.py;drc=628c608971bc01c96193055bb0848149cccde645)
to translate standard library's `Cargo.toml` files into
[equivalent `BUILD.gn` rules](https://source.chromium.org/chromium/chromium/src/+/main:build/rust/std/rules/BUILD.gn;drc=35fb76c686b55acc25b53f7e5c9b58e56dca7f4a)),
which is one reason why this RFC is a viable UB fix for
https://crbug.com/418073233.

</details>

# Prior art
[prior-art]: #prior-art

## Other languages

This RFC is quite directly based on how C/C++ supports
per-item `__attribute__ ((visibility ("default")))` (at least in
[Clang](https://clang.llvm.org/docs/AttributeReference.html#visibility)
and
[GCC](https://gcc.gnu.org/onlinedocs/gcc/Common-Function-Attributes.html#index-visibility-function-attribute)).
Using an assembly language, one can also use the `.hidden` directive
(e.g. see
[here](https://docs.oracle.com/cd/E26502_01/html/E28388/eoiyg.html#:~:text=.hidden%20symbol1%2C%20symbol2%2C%20...%2C%20symbolN)).

It seems that so far a similar feature hasn't yet been introduced to other
languages that compile to native binary code:

* It is unclear if GoLang has a way to explicitly specify visibility.
  Using `#pragma GCC visibility push(hidden)` has been proposed as a workaround
  (see
  [here](https://github.com/golang/go/issues/28340#issuecomment-466645246)).
* Haskell libraries can say
  `foreign export ccall some_function_name :: Int -> Int`
  to export a function (see
  [the Haskell wiki](https://wiki.haskell.org/Foreign_Function_Interface)).
  Presumably such functions are publicly exported
  (just as with Rust's `#[no_mangle]`).
* There is
  [a proposal](https://forums.swift.org/t/current-status-of-swift-symbol-visibility/66949)
  for Swift language to leverage
  [the `package` access modifier](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0386-package-access-modifier.md)
  as a way to specify public visibility.
* There is an open issue that tracks adding a similar mechanism to Zig:
  https://github.com/ziglang/zig/issues/9762

## Rust language

[`#[linkage...]` attribute](https://github.com/rust-lang/rust/issues/29603)
has been proposed in the past for specifying
[linkage type](https://llvm.org/docs/LangRef.html#linkage-types) of a symbol
(e.g. `weak`, `linkonce_odr`, etc.).
Linkage type is related to, but nevertheless different from
[linkage visibility](https://llvm.org/docs/LangRef.html#visibility-styles)
that this RFC focuses on.

The `#[export_visibility = ...]` attribute has been earlier covered by a
Major Change Proposal (MCP) at
https://github.com/rust-lang/compiler-team/issues/881, but it was pointed
out that
"a compiler MCP isn't quite the right avenue here,
as attributes are part of the language."

# Unresolved questions
[unresolved-questions]: #unresolved-questions

There are no unresolved questions at this point.

# Future possibilities
[future-possibilities]: #future-possibilities

## Provide reference-level definitions of supported visibility levels

`#[export = "target_default"]` defers the choice of an actual visibility level
to:

1. Session-wide default of
   [`SymbolVisibility::Interposable`](https://github.com/rust-lang/rust/blob/910617d84d611e9ba508fd57a058c59b8a767697/compiler/rustc_session/src/session.rs#L551-L557)
2. Unless overridden by target platform’s default visibility specified in
   [`rustc_target::spec::TargetOptions`](https://github.com/rust-lang/rust/blob/910617d84d611e9ba508fd57a058c59b8a767697/compiler/rustc_target/src/spec/mod.rs#L2225-L2230),
3. Or overridden by
   [`-Zdefault-visibility=...`](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/default-visibility.html)
   command-line flag.

This means that _this_ RFC doesn't necessarily need to
define the exact semantics and behavior of supported visibility levels.
OTOH, such definitions may be desirable in the future:

* If/when stabilizing `-Zdefault-visibility=...`
* If/when extending `#[export_visibility = ...]` to support specific visibility
  levels (i.e. if the attribute would support not only the `"target_default"`
  visibility value, but also `"hidden"`, `"protected"`, and/or
  `"interposable"`).

One way to provide such definitions would be to map different visibility levels
into specific behavior on the supported Tier 1 platforms.   This can be limited
to documenting the impact for ELF, Mach-O, and PE binaries, because all of
[Tier 1 target triples](https://doc.rust-lang.org/beta/rustc/platform-support.html#tier-1-with-host-tools)
use one of those three binary formats:

* `aarch64-apple-darwin`: MachO (documented
  [here](https://doc.rust-lang.org/beta/rustc/platform-support/apple-darwin.html#binary-format))
* `aarch64-pc-windows-msvc`: PE/COFF (documented
  [here](https://doc.rust-lang.org/beta/rustc/platform-support/windows-msvc.html#platform-details))
* `aarch64-unknown-linux-gnu`: ELF
* `i686-pc-windows-msvc`: PE/COFF (same documentation as above)
* `i686-unknown-linux-gnu`: ELF
* `x86_64-pc-windows-gnu`: PE (documented
  [here](https://doc.rust-lang.org/beta/rustc/platform-support/windows-gnu.html#requirements))
* `x86_64-unknown-linux-gnu`: ELF

Ad-hoc, manual tests (e.g. see
[here](https://github.com/rust-lang/rfcs/pull/3834#issuecomment-3403039933))
of `#[export_visibility = "target_default"]` provide some
reassurance that such definitions should be possible in the future.
OTOH, when future RFCs or PRs consider implementing specific visibility levels,
they should ideally come with:

* Codegen tests that verify how `#[export_visibility = …]` is translated into
  LLVM syntax
* End-to-end tests for 3 platforms that cover ELF, Mach-O, and PE binaries.
  Verification in such tests would most likely have to depend on arbitrary
  developer tools (e.g.
  [`readelf`](https://man7.org/linux/man-pages/man1/readelf.1.html) or
  [`dumpbin`](https://learn.microsoft.com/en-us/cpp/build/reference/dumpbin-reference?view=msvc-170))
  and therefore such tests would most likely have to be
  `make`-based.

## Support hidden visibility

In the future, we may consider supporting `#[export_visibility = “hidden”]`.
In terms of the internal `rustc` APIs this would map to
[`rustc_target::spec::SymbolVisibility::Hidden`](https://github.com/rust-lang/rust/blob/910617d84d611e9ba508fd57a058c59b8a767697/compiler/rustc_target/src/spec/mod.rs#L884).
The hidden visibility would have the following impact on Tier 1 binaries:

* ELF binaries: The symbol is marked
  [`STV_HIDDEN`](https://man7.org/linux/man-pages/man5/elf.5.html#:~:text=specific%20hidden%20class.-,STV_HIDDEN,-Symbol%20is%20unavailable)
* PE binaries: The symbol is non-exported (i.e. the symbol is not listed in
  [the `.edata` section](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#the-edata-section-image-only))
* MachO binaries: The symbol is non-exported (i.e. the symbol is not listed in
  [the export trie](https://github.com/apple-oss-distributions/xnu/blob/8d741a5de7ff4191bf97d57b9f54c2f6d4a15585/EXTERNAL_HEADERS/mach-o/loader.h#L1369))

### Open question: `#[export_visibility = "hidden"]` vs `dylib`s
[hidden-vs-dylibs]: #interaction-between-export_visibility--hidden-vs-dylibs

#### Problem description

https://github.com/rust-lang/rust/issues/73958#issuecomment-2635015556
points out that using `#[export_visibility = "hidden"]` may break some `dylib`
scenarios.

For example, let's say that a crate named `rlib` is compiled into an `rlib` with
the following functions:

```rust
/// Let's say that this is an internal helper that is only intended to be called
/// from code within this library.  To facilitate this, this function is *not*
/// `pub`.
///
/// To also enable calling the helper from a friendly (also internal-only),
/// supporting C/C++ library we may use `#[no_mangle]`.  To keep this function
/// internal and only enable directly calling this helper from statically-linked
/// C/C++ libraries we may /// use `#[export_visibility = "hidden"]`.  We will
/// see below how the hidden visibility may have some undesirable
/// interactions with `dylib`s.
#[no_mangle]
#[export_visibility = "hidden"]
fn internal_helper_called_from_rust_or_cpp() { todo!() }

/// This is a public (`pub`) Rust function - it may be called from other Rust
/// crates.
///
/// This function may internally (say, as an implementation detail) call
/// `fn internal_helper_called_from_rust_or_cpp` above.  If this public function
/// gets inlined into another `dylib` then the call to the internal helper
/// will cross `dylib` boundaries - this will **not** work if the internal
/// helper is hidden from dynamic linking.
#[inline]
pub fn public_function() {
    internal_helper_called_from_rust_or_cpp()
}
```

#### Potential answers

The following options have been identified so far as a potential way for
answering the `dylib`-vs-`hidden`-visibility problem:

* Don't stabilize `#[export_visibility = "hidden"]` (initially? forever?)
* Support `#[export_visibility = "hidden"]`, but
    - Document that `hidden` visibility may break linking of `dylib`s
      (see the "Hidden visibility" section in the guide-level explanation above)
    - Document a recommendation that reusable crates shouldn't use a hardcoded
      visibility
* Avoid inlining if the inlined code ends up calling a hidden symbol from
  another `dylib`
    - Currently preventing inlining is problematic, because `#[inline]` will
      stop the function from being codegened in the original crate unless used
      (hattip
      [@chorman0773](https://github.com/rust-lang/rfcs/pull/3834#issuecomment-3352655525)).
      OTOH, this doesn't necessarily seem like a hard blocker (i.e. maybe this
      behavior can change).
    - Generics also cannot have code generated in the original crate, because
      codegen requires knowing the generic parameters.  But generics seem
      irrelevant here, because `#[export_visibility = ...]` does _not_ apply to
      generics.  In particular, `#[no_mangle]`
      ([Rust
      playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=ac8f26f9b05471c2480b3185388c05e8))
      and `#[export_name = ...]` cannot be used with generics, because the names
      of the symbols (ones generated during monomorphization) need to differ
      based on the generic parameters.
    - One major problem with avoiding inlining is that during codegen it is not
      yet known if two crates will end up getting linked into the same or
      different dylib.  This means that inlining would need to be inhibited for
      any cross-crate calls into hidden symbols.  And this would suppress many
      legitimate optimizations. (hattip
      [@bjorn3](https://github.com/rust-lang/rfcs/pull/3834#issuecomment-3352658642))
* Add a lint/warning that detects when `#[export_visibility = ...]` is used
  inappropriately
    - Sub-idea 1: when a hidden function is called from a caller that may be
      inlined into another crate.  (hattip
      [@tmandry](https://github.com/rust-lang/rfcs/pull/3834#issuecomment-3282373591))
        - This idea is problematic, because using inlineability for restricting
          how source programs are written means committing to implementation
          details of rustc’s codegen strategy.  For example, `rustc` currently
          has some logic to treat small functions as-if they were `#[inline]`
          for codegen purposes even if they weren’t declared as such in the
          source code. (hattip
          [@hanna-kruppe](https://github.com/rust-lang/rfcs/pull/3834#discussion_r2395437679))
    - Sub-idea 2: when a hidden function is called _at all_ from another Rust
      function
        - This seems very drastic, but in practice `#[no_mangle]` are oftentimes
          called only from _another, non-Rust_ language.  This is definitely the
          case for FFI thunks used as one of motivating examples in this RFC.

## Support protected visibility

In the future, we may consider supporting `#[export_visibility = “protected”]`.

Open question:

* Need to clarify how `protected` vs `interposable` visibilities would work for
  Tier 1 platforms.  In particular, it seems that PE and Mach-O binary formats
  may not be able to distinguish between `protected` and `interposable`
  visibilities (the latter is the default when a `#[no_mangle]` symbol is not
  accompanied by `#[export_visibility = ...]`).
