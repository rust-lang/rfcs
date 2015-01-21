- Start Date: 2015-01-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Fix rustc’s handling of output (`-o`, `--out-dir` and `--emit`) options and their combinations.

# Motivation

rustc’s handling of various `-o`, `--out-dir`, and `--emit` command line option combinations is
very inconsistent and sometimes unexpected. For example, the compiler will use filename provided
via `-o` option for *some* of `--emit` targets, even though it emits a warning about filename being
ignored. The document aims to propose rules governing interactions of options which influence
compiler output.

# Detailed design

## `--out-dir`

`--out-dir` is never ignored or adjusted, even in presence of other options that would usually
conflict with `--out-dir`. If the option is provided and the directory does not exist yet, the
compiler should create it. Defaults to `.`, otherwise known as current working directory or `$PWD`.
Value of this option from now on will be referred to as `[out-dir]`.

Note, that a requirement for the compiler to create the output directory if it didn’t exist yet is
introduced. This is purely for convenience reasons: creating the output directory after a failure
and restarting the program is exceedingly common nowadays.

## `--emit` and `--crate-type`

The two options influence what and how many files the compiler has to write to the filesystem. The
common case compiler having to write one file only (e.g. `--emit` only has one value). In this case
the path to output file shall be built using these templates:

* `[out-dir]/[filename][.extension]` if `[filename]` was [inferred][inferred].
* `[out-dir]/[filename]` otherwise;

[inferred]: #-o-is-not-specified

When there’s multiple files to output, each output file shall be written to path generated using
`[out-dir]/[filename][.extension]` template.

What `[filename]` resolves to is specified in [section about `-o`](#-o).

The `[.extension]` is file extension specific to the emitted target. This is a list of extensions
for each currently supported target:

* asm – `.s`;
* llvm-bc – `.bc`;
* llvm-ir – `.ll`;
* obj – `.o`;
* dep-info – `.d`;
* link – depends on `crate-type`:
  * bin – no extension or `.exe`;
  * lib – `.rlib`;
  * rlib – `.rlib`;
  * dylib – `.so`, `.dll` or `.dylib`;
  * staticlib – `.a`.

An exception to most of the rules above is the `link` target. Different path templates for this
target are necessitated by the fact that the target depends on value(s) of `--crate-type`:

* `[out-dir]/[filename][.extension]` for `bin` `crate-type`;
* `[out-dir]/lib[filename][.extension]` for other `crate-type`s.

```
$ rustc foo.rs -o foo --crate-type=staticlib
# Output: libfoo.a
$ rustc foo.rs -o foo
# Output on Windows: foo.exe
```

## `-o`

The value of `-o` option, unlike `--out-dir` may be adapted by `rustc` in case it conflicts with
other options. Ignoring the option is strongly discouraged.

Were the value be ignored or adapted, the warning detailing actions taken should be emitted.

### Interactions with `--out-dir`

In absence of `--out-dir` option, `-o` value may contain directory path components:

    $ rustc foo.rs -o bar/foo # A-OK

For this particular command the compiler behaves as if it had gotten options `--out-dir=bar/` and
`-o foo`. More generally, the compiler interprets the option by splitting value into
[`filename`][filename] to be used as `[filename]` and [`dirname`][dirname] to be used as
`[out-dir]`.

[filename]: http://doc.rust-lang.org/std/path/trait.GenericPath.html#tymethod.filename
[dirname]: http://doc.rust-lang.org/std/path/trait.GenericPath.html#tymethod.dirname

If both `--out-dir` and `-o` are provided, the `-o` value is adjusted so it only contains a
`filename`, which is used as `[filename]`:

    $ rustc foo.rs --out-dir=baz -o foo # A-OK
    $ rustc foo.rs --out-dir=baz -o bar/foo
    warning: `-o foo` is used instead of `-o bar/foo`, because `--out-dir=baz` is specified

Note, that this proposed behaviour is completely different than status quo. At the time of writing
both commands in the previous example completely ignore `--out-dir` and write the files to `./foo`
and `./bar/foo` respectively. This is, arguably, completely unintuitive and requires extensive
documentation for the options. With the change both options’ behaviour becomes as intuitive as it
gets: `--out-dir` is the option to specify output directory and `-o` is the option to specify
output filename. Ability to specify output directory with `-o` is only a (sometimes unavailable)
shorthand convenience.

### Interactions with `--emit`

The extension has no special meaning in `-o` option values and becomes a part of `[filename]`
verbatim:

    $ rustc foo.rs --emit=asm,obj -o foo.bar
    # Output: foo.bar.s foo.bar.o
    $ rustc foo.rs --crate-type=staticlib,rlib -o foo.bar
    # Output: libfoo.bar.rlib libfoo.bar.a
    $ rustc foo.rs --emit=asm -o foo.bar
    # Output: foo.bar
    $ rustc foo.rs --crate-type=rlib -o foo.bar
    # Output: libfoo.bar.rlib

### `-o` is not specified

In case `-o` is not specified, compiler generates `[filename]` on a best-effort basis. rustc could
use any of following sources of data:

* `crate-name`;
* [Filestem][filestem] of the input file;
* …

[filestem]: http://doc.rust-lang.org/std/path/trait.GenericPath.html#method.filestem

If `[filename]` cannot be generated from the available data, a sensible default such as `rust_out`
is used.

## `-C extra-filename`

When `-C extra-filename` option is specified, the `[filename]` is mutated so the new value is
`[filename][extra-filename]`.

    $ rustc foo.rs -C extra-filename=qux -o foo.bar --crate-type=rlib
    # Output: libfoo.barqux.rlib

# Drawbacks

The special case for `link` target might be somewhat confusing:

    $ rustc foo.rs --emit=link -o foo.bar
    # Output on Windows: foo.bar.exe

but

    $ rustc foo.rs --emit=asm -o foo.bar
    # Output: foo.bar

It does not allow setting precise filename for `link` target outputs, since both `lib` prefix and
an extension might be appended, while other targets don’t share such a restriction. On the other
hand, the drawback is necessary, because rustc needs both the extension and the prefix to consider
it as a linkage candidate and binaries without `.exe` extension look silly on Windows.

# Alternatives

Keeping current inconsistent behaviour.

# Unresolved questions

None.
