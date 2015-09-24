- Feature Name: `link_alias`
- Start Date: 2015-09-24
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Add a new `alias` attribute to `#[link]` and the `-l` flag which indicates that
the linkage will happen through another annotation to inform how a library is
linked. This is then leverage to inform the compiler about dllimport and
dllexport with respect to native libraries on the MSVC platform.

# Motivation

Most of the time a linkage directive is only needed to inform the linker about
what native libraries need to be linked into a program. On some platforms,
however, the compiler needs more detailed knowledge about what's being linked
from where in order to ensure that symbols are wired up correctly.

For example, on MSVC, if a symbol imported from a native library is actually
imported from a DLL, it is linked to differently than if it's imported from a
native library that's linked statically. In linkage terms, importing a function
from a DLL requires the compiler to tag the import with "dllimport", an
attribute to LLVM, in order for the native library to be linked correctly.

Currently the compiler is not able to correctly place dllimport annotations on
imports from native libraries as it has no knowledge of whether a symbols is
being imported from a DLL or not. Blocks of symbols (those wrapped in an
`extern` directive) may be tagged with `#[link]` in which case the compiler
could reasonbly infer where the symbols come from, but not all `extern` blocks
are always tagged as such:

* Many native libraries are linked via the command line via `-l` which is passed
  in through Cargo build scripts instead of being written in the source code
  itself. As a recap, a native library may change names across platforms or
  distributions or it may be linked dynamically in some situations and
  statically in others which is why build scripts are leveraged to make these
  dynamic decisions.
* Many `extern` blocks are empty to have the `#[link]` directives located
  elsewhere either for convenience or for organizational purposes.

Another example of where the compiler needs more knowledge about how to deal
with symbols from native libraries (specifically on MSVC) is that any symbol
exported from a DLL also needs to be tagged as such. This specifically comes up
whenever the compiler produces a dylib which contains some statically linked
native libraries. If any of the native libraries' symbols are reachable via the
public API, then the symbols need to be tagged with dllexport.

Similar to the dllimport situation, the compiler cannot currently reason about
the connection between symbols and what libraries they came from, so the
compiler isn't able to place dllexport annotations to ensure that symbols are
exported correctly.

Overall, the common motivation of these scenarios is that the compiler does not
always have a connection between a set of symbols from a native library and
which native library they came from. By enabling the compiler to have this
knowledge in all situations the it can automatically handle dllexport/dllimport
on MSVC and perhaps do more-clever things on Unix in the future.

# Detailed design

Two changes will be made to the compiler, the first to add an `alias` kind to
native libraries and the second is how treatment of native libraries with
respect to dllimport and dllimport will change.

### Adding `alias`

The addition of aliases is handled in two separate locations, the `#[link]`
attribute and the `-l` flag. At a high level, each of these will be able to
introduce a named alias for the library also being linked, and the attribute
form `#[link]` will be able to reference these aliases. First, let's look of the
way to introduce an alias for a library being linked.

#### Introducing an alias

First, the existing `#[link]` attribute and `-l` flag will introduce an alias of
the same name as the name of the library being linked. Both of the following
forms will introduce an alias of the name "foo" pointing to the native library
"foo":

```rust
#[link(name = "foo")]
```

```
-l foo
```

The purpose of `alias`, however, will be to introduce a name that's not exactly
the same as the native library's name (because the native library could have
different names across platforms/distributions). The `#[link]` attribute will be
extended with an `alias` key as well as the `-l` flag being extended:

```rust
#[link(name = "bar", alias = "foo")]
```

```
rustc -l alias=foo=bar
```

(note that `-l` argument looks like this Cargo build script)

```rust
fn main() {
    println!("cargo:rustc-link-lib=alias=foo=bar");
}
```

These alias forms mean the dynamic library "bar" is linked but also introduces
an alias "foo" for the library. Note that in all of these cases an optional
`kind` can also be specified:

```rust
#[link(name = "bar", alias = "foo", kind = "static")]
```

```
rustc -l alias=foo=static=bar
```

With these introduction forms the compiler now has a mapping from all native
libraries being linked to a set of aliases that library is known under. The
compiler will use this to construct a mapping from alias name to native library
for use in the next section.

#### Using an alias

Now that we've established aliases for all native libraries being linked as part
of a compilation, the compiler will also support a `#[link]` attribute of the
form:

```rust
#[link(alias = "foo")]
```

The compiler will resolve the alias name "foo" to a native library using the
mapping built up from the introduction forms above. For example this attribute
above would be connected with a directive or a flag that looked like:

```rust
#[link(name = "foo")]                  // implicit alias is called "foo"
#[link(name = "bar", alias = "foo")]   // explicitly aliased as `foo`
```

```
rustc -l foo            # implicit alias is called "foo"
rustc -l alias=foo=bar  # "bar" is explicitly aliased as "foo"
```

A `#[link(alias = "...")]` annotation is required to resolve to some native
library, and an error will be generated if the alias has not been introduced.
The culmination of this is now the compiler can connect an alias directive to a
native library to understand that the symbols in the `extern` block are
contained in that native library.

#### Alias examples

Some example usage of introducing aliases and then using them looks like:

```rust
// compiled with: rustc -l alias=a1=d1

#[link(name = "lib1")]
extern {}

// aliases the library defined above
#[link(alias = "lib1")]
extern {}

// aliases the library `d1` on the command line
#[link(alias = "a1")]
extern {}

// also aliases the library `d1` on the command line
#[link(alias = "d1")]
extern {}

// introduce the alias `a2` for `lib2
#[link(name = "lib2", alias = "a2")]
extern {}

// aliases the library `lib2` above
#[link(alias = "a2")]
extern {}
```

### Treatment of dll{import,export}

As a recap, let's take a look at today's treatment of dll{import,export} in the
compiler with respect to native libraries. Note that the terms "functions" and
"static" here refer to those defined in native libraries (e.g. connected via
FFI). Currently dllimport is only applied when a static is referenced through an
external crate. References to locally declared statics or functions in any
circumstance never have dllimport applied. The compiler also currently has an
unstable `#[linked_from]` attribute which it leverages to apply the dllexport
annotation to statically linked native libraries, but otherwise dllexport is
never applied. This treatment is incorrect in [a number of ways][issue], and is
a strong motivating factor for this RFC!

[issue]: https://github.com/rust-lang/rust/issues/27438

Armed with `alias` directives, the compiler is now able to handle
dllimport/dllexport correctly in all cases for native libraries. The dllimport
attribute will be applied to all symbols in an `extern` block if that block has
any linkage directive indicating that the symbols are linked via a dynamic
library. (e.g. following alias pointers to their concrete linkage directives).
Similarly, dllexport will only be applied to a block of symbols if a directive
indicates that they're linked statically.

Example application of dllexport/dllimport looks like:

```rust
// compiled with: rustc -l l1

#[link(alias = "l1")]
extern {
    // dllimport applied, dllexport not applied
}

#[link(name = "l2")]
extern {
    // dllimport applied, dllexport not applied
}

#[link(name = "l3", kind = "static")]
extern {
    // dllimport not appplied, dllexport applied if linked staticaly to dylib
}

extern {
    // dllimport not applied, dllexport not applied
}
```

# Drawbacks

For libraries to work robustly on MSVC, the correct `#[link]` annotation will
be required. Most cases will "just work" on MSVC due to the compiler strongly
favoring static linkage, but any symbols imported from a dynamic library or
exported as a Rust dynamic library will need to be tagged appropriately to
ensure that they work in all situations. Worse still, the `#[link]` annotations
on an `extern` block are not required on any other platform to work correctly,
meaning that it will be common that these attributes are left off by accident.

Unfortunately, however, there doesn't seem to be a "zero annotation" solution to
the dllimport/dllexport problem on MSVC (e.g. even C libraries need to annotate
correctly). Given that annotations are required in *some* form or another, the
solution here is relatively lightweight and easy to add backwards compatibly to
libraries desiring MSVC support.

Another drawback is that the CLI syntax is a little wonky with three `=`
characters in some situations.

# Alternatives

* Instead of enhancing `#[link]`, a `#[linked_from = "foo"]` annotation could
  replace `#[link(alias = "foo")]` without supporting `alias` in `#[link]` or
  `-l`. This has the drawback of not being able to handle native libraries whose
  name is unpredictable across platforms in an easy fashion, however.
  Additionally, it adds an extra attribute to the comipler that wasn't known
  previously.

* Instead having a desire to connect symbols to libraries, the compiler could
  instead simply support a `#[dllimport]` and `#[dllexport]` attribute both for
  native symbols. These would directly correspond to the respective attributes
  and the burden of deciding when to apply them would be on the author instead
  of the compiler. This has a number of drawbacks, however:

    * The annotation burden here is much higher than with `alias` as an
      attribute is needed per-function.
    * It's not always known whether `#[dllexport]` is needed. If a native
      library is statically linked into an rlib then that rlib could later
      either become an executable or a DLL itself. In the executable case
      `dllexport` needs to not be applied, but in the DLL case it may need to be
      applied (if the symbol is reachable). Handling all this logic is possible
      from a crate author's perspective, but it would be quite tedious to
      replicate this logic across all crates in the ecosystem.
    * Similarly, it's not always known whether `#[dllimport]` is needed. Native
      libraires are not always known whether they're linked dynamically or
      statically (e.g. that's what a build script decides), so setting up the
      build script to enable the crate to conditionally emit `dllimport` has an
      even higher annotation burden than just applying `#[dllimport]` itself.

  Overall, it appears all usage of manyual dllimport/dllexport can be encoded
  via `alias` which has a much smaller annotation burden and is much more robust
  in the face of dllexport particularly (e.g. only the compiler really knows
  whether the symbols are reachable or not) but also dllimport (auto applying or
  not applying depending on how the library is linked).

* When linking native libraries, the compiler could attempt to locate each
  library on the filesystem and probe the contents for what symbol names are
  exported from the native library. This list could then be cross-referenced
  with all symbols declared in the program locally to understand which symbols
  are coming from a dylib and which are being linked statically. Some downsides
  of this approach may include:

    * It's unclear whether this will be a performant operation and not cause
      undue runtime overhead during compiles.

    * On Windows linking to a DLL involves linking to its "import library", so
      it may be difficult to know whether a symbol truly comes from a DLL or
      not.

    * Locating libraries on the system may be difficult as the system linker
      often has search paths baked in that the compiler does not know about.

# Unresolved questions

What does the transition plan for crates look like with these attributes? Today
the compiler's liberal application of dllimport to statics enables many crates
to link correctly, but once this change is implemented that will no longer be
the case. If a crate wants to work on stable it cannot use
`#[link(alias = "foo")]` and on nightly it *must* use it if no other `#[link]`
directive is applied. Is it the case that in this situation `#[link]` is already
applied with `kind = "dylib"`?

