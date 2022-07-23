- Feature Name: `native_link_modifiers`
- Start Date: 2020-06-12
- RFC PR: [rust-lang/rfcs#2951](https://github.com/rust-lang/rfcs/pull/2951)
- Rust Issue: [rust-lang/rust#81490](https://github.com/rust-lang/rust/issues/81490)

# Summary
[summary]: #summary

Provide an extensible mechanism for tweaking linking behavior of native libraries
both in `#[link]` attributes (`#[link(modifiers = "+foo,-bar")]`)
and on command line (`-l static:+foo,-bar=mylib`).

# Motivation
[motivation]: #motivation

Occasionally some tweaks to linking behavior of native libraries are necessary,
and currently there's no way to apply them.

For example, some static libraries may need to be linked as a whole archive
without throwing any object files away because some symbols in them appear to be unused
while actually having some effect. \
This RFC introduces modifier `whole-archive` to address this.

In other cases we need to link to a library located at some specific path
or not matching the default naming conventions. \
This RFC introduces modifier `verbatim` to pass such libraries to the linker.

This RFC also reformulates the `static-nobundle` linking kind as a modifier `bundle`
thus providing an opportunity to change the static linking default to non-bundling
on some future edition boundary, and hopefully unblocking its stabilization.

The generic syntax provides a way to add more such modifiers in the future
without introducing new linking kinds.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This is an advanced feature not expected to be used commonly,
see the reference-level explanation if you know that you need some of these modifiers.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Existing syntax of linking attributes and options

- Attributes: `#[link(name = "string", kind = "string", cfg(predicate))]`
(some components are optional.)
- Command line options: `-l kind=name:rename` (some components are optional).

## Proposed extensions to the syntax

- Attributes: `#[link(/* same */, modifiers = "+foo,-bar")]`.
- Command line options: `-l kind:+foo,-bar=name:rename`.

The modifiers are boolean and applied only to the single library specified with `name`. \
`+` means enable, `-` means disable, multiple options can be separated by commas,
the last boolean value specified for the given modifier wins. \
The notation is borrowed from
[target features](https://doc.rust-lang.org/rustc/codegen-options/index.html#target-feature)
in general and should have the same semantics.

If the `:rename` component is specified on the command line, then in addition to the name
and linking kind the modifiers will be updated as well (using concatenation).

## Specific modifiers

### `bundle`

Only compatible with the `static` linking kind.

`+bundle` means objects from the static library are bundled into the produced crate
(a rlib, for example) and are used from this crate later during linking of the final binary.

`-bundle` means the static library is included into the produced rlib "by name"
and object files from it are included only during linking of the final binary,
the file search by that name is also performed during final linking.

This modifier is supposed to supersede the `static-nobundle` linking kind defined by
[RFC 1717](https://github.com/rust-lang/rfcs/pull/1717).

The default for this modifier is currently `+bundle`,
but it could be changed later on some future edition boundary.

### `verbatim`

`+verbatim` means that `rustc` itself won't add any target-specified library prefixes or suffixes
(like `lib` or `.a`) to the library name,
and will try its best to ask for the same thing from the linker.

For `ld`-like linkers `rustc` will use the `-l:filename` syntax (note the colon)
when passing the library, so the linker won't add any prefixes or suffixes as well. \
See [`-l namespec`](https://sourceware.org/binutils/docs/ld/Options.html) in `ld` documentation
for more details. \
For linkers not supporting any verbatim modifiers (e.g. `link.exe` or `ld64`)
the library name will be passed as is.

The default for this modifier is `-verbatim`.

This RFC changes the behavior of `raw-dylib` linking kind specified by
[RFC 2627](https://github.com/rust-lang/rfcs/pull/2627).
The `.dll` suffix (or other target-specified suffixes for other targets)
is now added automatically. \
If your DLL doesn't have the `.dll` suffix, it can be specified with `+verbatim`.

### `whole-archive`

Only compatible with the `static` linking kind.

`+whole-archive` means that the static library is linked as a whole archive
without throwing any object files away.

This modifier translates to `--whole-archive` for `ld`-like linkers,
to `/WHOLEARCHIVE` for `link.exe`, and to `-force_load` for `ld64`. \
The modifier does nothing for linkers that don't support it.

The default for this modifier is `-whole-archive`.

A motivating example for this modifier can be found in
[issue #56306](https://github.com/rust-lang/rust/issues/56306).

### `as-needed`

Only compatible with the `dynamic` and `framework` linking kinds.

`+as-needed` means that the library will be actually linked only if it satisfies some
undefined symbols at the point at which it is specified on the command line,
making it similar to static libraries in this regard.

This modifier translates to `--as-needed` for `ld`-like linkers,
and to `-dead_strip_dylibs` / `-needed_library` / `-needed_framework` for `ld64`. \
The modifier does nothing for linkers that don't support it (e.g. `link.exe`).

The default for this modifier is unclear, some targets currently specify it as `+as-needed`,
some do not. We may want to try making `+as-needed` a default for all targets.

A motivating example for this modifier can be found in
[issue #57837](https://github.com/rust-lang/rust/issues/57837).

## Stability story

The modifier syntax can be stabilized independently from any specific modifiers.

All the specific modifiers start unstable and can be stabilized independently from each other
given enough demand.

## Relative order of `-l` and `-Clink-arg(s)` options

This RFC also proposes to guarantee that the relative order of `-l` and `-Clink-arg(s)`
command line options of `rustc` is preserved when passing them to linker. \
(Currently they are passed independently and the order is not guaranteed.)

This provides ability to tweak linking of individual libraries on the command line
by using raw linker options. \
An equivalent of order-preserving `-Clink-arg`, but in an attribute form,
is not provided at this time.

# Drawbacks
[drawbacks]: #drawbacks

Some extra complexity in parsing the modifiers
and converting them into a form suitable for the linker.

Not all modifiers are applicable to all targets and linkers,
but that's true for many existing `-C` options as well.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives


## Alternative: rely on raw linker options

The primary alternative for the (relatively cross-platform) `whole-archive` and `as-needed`
modifiers is to rely on more target-specific raw linker options more.

(Note, that raw linker options don't cover the `bundle` and `verbatim` modifiers
that are `rustc`-specific.)

The modifier support is removed from the command line options,
the desired effect is achieved by something like this.
```sh
-Clink-arg=-Wl,--whole-archive -lfoo -Clink-arg=-Wl,--no-whole-archive
```

Note the `-Wl,` that is needed when using `gcc` as a linker,
but not when using an `ld`-like linker directly.
So this solution is not only more target-specific, but also more linker specific as well.

The `-Wl,` part can potentially be added automatically though, there's some prior art from CMake
regarding this, see the `LINKER:` modifier for
[`target_link_options`](https://cmake.org/cmake/help/git-stage/command/target_link_options.html).

Relying on raw linker options while linking with attributes will requires introducing
a new attribute, see the paragraph about `#[link(arg = "string")]` in "Future possibilities".

## Alternative: merge modifiers into kind in attributes

`#[link(kind = "static", modifiers = "+foo,-bar")]` -> `#[link(kind = "static:+foo,-bar")]`.

This make attributes closer to command line, but it's unclear whether it's a goal we want to pursue.
For example, we already write `kind=name` on command line,
but `kind = "...", name = "..."` in attributes.

# Prior art
[prior-art]: #prior-art

`gcc` provides the `-Wl,foo` command line syntax (and some other similar options) for passing
arbitrary options directly to the linker.

The relative order of `-Wl` options and `-l` options linking the libraries is preserved.

`cl.exe` provides `/link link-opts` for passing options directly to the linker,
but the options supported by `link.exe` are generally order-independent,
so it is not as relevant to modifying behavior of specific libraries as with `ld`-like linkers.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

None currently.

# Future possibilities
[future-possibilities]: #future-possibilities

## New modifiers

### `dedup`

`rustc` doesn't currently deduplicate linked libraries
[in general](https://github.com/rust-lang/rust/issues/73319).

The reason is that *sometimes* the linked libraries need to be duplicated on the command line.

However, such cases are rare and we may want to deduplicate the libraries by default,
but provide the `-dedup` modifier as an opt-out for these rare cases.

Introducing the `dedup` modifier with the current `-dedup` default doesn't make much sense.

## Support `#[link(arg = "string")]` in addition to the modifiers

`ld` supports some other niche per-library options, for example `--copy-dt-needed-entries`.

`ld` also supports order-dependent options like `--start-group`/`--end-group`
applying to groups of libraries.

We may want to avoid new modifiers for all possible cases like this and provide an order-preserving
analogue of `-C link-arg`, but in the attribute form. \
It may also resolve issues with the existing unstable attribute
[`#[link_args]`](https://github.com/rust-lang/rust/issues/29596)
and serve as its replacement.

Some analogue of
[CMake's `LINKER:`](https://cmake.org/cmake/help/git-stage/command/target_link_options.html)
mentioned above can improve portability here.
