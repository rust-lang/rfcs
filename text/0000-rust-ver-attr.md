- Start Date: 2014-11-04
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC proposes adding a `#![rust_ver="..."]` attribute and `--rust-ver` compiler flag.

## Context

- [Issue #3392: language version markers](https://github.com/rust-lang/rust/issues/3392) (Open).
- [Issue #3795: Need a mechanism to write rust-version-specific code](https://github.com/rust-lang/rust/issues/3795) (Open).

# Motivation

There are several major motivations for this addition:

- Giving users clearer, more accurate information when attempting to compile code that is unsupported by their current compiler.
- Giving the package ecosystem a formal way of dealing with language version dependencies.
- Smoothing the transition to a future, backward-incompatible release of the language.

## Clearer Errors and Advice

Consider what would happen if a user of the current version of `rustc` were to try and compile a source file from Rust 1.8 with its amazing support for numbers with units:

```shell
> type unit_test.rs
fn main() {
    let x = 42.0_m;
    println!("{}", x);
}
> rustc unit_test.rs
unit_test.rs:2:18: 2:19 error: expected `;`, found `m`
unit_test.rs:2     let x = 42.0_m;
                                ^
```

It's not clear whether this is unsupported syntax or a typo that made its way into the code (unlikely, but possible), and it's not clear what the solution is.  Now, consider what the compiler could tell the user if the code itself contained version information:

```shell
> type unit_test_ver.rs
#![rust_ver="1.8"]
fn main() {
    let x = 42.0_m;
    println!("{}", x);
}
> rustc unit_test_ver.rs
unit_test.rs:1:1: 1:19 error: unsupported version of the Rust language required.
unit_test.rs:1 #![rust_ver="1.8"]
               ^~~~~~~~~~~~~~~~~~
unit_test.rs:1:1: 1:19 note: this compiler supports Rust 0.14.0, version 1.8 required.
```

This gives the user a clear indication that the problem is not the code itself, but the version of the compiler, *and* it tells them what version they need to acquire.  The compiler does not need to guess, or be aware of future changes to the language.

We have seen this issue on IRC in cases where potential users are confronted with example code that does not work because their OS ships an old version of the compiler.  For a new user, it's not clear where the problem lies; perhaps they've failed to correctly configure the compiler itself?  Is it an issue with their packaged version specifically?  If it's out of date, *how* out of date is it?  Does the code only work on nightly builds, not stable (assuming they even know about the release channels)?

## Package Ecosystem Support

Although not a direct concern for the compiler, having a formal notion of what language version is required for code will allow for some useful behaviour on the part of the surrounding package ecosystem.

First, it means that if a user attempts to compile a package that requires a newer version of Rust than their current compiler supports, they can be told this directly, rather than through syntax errors, or errors about missing functions.  This also means it will be much easier to distinguish between "I forgot to use a trait" and "the required method really doesn't exist yet".  Not only that, they can be told precisely what version they need to update to.

In addition, it means that the package manager can develop the ability to automatically constrain its selection of packages based on language version.  For various reasons, users can find themselves "trapped" on a given version of the compiler.  Consider a scenario where a developer is already using `libsplang` version 2.7 as part of a larger project that uses Rust 1.5.  They do a `cargo update` and see the following:

```text
warning: found libsplang v2.8.0, but did not update: requires Rust 1.6.
```

This removes a potential source of apprehension when updating dependencies.  This may be particularly valuable to the aforementioned "trapped" developers for whom "just update `rustc`" is not a viable solution if they discover a dependency has stopped building.  It also means they do not need to go hunting through the package history to find the most up-to-date version that supports their compiler.

## Transitioning To 2.0 (Future Concern)

Consider the following piece of Python code:

```python
print "Hello, World!"
```

This code works in Python 2.x, but produces the following output in 3.x:

```text
  File "<stdin>", line 1
    print "Hello, World!"
                        ^
SyntaxError: invalid syntax
```

Is it the string literal that's the problem, the `!` in the string, or the `print` itself?  (In fact, it is because `print` is no longer a statement in Python 3.x, it is a regular function.)  The error does not make it in any way clear that the code is fundamentally incompatible with their interpreter.

Rust, at present, does somewhat better than this.  For example, with a recent `rustc`:

```shell
> type tilde_test.rs
fn main() {
    let x = ~42;
    println!("{}", x);
}
> rustc tilde_test.rs
tilde_test.rs:2:13: 2:14 error: obsolete syntax: `~` notation for owned pointer
allocation
tilde_test.rs:2     let x = ~42;
                            ^
note: use the `box` operator instead of `~`
error: aborting due to previous error
```

However, how long will this error continue to exist?  This also means the compiler has to keep *parsing* obsolete, invalid syntax.  Consider a hypothetical version 2.0 of rust; with enough changes, it may simply be infeasible to support errors on every construct that has changed or been removed.  Worse, it may not be possible to syntactically distinguish between 1.x and 2.x code that has changed meaning.

A `rust_ver` attribute provides a simple way to provide more accurate information to the user.

This could also work with `cfg` attributes (assuming they are processed *prior* to parsing modules; see "Aside: Improving `cfg`" below) to allow libraries to phase in support for new features without breaking backward compatibility.

Finally, this design also leaves open the door for having multiple versions of `rustc` installed side-by-side, or possibly even a `rustc` that supports multiple language versions.  In the former case, a launcher program (much like `py`/`pyw` on Windows) could use this information to decide which version of the `rustc` compiler to invoke for a given crate.  In the latter, a single compiler could embed the parsing and analysis logic for multiple versions of the language, making a transition even easier.

## Why *Now*?

One important question is: why not simply make these changes later, when Rust 2.0 is definitely going to happen?  The problem here is not with the language itself, but rather with the *ecosystem*.  The "Python Package Index" has, amongst its package metadata, what version or versions of the language a package supports.  However, this metadata is *not required* and (to the author's knowledge) was only added *after* Python 3.0 became a concern.  As a result, it is entirely possible to accidentally install a package for the wrong version of the language.

In addition, the 2.7/3.0 break caused problems for anyone attempting to *run* Python code.  In cases where scripts began with `#!/usr/bin/env python`, installing version 3.0 of Python could potentially break all existing Python scripts.  As a result, more (though not all!) developers started to use `#!/usr/bin/env python2` or `#!/usr/bin/env python3` as appropriate.  However, this *does not work on Windows*, which has no support for hashbangs.  Instead, the developers had to write and ship a custom launcher (`py`/`pyw`) which handled this for Windows users.

This is less of an issue now, but is still a potential for user confusion if a script is *not* annotated correctly.  It is also still an issue for other programs still coded to directly execute the default `python` on a system.  Finally, the solution on Windows (run with `py` instead of `python`) does not apply to other platforms.

Making these changes *now* ensures that if Rust 2.0 starts development, the ecosystem has the necessary metadata and conventions to ease the transition.

# Detailed design

The first change would be to formally give the compiler a concept of what version of the language it compiles.  This would not necessarily (but preferably would) be the same as the version of the *compiler*.  This version would be interpreted as a semantic version: a `rustc` that supports version 1.y of the Rust language would assume it can compile code for version 1.x (where x < y), but not code for version 1.z (where z > y) or 2.0 or higher.

A user should be able to ask `rustc` what version (or versions) it supports.  To facilitate this, this RFC proposes an additional line be output with the `-v verbose` switch that specifies the minimum versions supported by the compiler.  This would be a comma-delimited list of semantic version numbers.  To give a hypothetical example of a `rustc` compiler that supports multiple language versions:

```shell
> rustc -v verbose
rustc 2.3.56 (fe28bad1g 2016-10-31 02:27:15 +0000)
rust-version: 1.14.3, 2.3.56
binary: rustc
host: i686-w64-mingw32
release: 2.3.56
```

Secondly, this RFC proposes a method for specifying the minimum Rust language version required to compile a piece of code.  This would be achieved using a `rust_ver` attribute (name open for bike-shedding), which can be applied in any position where attributes are valid (although, see below about "Halting Parsing Early").  The specific syntax is:

```bnf
rust_ver_item : 'rust_ver' '=' string_lit
```

It the compiler encounters a version attribute indicating a version which is *not* supported, then the compiler should *immediately* abort, informing the user of the required version, as well as what version or versions it supports.

If the version attribute specifies a *compatible* version, then the compiler should do nothing.

The justification for allowing the attribute in *all* positions is that it allows language version-specific additions to be localised upon introduction.  It also means that *example code* (such as might appear in the guides, tutorials, snippets, *etc.*) can accurately record its version requirements without having to always resort to a complete crate.  Ensuring that documentation which contains code examples specify the version of *at least* one representative piece of code (such that it would be caught my simple copy and paste) could be encouraged as a social norm.

If the compiler *does not* encounter any language version attributes, it should assume the code is compatible.  The reasoning for this choice is that if a new user comes to the language and tries to compile a "Hello, World!" program, or some other short, simple program, then having to also specify a language version (without knowing what version to specify) simply imposes an additional point of friction: another excuse to say "this is too much trouble".

On the other hand, this is a bad default for the package ecosystem; it would allow old code to break with no indication as to why.  For this reason, this RFC also proposes a compiler switch used to specify the "default language version" of the code being passed to it.

```shell
    --rust-ver VERSION  Specify the version of the Rust language that the
                        code being compiled requires.
```

If the compiler is given a default language version that it does not support, it should *immediately* abort, informing the user of the required version, as well as what version or versions it supports.

The intention is that Cargo (or other packaging tools) would be modified to strongly discourage, *preferably reject*, packages which do not contain an explicit language version in their metadata.

To prevent needlessly annoying developers when working on an in-development package, this could potentially be enforced *only* upon uploading a package to the central repository.  In other cases, a warning about the missing information, as well as an indication as to what the fallback version is, would be useful.  For example:

```shell
> cargo build
   Compiling libsplang v0.4.7
warning: expected a value of type `string` for `package.language-version`
warning: defaulting to current rustc version: `1.14.3`
```

## Limiting Features by Version

One potentially *very* useful feature would be to allow the compiler to *restrict* what features are available, based on the stated language version.

Currently, new features can be introduced behind a feature gate.  If that feature is used *without* the gate, a compile error is issued, indicating the syntax in question, and what the feature gate is called.

This mechanism could be expanded to inform users about features they are using which require a newer version of the language than the one they have specified.  This would take the form of an error, telling the user what minimum language version they need for that feature.  In effect, new features would transition from a feature gate to a language version gate upon stabilisation.

It is unclear how much effort implementing these errors *comprehensively* would be.  As such, this RFC merely *recommends* that such errors be introduced where feasible.

## Halting Parsing Early

One issue with the `rust_ver` attribute is that the compiler will attempt to parse the entire source *before* examining attributes.  As such, the RFC proposes the following two-stage implementation:

- Initially, `rust_ver` will *only* be valid when applied to a module as an inner attribute.  When `rust_ver` is encountered, the compiler should assert the specified version is compatible *immediately*.  This might be done by making the parser itself aware of what language version it should be parsing, with an "ignore `rust_ver`" setting to assist with external users (such as code formatting or completion).

- The above should be expanded to attributes in all contexts.  This may be more difficult, especially when macros are considered.  Since this would permit *strictly* more code to be valid, it would be a backwards-compatible change and could be introduced as convenient.

This would allow the compiler to report problems with incompatible source code *prior* to encountering a syntax error.

## Aside: Improving `cfg`

The "process `rust_ver` early" behaviour described above could potentially be generalised to allow attributes to flag themselves as needing "early evaluation".  In the case of `cfg`, it could be changed so that instead of discarded an already-parsed item, it instead causes the parser to *ignore* the body of an item.

The simplest and most useful such behaviour would be for the parser to *not* fetch and parse the body of a `mod` item when the body is kept in an external file.  This would allow code to be segmented based on language version support.

However, this is not needed for this RFC and should be specified independently, if at all.

# Drawbacks

This represents an additional "hoop" to jump through when contributing a new package to the ecosystem.  It also represents (for Cargo), additional work in the form of almost assured requests from users for them to "do the right thing" in selecting packages with supported language versions.

Additionally, in the absence of a general "early attribute processing" system, this requires the parser to actively enforce the semantics of the `rust_ver` attribute.  Having such processing hard-wired into the parser is not ideal, from a "separation of concerns" perspective.

If the "language version gate" idea is adopted, this likely means additional internal complexity in the compiler to define, check, and assert these gates, for every new feature introduced.

# Alternatives

One alternative is to simply do nothing.  This will likely cause, at worst, minor discomfort to users, until a backward-incompatible version of the language comes into existence.  It should be noted that these changes would be *difficult to make at a later date*, since in order to be effective, all existing packages would have to be updated.  That said, provided there is sufficient time between implementation and the existence of a backward-incompatible version of Rust, any negative effects are likely to be minimal.

The package ecosystem concerns could also *potentially* be ameliorated prior to a backward-incompatible transition by tagging all existing packages with external metadata and requiring all new or updated packages to specify a version number.

A potential alternative to the simple `rust_ver` syntax would be to allow for more specific version specifications.  For example, `"1.3 .. 1.3.78, 1.4 .."` might be used to work around problems in specific versions.  However, this is really more appropriate for the `cfg` attribute.

A more involved alternative was proposed by Jurily in the ["Rust Version Attribute, based on the Haskell model"](http://discuss.rust-lang.org/t/rust-version-attribute-based-on-the-haskell-model/748) discussion thread.  In brief, this proposes what is effectively a stable version of feature flags, with new features being explicitly requested either individually, or as part of "roll-up" standards.  This alternative could be viewed as a future expansion of *this* proposal.

# Unresolved questions

What should the exact name of the attribute be?  Some other possibilities include: `lang`, `lang_ver`, `language`, `language_version`, `rust_version`, or `rust`.
