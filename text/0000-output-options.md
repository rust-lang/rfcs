- Start Date: 2015-01-18
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Fix rustc’s handling of `-o`, `--out-dir` and `--emit` options and their combinations.

# Motivation

rustc’s handling of various `-o`, `--out-dir` and `--emit` command line option combinations is very
inconsistent and sometimes unexpected. For example the compiler will use filename provided through
`-o` option for *some* of `--emit` targets, even though, it emits a warning about the filename
being ignored. The document aims to propose rules governing interactions of the three options.

# Detailed design

## `--out-dir`

`--out-dir` is never ignored or adjusted, even in presence of other options that would usually
conflict with `--out-dir`. If the option is provided and the directory does not exist yet, the
compiler will create it.

## `--emit`

If there is a single emit target or the option is not specified (defaults to `--emit=link`), the
output will be written to:

* `[out-dir]/[filestem]` if `--dir-name` is specified;
* `[filestem]` otherwise.

In case there is multiple emit targets, each output will be written to:

* `[out-dir]/[filestem][.extension]` if `--dir-name` is specified;
* `[filestem][.extension]` otherwise.

We will cover the `[filestem]` in section about `-o`.

The `[.extension]` is file extension specific to the emitted target. This is a list of extensions
for each currently supported target:

* asm - `.s`;
* llvm-bc - `.bc`;
* llvm-ir - `.ll`;
* obj - `.o`;
* dep-info – `.d`;
* link – depends on `crate-type`:
  * bin – no extension;
  * lib – `.rlib`;
  * rlib – `.rlib`;
  * dylib – `.so`, `.dll` or `.dylib`;
  * staticlib – `.a`.

## `-o`

The value of `-o` option, unlike `--out-dir` may be adjusted by `rustc` in case it conflicts with
other options. Ignoring it is strongly discouraged. Were the value be ignored or adjusted, the
warning is output detailing from what to what the value was changed and the reason why it was
ignored or adjusted.

* `-o` value may be a full path to the output file, like this:

        $ rustc foo.rs -o bar/foo # A-OK

* However this is not the case when both `-o` and `--out-dir` are specified. Any directory path
  components in `-o` are ignored:

        $ rustc foo.rs --out-dir=baz -o foo # A-OK
        $ rustc foo.rs --out-dir=baz -o bar/foo
        warning: `-o foo` is used instead of `-o bar/foo`, because `--out-dir=baz` is specified

* Were both `-o` and multiple `--emit` targets specified, any directory path components in `-o` are
  ignored too:

         $ rustc foo.rs --emit=asm -o bar/foo
         # A-OK, the file is output to bar/foo
         $ rustc foo.rs --emit=asm,obj -o bar/foo
         warning: `-o foo` is used instead of `-o bar/foo`, because multiple `--emit` targets are specified
         # Output: foo.s foo.o

    The adjusted value of `-o` option then becomes `[filestem]` to be used in output file path
    generation. See `--emit` section about its usage.

* extension has no special meaning in `-o` option values and is not stripped or replaced:

        $ rustc foo.rs --emit=asm,obj -o foo.bar
        # Output: foo.bar.s foo.bar.o

In case `-o` is not specified, `[filestem]` is inferred from `crate-name` and other data available
to the compiler.

# Drawbacks

Nothing yet.

# Alternatives

Keeping current inconsistent behaviour.

# Unresolved questions

None.
