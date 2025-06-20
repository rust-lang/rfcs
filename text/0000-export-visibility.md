- Feature Name: `export_visibility`
- Start Date: 2025-06-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Documentation of
[`#[no_mangle]`](https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute)
points out that by default a `#[no_mangle]` function (or `static`)
will be "publicly exported from the produced library or object file".
This RFC proposes a new `#[export_visibility = ...]` attribute
to override this behavior.
This means that if the same `#[no_mangle]` function is also
decorated with `#[export_visibility = "inherit"]`,
then it will instead inherit the default visibility of the target platform
(or the default visibility specified with the
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

## Benefit: Avoiding undefined behavior

Documentation of
[`#[export_name = ...]`](https://doc.rust-lang.org/reference/abi.html#the-export_name-attribute)
points out that "this attribute is unsafe as a symbol with a custom name may
collide with another symbol with the same name (or with a well-known symbol),
leading to undefined behavior".  Similar unsafety risk is present for the
[`#[no_mangle]`](https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute)
attribute.  Unnecessary public exports increase the scope of this risk.

A more concrete example of this problem is related to a memory allocator.
A memory allocator may use some global or per-thread data structures to
manage active allocations.  Each
[Dynamic Shared Object (DSO)](https://en.wikipedia.org/wiki/Dynamic_shared_object)
can use a different allocator library (and even if using the same allocator,
it can use separate, DSO-specific global data structures).
Therefore taking an allocation made in one DSO
and freeing it in another DSO can lead to memory unsafety
(when the freeing allocator expects that the pointer it got was earlier
allocated by the same allocator instance).

This is what happened in https://crbug.com/418073233.  In the smaller repro
from https://crrev.com/c/6580611/1 we see the following:

* Without this RFC the [`cxx`](https://cxx.rs) library cannot avoid publicly
  exporting symbols that are called from C++.  In particular, the following
  two symbols are publicly exported from a static library called `rust_lib`:
    - `rust_lib$cxxbridge1$get_string` (generated by `#[cxx::bridge]` proc macro
      to generate an FFI/C-ABI-friendly thunk for
      [`rust_lib::get_string`](https://chromium-review.googlesource.com/c/chromium/src/+/6580611/1/build/rust/tests/test_unexpected_so_hop_418073233/src/lib.rs)
    - `cxxbridge1$string$drop` exported from
      [`cxx/src/symbols/rust_string.rs`](https://github.com/dtolnay/cxx/blob/07d2bca38b7bfbbe366a9e844d3d66b80820d339/src/symbols/rust_string.rs#L83C18-L86)
* In the repro case, `rust_lib` is statically linked into the main test
  executable, and into an `.so`.
    - The `.so` statically links `rust_lib`, but doesn't actually use it.
      (In the original repro the `.so` used a small part of a bigger statically linked
      "base" library and also didn't actually use Rust's `cxx`-related symbols.  See
      https://crrev.com/c/6504932 which removes the `.so`'s dependency on the
      "base" library as a workaround for this problem.)
    - The test executable calls `rust_lib::get_string` and then
      `cxxbridge1$string$drop`.
* This scenario leads to memory unsafety when:
    - The call from test executable to `rust_lib::get_string` ends up calling
      `dso!rust_lib::get_string` rather than `exe!rust_lib::get_string`.
    - The call from test executable to `cxxbridge1$string$drop` ends up
      calling `exe!cxxbridge1$string$drop`.
    - This means that the `exe`'s allocator tries to free an allocation made
      by the allocator from the `dso`.  In debug builds this is caught by an
      assertion.  In release builds this would lead to memory unsafety.

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

### Interposable visibility

`#[export_visibility = "interposable"]` will cause symbols to be emitted with
"default" visibility. On platforms that support it, this makes it so that
symbols can be interposed, which means that they can be overridden by symbols
with the same name from the executable or by other shared objects earlier in the
load order.

> **Note**:
> See [interposable-vs-llvm] section below for discussion about an open
> question that asks about interactions between `interposable` visibility
> and LLVM optimization passes.

> **Note**:
> See [interposable-vs-dllexport] section below for discussion whether
> this visibility should also inject `dllexport` when targeting Windows
> platform.

> **TODO**: This section (as well as `protected` and `hidden` sections below) is based on
> https://doc.rust-lang.org/beta/unstable-book/compiler-flags/default-visibility.html#interposable
> In the long-term we should deduplicated these docs/definitions (for example
> description of `hidden` in this RFC is a bit expanded and brings up additional
> benefits of hiding symbols).  "long-term" probably means: 1) once this or the
> other feature have been stabilized and/or 2) once we are confident with names,
> behavior, etc of all the visibility levels.

### Protected visibility

<!-- This section is based on
https://doc.rust-lang.org/beta/unstable-book/compiler-flags/default-visibility.html#protected-visibility
-->

`#[export_visibility = "protected"]` signals to the compiler, the linker, and
the runtime linker that the symbol cannot be overridden by the executable or by
other shared objects earlier in the load order.

This allows the compiler to emit direct references to symbols, which may improve
performance. It also removes the need for these symbols to be resolved when a
shared object built with this option is loaded.

Using protected visibility when linking with GNU `ld` prior to 2.40 will result
in linker errors when building for Linux. Other linkers such as LLD are not
affected.

### Hidden visibility

<!-- This section is based on
https://doc.rust-lang.org/beta/unstable-book/compiler-flags/default-visibility.html#hidden-visibility,
but it was expanded to point out additional benefits and risks of hidden visibility.

TODO: copy those benefits to the `-Zdefault-visibility=...` docs?  Or move them
to a shared location somewhere?
-->

`#[export_visibility = "hidden"]` marks the symbol as hidden.
Hidden symbols will not be exported from the created shared object, so cannot be
referenced from other shared objects or from executables.

<!--
The claim of reduced runtime overhead is based on
https://gcc.gnu.org/wiki/Visibility
-->
Similarly to protected visibility, hidden visibility may allow the compiler
to improve performance of the generated code by
emitting direct references to symbols.
And it may remove the runtime overhead of linking these symbols at runtime.

<!-- The claim about extra LTO opportunities is based on
https://github.com/rust-lang/rust/issues/73958#issue-649745016.
The claim about the reduced safety risk is based on https://crbug.com/418073233.
-->
Unlike protected visibility, hidden visibility may also enable additional inlining
during Link Time Optimization (LTO), which may be especially important for small
functions (thunks) used for cross-language calls.  It may also limit the scope
of the safety risk of having 2 symbols with the same name.

<!-- The dylib problems caused by hidden visibility have been pointed out in
https://github.com/rust-lang/rust/issues/73958#issuecomment-2635015556 -->
`hidden` visibility should *not* be used when it is not possible to control
whether the symbol may be referenced from another shared object.  For example,
`hidden` visibility should be avoided when building `dylib`s, because
cross-`dylib` inlining may lead to linking errors.

> **Note**:
> See [hidden-vs-dylibs] section below for more discussion on what to do
> about the interaction between `dylibs` and this RFC.

### Inherited visibility

`#[export_visibility = "inherit"]` uses
the standard visibility of the target platform.

Note: the nightly version of the `rustc` compiler
supports overriding the target platform's visibility with the
[`-Zdefault-visibility=...`](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/default-visibility.html)
command-line flag.

### Choosing the right visibility

If you control the linking process (i.e. you control how your symbols are linked
into an executable, or into a `cdylib`, `so` or `dll`), then you should use the
lowest possible visibility.  If a public export is not needed, then use the
`hidden` visibility.  Otherwise consider using `protected` or `interposable`
visibility.

If you are an author of a reusable crate, then you don't know how users of your
crate will link it into executables, `cdylib`s, `dylib`s, etc.  In this case it
is best to give control over visibility of your symbols to your clients by using
`#[export_visibility = "inherit"]`.  Alternatively (e.g. if you provide a proc
macro to generate the exported symbols) you can consider parametrizing the
behavior of your crate to let your clients specify the exact visibility that
your library will declare through the `#[export_visibility = ...]` attribute.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

It seems that the guide-level explanation above may also work as a
reference-level explanation.  (At least, the reference documentation of
[`#[no_mangle]`](https://doc.rust-lang.org/reference/abi.html#the-no_mangle-attribute)
and
[`#[export_name = ...]`](https://doc.rust-lang.org/reference/abi.html#the-export_name-attribute)
attributes provides a similar level of details.)

A few additional notes attempt to clarify the intended behavior of the proposed
behavior beyond what is described in the guide-level explanation above:

* The `#[export_visibility = ...]` attribute may only be applied to item
  definitions with an "extern" indicator as checked by
  [`fn contains_extern_indicator`](https://github.com/rust-lang/rust/blob/3bc767e1a215c4bf8f099b32e84edb85780591b1/compiler/rustc_middle/src/middle/codegen_fn_attrs.rs#L174-L184).
  Therefore it may only be applied to items to which
  `#[no_mangle]`, `#[export_name = ...]`, and similar already-existing
  attributes may be already applied.
* The proposal in this RFC has been prototyped in
  https://github.com/anforowicz/rust/tree/export-visibility

# Drawbacks
[drawbacks]: #drawbacks

See "Open questions" section.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

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
  Instead, MSVC uses `.def` (module-definition) files to control symbol export and
  other aspects of DLL creation.  Having to use
  [a `.def` file](https://learn.microsoft.com/en-us/cpp/build/reference/exports?view=msvc-170)
  has a few extra downsides compared to a version script:
    - Having to support both formats
    - Lack of support for wildcards means that it is impossible to hide
      all symbols matching a pattern like `*cxxbridge*` used by `cxx` in
      auto-generated FFI thunks.
* Using a version script is one way of fixing https://crbug.com/418073233.
  This fix approach requires that authors of each future shared library know
  about the problem and use a version script.  This is in contrast to using
  `-Zdefault-visibility=hidden` and `#[export_visibility = "inherit"]` for `cxx`
  symbols, which has to be done only once to centrally, automatically avoid the
  problem for all `cxx`-dependent libraries in a given build environment.
  (In fairness, using the command-line flag also requires awareness and opt-in,
  but it seems easier to append `-Zdefault-visibility=hidden` to default
  `rustflags` in globally-applicable build settings than it is to modify build
  tools to require a linker script for all shared libraries.  In fact, Chromium
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
`#[export_visibility = "interposable"]` to
`#[export_visibility = "inherit"]`
(which combined with `-Zdefault-visibility=hidden` should address
https://crbug.com/418073233).

It seems that the new edition behaviors proposed above would still benefit from
having a way to diverge from the default visibility behavior of `#[no_mangle]`
symbols.  And therefore it seems that the `#[export_visibility = ...]` attribute
proposed in this RFC would be useful not only in the current Rust edition,
but also in the hypothetical future Rust edition.

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
  (see [here](https://github.com/golang/go/issues/28340#issuecomment-466645246)).
* Haskell libraries can say `foreign export ccall some_function_name :: Int -> Int`
  to export a function (see
  [the Haskell wiki](https://wiki.haskell.org/Foreign_Function_Interface)).
  Presumably such functions are publicly exported
  (just as with Rust's `#[no_mangle]`).
* There is
  [a proposal](https://forums.swift.org/t/current-status-of-swift-symbol-visibility/66949)
  for Swift language to leverage
  [the `package` access modifier](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0386-package-access-modifier.md)  as a way to specify public visibility.
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

## Interaction between `#[export_visibility = "hidden"]` vs `dylib`s
[hidden-vs-dylibs]: #interaction-between-export_visibility--hidden-vs-dylibs

### Problem description

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
///
/// TODO: Do we need `rustc` command-line examples that would show how such
/// static linking would be done when building a `staticlib`, `bin`, `cdylib`,
/// or a `dylib` (I haven't checked how/if this would work in all of those
/// cases;  i.e. I haven't checked how to ask `rustc` to link with static
/// libraries produced by a C/C++ compiler).
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

### Potential answers

The following options have been identified so far as a potential way for
answering the `dylib`-vs-`hidden`-visibility problem:

* Don't stabilize (or don't support at all) `#[export_visibility = "hidden"]`
  but still support other visibilities
* Support `#[export_visibility = "hidden"]`, but
    - Document that `hidden` visibility may break linking of `dylib`s
      (see the "Hidden visibility" section in the guide-level explanation above)
    - Document a recommendation that reusable crates shouldn't use a hardcoded
      visibility (see the "Choosing the right visibility" section in the
      guide-level explanation above)
* Investigate if cross-`dylib`-inlining can (should?) be avoided if the inlined
  code ends up calling a hidden symbol from the other crate.

## Cross-platform behavior

We don't really know
whether the `hidden` / `protected` / `interposable` visibilities
make sense across different target platforms and/or map to distinct entities
(see
[a Zulip question here](https://rust-lang.zulipchat.com/#narrow/channel/233931-t-compiler.2Fmajor-changes/topic/.60.23.5Bexport_visibility.20.3D.20.2E.2E.2E.5D.60.20attribute.20compiler-team.23881/near/522491140)).

One weak argument is that these visibilities are supported by LLVM and Clang, so hopefully
they would also make sense for Rust:

* **LLVM**: Those visibilities are ultimately mapped from
[`rustc_target`'s `SymbolVisibility`](https://github.com/rust-lang/rust/blob/81a964c23ea4fe9ab52b4449bb166bf280035797/compiler/rustc_target/src/spec/mod.rs#L839-L843),
through
[`rustc_middle`'s `Visibility`](https://github.com/rust-lang/rust/blob/81a964c23ea4fe9ab52b4449bb166bf280035797/compiler/rustc_middle/src/mir/mono.rs#L396-L407),
and into
[`rustc_codegen_llvm`'s `Visibility`](https://github.com/rust-lang/rust/blob/81a964c23ea4fe9ab52b4449bb166bf280035797/compiler/rustc_codegen_llvm/src/llvm/ffi.rs#L153-L160).
So all the values make some sense at
[the LLVM level](https://llvm.org/docs/LangRef.html#visibility-styles).
* **Clang** and **GCC** support those 3 visibilities
  (see the "Parity with C++" subsection in the "Motivation" section above).

OTOH, ideally we would somehow check what happens on some representative subset
of target platforms (maybe: Posix, Windows, Wasm?):

* TODO: what exactly do we want to verify on these target platforms?

## Rust standard library

### Problem description

The scope of this RFC is currently limited to just introducing the
`#[export_visibility = ...]` attribute.  This should help realize the
benefits described by this RFC wherever the new attribute is used
(even if there remain places where the new attribute is not used).
OTOH this means that this RFC treats
visibility of Rust standard library symbols
as out of scope.

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

### Potential answers

The following options have been identified so far as a potential way for
hiding symbols coming from Rust standard library:

* Do nothing.
    - Hiding symbols would require rebuilding Rust standard library with
      `-Zdefault-visibility=hidden`.
    - Note that there are other valid reasons
      for rebuilding the standard library when building a given project.
      For example this is a way to use globally consistent `-C` options
      like `-Cpanic=abort`,
      [`-Clto=no`](https://source.chromium.org/chromium/chromium/src/+/main:build/config/compiler/BUILD.gn;l=1115-1118;drc=26d51346374a0d16b0ba2243ef83c015a944d975),
      etc.
    - Rebuilding the standard library is possible,
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
* Alternative: change the semantics of `#[rustc_std_internal_symbol]`
    - Drawback: On its own this wouldn't affect
      visibility of non-`#[rustc_std_internal_symbol]` symbols
      like `String::new`.

## Windows and `__declspec(dllexport)`
[interposable-vs-dllexport]: #windows-and-__declspecdllexport

We need to decide whether `#[export_visibility = "interposable"]` should also
result in `__declspec((dllexport))` being added to a symbol.  See for example
[this Stack Overflow question and answer](https://stackoverflow.com/a/25746044/24042981).

Potential answers:

* Don't stabilize for now (or don't support at all)
  `#[export_visibility = "interposable"]` but still support other visibilities
* `#[export_visibility = "interposable"]` should only control visibility
* `#[export_visibility = "interposable"]` should control visibility
  and also use `__declspec(dllexport)`

## Interposability vs LLVM optimization passes
[interposable-vs-llvm]: #interposability-vs-llvm-optimization-passes

This RFC proposes to use `interposable` to map to
[`SymbolVisibility::Interposable`](https://github.com/rust-lang/rust/blob/81a964c23ea4fe9ab52b4449bb166bf280035797/compiler/rustc_target/src/spec/mod.rs#L842)
which is then mapped to
[`llvm::Visibility::Default`](https://github.com/rust-lang/rust/blob/81a964c23ea4fe9ab52b4449bb166bf280035797/compiler/rustc_codegen_llvm/src/llvm/ffi.rs#L167).  This mimics how `interposable` is implemented and supported
in
[`-Zdefault-visibility=...`](https://doc.rust-lang.org/beta/unstable-book/compiler-flags/default-visibility.html).

One problem here is that `llvm::Visibility::Default` is not sufficient to
achieve actual interposability.  https://crbug.com/418073233 has one example of
undefined behavior, but even if DSO-local global data structures were not an
issue, then LLVM-level assumptions could still lead to undefined behavior.
This is because the LLVM optimization passes assume that a symbol with normal
external linkage (not weak, odr, etc) the definition it can see is the
definition that will be actually used.  To avoid these LLVM assumptions `rustc`
would have to enable
[the SemanticInterposition feature](https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fsemantic-interposition).

Special thanks to @jyknight for pointing out this concern.

Potential answers:

* Don't stabilize for now (or don't support at all)
  `#[export_visibility = "interposable"]` but still support other visibilities
* Rename `interposable` to `public` or `default`.
  (It is quite unfortunate that `default` is an overloaded term and
  may be potentially confused with the `inherit` behavior.)

# Future possibilities
[future-possibilities]: #future-possibilities

Couldn't think of anything so far.
