- Start Date: 2015-01-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Fix rustc’s handling of output (`-o`, `--out-dir` and `--emit`) options and their combinations.

# Motivation

rustc’s handling of various `-o`, `--out-dir` and `--emit` command line option combinations is very
inconsistent and sometimes unexpected. For example, the compiler will use filename provided via
`-o` option for *some* of `--emit` targets, even though it emits a warning about filename being
ignored. The document aims to propose rules governing interactions of the three options.

# Detailed design

## `--out-dir`

`--out-dir` is never ignored or adjusted, even in presence of other options that would usually
conflict with `--out-dir`. If the option is provided and the directory does not exist yet, the
compiler should create it.

## `--emit`

If there is a single emit target or the option is not specified (defaults to `--emit=link`), the
output will be written to:

* `[out-dir]/[filestem]` if `--out-dir` is specified;
* `[filestem]` otherwise.

In case there is multiple emit targets, each output will be written to:

* `[out-dir]/[filestem][.extension]` if `--out-dir` is specified;
* `[filestem][.extension]` otherwise.

`[out-dir]` is the value of `--out-dir`. We will cover `[filestem]` in depth in section about `-o`.

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

`link` target should prepend prefix `lib` to `[filestem]` for all `crate-type`s except `bin`:

    $ rustc foo.rs -o foo --crate-type=staticlib
    # Output: libfoo.a

## `-o`

The value of `-o` option, unlike `--out-dir` may be adapted by `rustc` in case it conflicts with
other options. Ignoring the option is strongly discouraged.

Were the value be ignored or adapted, the warning detailing actions taken should be emitted.

### Interactions with `--out-dir`

In absence of `--out-dir` option, `-o` value may contain directory path components:

    $ rustc foo.rs -o bar/foo # A-OK

For this particular command the compiler behaves as if it had gotten options `--out-dir=bar/` and
`-o foo`. More generally, the compiler interprets the option by splitting value into
[`filename`][filename] to be used as `[filestem]` and [`dirname`][dirname] to be used as
`[out-dir]`.

[filename]: http://doc.rust-lang.org/std/path/trait.GenericPath.html#tymethod.filename
[dirname]: http://doc.rust-lang.org/std/path/trait.GenericPath.html#tymethod.dirname

If both `--out-dir` and `-o` are provided, the `-o` value is adjusted so it only contains a
`filename`, which is used as `[filestem]`:

    $ rustc foo.rs --out-dir=baz -o foo # A-OK
    $ rustc foo.rs --out-dir=baz -o bar/foo
    warning: `-o foo` is used instead of `-o bar/foo`, because `--out-dir=baz` is specified

### Interactions with `--emit`

The extension has no special meaning in `-o` option values and becomes a part of `[filestem]`
verbatim:

    $ rustc foo.rs --emit=asm,obj -o foo.bar
    # Output: foo.bar.s foo.bar.o
    $ rustc foo.rs --emit=asm -o foo.bar
    # Output: foo.bar

### `-o` is not specified

In case `-o` is not specified, compiler generates `[filestem]` on a best-effort basis. rustc could
use any of following sources of data:

* `crate-name`;
* Filename of the input file;
* …

If `[filestem]` cannot be generated from the available data, a sensible default such as `rust-out`
is used.

## `--extra-filename`

When `--extra-filename` option is specified, the `[filestem]` is mutated so the new value is
`[filestem][extra-filename]`.

# Drawbacks

Nothing yet.

# Alternatives

Keeping current inconsistent behaviour.

# Unresolved questions

None.
