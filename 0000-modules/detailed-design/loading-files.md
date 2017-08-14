# Loading Files

When building a Rust project, rustc will and parse some files as Rust code in
addition to the root module. These will be used to construct a module tree. By
default, cargo will generate the list of files to load in this way for you,
though you can generate such a list yourself and specify it in your
`Cargo.toml`, or you can generate the list in another way for your non-cargo
build system.

This eliminates the need to write `mod` statements to add new files to your
project. Instead, files will be picked up automatically as a part of your
module hierarchy.

## Detailed design

### Processing the `--module` list (rustc)

rustc takes a new argument called `--module`. Each `--module` argument passes
the name of a file to treat as a module. rustc will attempt to open and parse
all of these files, and report any errors if it is unable to. It will mount
these files as a tree of Rust modules using rules which mirror the current
rules for looking up modules.

It will not attempt to open or parse files if the paths meet these conditions:

* The file name is not a valid Rust identifier followed by `.rs`.
* The file is not a subdirectory of the directory containing the root module.
* Any of the subdirectories of the root module in the path to this file are not
valid Rust identifiers.

(Cargo's default system will not pass any files that would be ignored by these
conditions, but if they are passed by some other system, they are ignored
regardless.)

Rust will mount files as modules using these rules:

* If a file is named `mod.rs`, it will mount it as the module for the name of
directory which contains it (the directory containing the root module cannot
contain a `mod.rs` file; this is an error).
* Otherwise, it will mount it at a module with the name of the file prior to
the `.rs`.

All modules mounted this way are visible to the entire crate, but are not (by
default) visible in the external API of this crate.

If, during parsing, a `mod` statement is encountered which would cause Rust to
load a file which was a part of the `--module` list, Rust does not
attempt to load that file. Instead, a warning is issued that that `mod`
statement is dead code. (See [migrations][migrations] for more info.)

If a module is mounted multiple times, or there are multiple possible files
which could define a module, that continues to be an error.

Another result of this design is that the naming convention becomes slightly
more flexible. Prior to this RFC, if a module file is going to have submodule
files, it must be located at `mod.rs` in the directory containing those
submodules - e.g. `src/foo/mod.rs`. As a result of this RFC, users can instead
locate it at `src/foo.rs`, but still have submodules in the `foo` directory.
Some users have requested this functionality because their tooling does not
easily support distinguishing files with the same name, such as all of their
`mod.rs` files.

In fact, in this design, it is not necessary to have a `foo.rs` or `foo/mod.rs`
in order to have modules in the `foo` directory. Without such a file, `foo`
will just have no items in it other than the automatically loaded submodules.
For example:

```
/foo
    bar.rs
    baz.rs
lib.rs
```

This mounts a submodule `foo` with two items in it: submodules `bar` and `baz`.
There is no compiler error.

### Gathering the `--module` list (cargo)

#### Library and binary crates

When building a crate, cargo will collect a list of paths to pass to rustc's 
`--module` argument. It will only gather files for which the file name
has the form of a valid Rust identifier, followed by the characters `.rs`.

cargo will recursively walk the directory tree, gathering all appropriate
files, beginning with the directory which contains the crate root file. It will
ignore these files and directories:

* The crate root file itself.
* Any directory with a name which is not a valid Rust identifier.
* If the crate root is in the `src` subdirectory of the Cargo manifest
directory, and there is a directory called `src/bin`, cargo will ignore that
subdirectory.

In short, cargo will include all appropriately named files inside the directory
which contains the crate root, except that it will ignore the `src/bin`
directory.

Packages containing multiple crates which wish to use the default module list
will need to make sure that they do not have multiple crates rooted in the same
directory, or within a subdirectory of another crate. The most likely
problematic crates today are those which have both a `src/lib.rs` and a
`src/main.rs`. We recommend those crates move their binary crate to the
`src/bin` directory solution.

While gathering the default module list, cargo will determine if any other
crate is rooted in a directory which would be collected by the default module
list, and will instead not pass a `--module` list and issue a warning in
that case, informing users that they need to rearrange their crates or provide
a list of modules themselves.

(**Note:** These projects will receive a warning, but will not immediately be
broken, because the `mod` statements they already contain will continue to pick
up files.)

#### Tests, examples, and benchmarks

Test, example, and benchmark crates follow a different set of rules. If the
crate is located in the appropriate top-level directory (`tests`, `examples`,
and so on), no `--module` list will be collected by default. However,
subdirectories of these directories will be treated as individual binary
crates: a `main.rs` file will be treated as the root module, and all other
appropriately named files will be passed as `--module`, using the same
rules described above.

So if you have an examples directory like this:

```
examples/
    foo.rs
    bar/
       main.rs
       baz.rs
```

This contains two examples, a `foo` example and a `bar` example, and the `bar`
crate will have `baz.rs` as a submodule.

The reason for this is that today, cargo will treat every file in `tests`,
`examples`, and `benches` as independent crates, which is a well-motivated
design. Usually, these are small enough that a single file makes sense.
However, today, cargo does not make it particularly easy to have tests,
examples, or benchmarks that are multiple files. This design will create a
pattern to enable users to do this.

#### Providing your own module list

The target section of the `Cargo.toml` will  gain a new item called `modules`.
This item is expected to be an array of strings, which are relative paths from
the cargo manifest directory to files containing Rust source code. If this item
is specified, cargo will canonicalize these paths and pass them to rustc as the
`--module` argument when building this target.

## Drawbacks

The RFC authors believe that making mod statements unnecessary is a *net* win,
but we must acknowledge that it is not a *pure* win. There are several
advantages that mod statements bring which will not be fully replicated in the
new system.

Some workflows have been convenienced by the fact that statements need to be
added to the source code to add new modules to files. For example, it makes it
easier for users to leave their src directories a little bit dirty while
working, such as through an incomplete `git stash`. If users wish to comment
out a module, it can be easier to comment out the `mod` statement than to
comment out the module file. In general, it enables users to leave code which
would not compile in their src directory without explicitly commenting it out.

Some users have expressed strong concerns that by deriving the module structure
from the file system, without making additional syntactic statements, they will
not be able to as easily find the information they need to navigate and
comprehend the codebases they are reading or working on. To partly ease their
concern, the RFC allows users to explicitly specify their module lists at the
build layer, instead of the source layer. This has some disadvantages, in that
users may prefer to not have to open the build configuration either.

This will require migrating users away from `mod` statements toward the new
system.

## Alternatives

An alternative is to do nothing, and continue to use `mod` statements.

We could also put the file-lookup in rustc, instead of cargo, and have rustc
perform its own directory walk. We believe this would be a bad choice of
layering.

During the design process, we considered other, more radical redesigns, such as
making all files "inlined" into their directory and/or using the filename to
determine the visibility of a module. We've decided not to steps that are this
radical right now.

[migrations]: detailed-design/migrations.md
