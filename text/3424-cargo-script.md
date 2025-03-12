- Feature Name: cargo-script
- Start Date: 2023-04-26
- Pre-RFC: [internals](https://internals.rust-lang.org/t/pre-rfc-cargo-script-for-everyone/18639)
- eRFC PR: [rust-lang/rfcs#3424](https://github.com/rust-lang/rfcs/pull/3424)
- Tracking Issue: [rust-lang/cargo#12207](https://github.com/rust-lang/cargo/issues/12207)

# Summary
[summary]: #summary

This *experimental RFC* adds unstable support for single-file
packages in cargo so we can explore the design and resolve
questions with an implementation to collect feedback on.

Single-file packages are `.rs` files with an embedded
manifest.  These will be accepted with just like `Cargo.toml` files with
`--manifest-path`.  `cargo` will be modified to accept `cargo <file>.rs` as a
shortcut to `cargo run --manifest-path <file>.rs`.  This allows placing
`cargo` in a `#!` line for directly running these files.

Example:
```rust
#!/usr/bin/env cargo

//! ```cargo
//! [dependencies]
//! clap = { version = "4.2", features = ["derive"] }
//! ```

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(short, long, help = "Path to config")]
    config: Option<std::path::PathBuf>,
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args);
}
```
```console
$ ./prog --config file.toml
Args { config: Some("file.toml") }
```

See [`cargo-script-mvs`](https://crates.io/crates/cargo-script-mvs) for a demo.

# Motivation
[motivation]: #motivation

**Collaboration:**

When sharing reproduction cases, it is much easier when everything exists in a
single code snippet to copy/paste.  Alternatively, people will either leave off
the manifest or underspecify the details of it.

This similarly makes it easier to share code samples with coworkers or in books
/ blogs when teaching.

**Interoperability:**

One angle to look at proposals is if there is a single obvious
solution.  While this isn't the case for single-file packages, there is enough of
a subset of one. By standardizing that subset, we allow greater
interoperability between solutions (e.g.
[playground could gain support](https://users.rust-lang.org/t/call-for-contributors-to-the-rust-playground-for-upcoming-features/87110/14?u=epage)
).  This would make it easier to collaborate..

**Prototyping:**

Currently to prototype or try experiment with APIs or the language, you need to either
- Use the playground
  - Can't access local resources
  - Limited in the crates supported
  - *Note:* there are alternatives to the playground that might have fewer
    restrictions but are either less well known or have additional
    complexities.
- Find a place to do `cargo new`, edit `Cargo.toml` and `main.rs` as necessary, and `cargo run` it, then delete it
  - This is a lot of extra steps, increasing the friction to trying things out
  - This will fail if you create in a place that `cargo` will think it should be a workspace member

By having a single-file package,
- It is easier to setup and tear down these experiments, making it more likely to happen
- All crates will be available
- Local resources are available

**One-Off Utilities:**

It is fairly trivial to create a bunch of single-file bash or python scripts
into a directory and add it to the path.  Compare this to rust where
- `cargo new` each of the "scripts" into individual directories
- Create wrappers for each so you can access it in your path, passing `--manifest-path` to `cargo run`

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

As an eRFC, this is meant to convey what we are looking to
accomplish.  Many of the details may change before
stablization.

### Creating a New Package

*(Adapted from [the cargo book](https://doc.rust-lang.org/cargo/guide/creating-a-new-project.html))*

To start a new [package][def-package] with Cargo, create a file named `hello_world.rs`:
```rust
#!/usr/bin/env cargo

fn main() {
    println!("Hello, world!");
}
```

Let's run it
```console
$ chmod +x hello_world.rs
$ ./hello_world.rs
Hello, world!
```

### Dependencies

*(Adapted from [the cargo book](https://doc.rust-lang.org/cargo/guide/dependencies.html))*

[crates.io] is the Rust community's central [*package registry*][def-package-registry]
that serves as a location to discover and download
[packages][def-package]. `cargo` is configured to use it by default to find
requested packages.

#### Adding a dependency

To depend on a library hosted on [crates.io], you modify `hello_world.rs`:
```rust
#!/usr/bin/env cargo

//! ```cargo
//! [dependencies]
//! regex = "1.8.0"
//! ```

fn main() {
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    println!("Did our date match? {}", re.is_match("2014-01-01"));
}
```

The `cargo` section in the doc-comment (any module inner doc-comment style is supported) is
called a [***manifest***][def-manifest], and it contains all of the metadata
that Cargo needs to compile your package. This is written in the [TOML] format
(pronounced /tɑməl/).

`regex = "1.8.0"` is the name of the [crate][def-crate] and a [SemVer] version
requirement. The [specifying
dependencies](https://doc.rust-lang.org/cargo/guide/../reference/specifying-dependencies.html) docs have more
information about the options you have here.

You can then re-run this and Cargo will fetch the new dependencies and all of
their dependencies.  You can see this by passing in `--verbose`:
```console
$ cargo --verbose ./hello_world.rs
      Updating crates.io index
   Downloading memchr v0.1.5
   Downloading libc v0.1.10
   Downloading regex-syntax v0.2.1
   Downloading memchr v0.1.5
   Downloading aho-corasick v0.3.0
   Downloading regex v0.1.41
     Compiling memchr v0.1.5
     Compiling libc v0.1.10
     Compiling regex-syntax v0.2.1
     Compiling memchr v0.1.5
     Compiling aho-corasick v0.3.0
     Compiling regex v0.1.41
     Compiling hello_world v0.1.0 (file:///path/to/package/hello_world)
Did our date match? true
```

Cargo will cache the exact information (in a location referred to as
`CARGO_HOME`) about which revision of all of these dependencies we used.

Now, if `regex` gets updated, we will still build with the same revision until
we choose to `cargo update --manifest-path hello_world.rs`.

## Package Layout

*(Adapted from [the cargo book](https://doc.rust-lang.org/cargo/guide/project-layout.html))*

When a single file is not enough, you can separately define a `Cargo.toml` file along with the `src/main.rs` file.  Run
```console
$ cargo new hello_world --bin
```

We’re passing `--bin` because we’re making a binary program: if we
were making a library, we’d pass `--lib`. This also initializes a new `git`
repository by default. If you don't want it to do that, pass `--vcs none`.

Let’s check out what Cargo has generated for us:
```console
$ cd hello_world
$ tree .
.
├── Cargo.toml
└── src
    └── main.rs

1 directory, 2 files
```
Unlike the `hello_world.rs`, a little more context is needed in `Cargo.toml`:
```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2021"

[dependencies]

```

Cargo uses conventions for file placement to make it easy to dive into a new
Cargo [package][def-package]:

```text
.
├── Cargo.lock
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs
│   └── bin/
│       ├── named-executable.rs
│       ├── another-executable.rs
│       └── multi-file-executable/
│           ├── main.rs
│           └── some_module.rs
├── benches/
│   ├── large-input.rs
│   └── multi-file-bench/
│       ├── main.rs
│       └── bench_module.rs
├── examples/
│   ├── simple.rs
│   └── multi-file-example/
│       ├── main.rs
│       └── ex_module.rs
└── tests/
    ├── some-integration-tests.rs
    └── multi-file-test/
        ├── main.rs
        └── test_module.rs
```

* `Cargo.toml` and `Cargo.lock` are stored in the root of your package (*package
  root*).
* Source code goes in the `src` directory.
* The default library file is `src/lib.rs`.
* The default executable file is `src/main.rs`.
    * Other executables can be placed in `src/bin/`.
* Benchmarks go in the `benches` directory.
* Examples go in the `examples` directory.
* Integration tests go in the `tests` directory.

If a binary, example, bench, or integration test consists of multiple source
files, place a `main.rs` file along with the extra [*modules*][def-module]
within a subdirectory of the `src/bin`, `examples`, `benches`, or `tests`
directory. The name of the executable will be the directory name.

You can learn more about Rust's module system in [the book][book-modules].

See [Configuring a target] for more details on manually configuring targets.
See [Target auto-discovery] for more information on controlling how Cargo
automatically infers target names.

[book-modules]: https://doc.rust-lang.org/cargo/guide/../../book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
[Configuring a target]: https://doc.rust-lang.org/cargo/guide/../reference/cargo-targets.html#configuring-a-target
[def-package]:           https://doc.rust-lang.org/cargo/guide/../appendix/glossary.html#package          '"package" (glossary entry)'
[Target auto-discovery]: https://doc.rust-lang.org/cargo/guide/../reference/cargo-targets.html#target-auto-discovery
[TOML]: https://toml.io/
[crates.io]: https://crates.io/
[SemVer]: https://semver.org
[def-crate]:             https://doc.rust-lang.org/cargo/guide/../appendix/glossary.html#crate             '"crate" (glossary entry)'
[def-package]:           https://doc.rust-lang.org/cargo/guide/../appendix/glossary.html#package           '"package" (glossary entry)'
[def-package-registry]:  https://doc.rust-lang.org/cargo/guide/../appendix/glossary.html#package-registry  '"package-registry" (glossary entry)'
[def-manifest]:          https://doc.rust-lang.org/cargo/guide/../appendix/glossary.html#manifest          '"manifest" (glossary entry)'

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The details will be deferred to the implementation.

Initial guidelines for evaluating decisions:
- Single-file packages should have a first-class experience
  - Provides a higher quality of experience (doesn't feel like a hack or tacked on)
  - Transferable knowledge, whether experience, stackoverflow answers, etc
  - Easier unassisted migration between single-file and multi-file packages
  - The more the workflows deviate, the higher the maintenance and support costs for the cargo team
  - Example implications:
    - Workflows, like running tests, should be the same as multi-file packages rather than being bifurcated
    - Manifest formats should be the same rather than using a specialized schema or data format
- Friction for starting a new single-file package should be minimal
  - Easy to remember, minimal syntax so people are more likely to use it in
    one-off cases, experimental or prototyping use cases without tool assistance
  - Example implications:
    - Embedded manifest is optional which also means we can't require users specifying `edition`
    - See also the implications for first-class experience
    - Workspaces for single-file packages should not be auto-discovered as that
      will break unless the workspaces also owns the single-file package which
      will break workflows for just creating a file anywhere to try out an
      idea.
- Cargo/rustc diagnostics and messages (including `cargo metadata`) should be
  in terms of single-file packages and not any temporary files
  - Easier to understand the messages
  - Provides a higher quality of experience (doesn't feel like a hack or tacked on)
  - Example implications:
    - Most likely, we'll need single-file packages to be understood directly by
      rustc so cargo doesn't have to split out the `.rs` content into a temp
      file that gets passed to cargo which will cause errors to point to the
      wrong file
    - Most likely, we'll want to muck with the errors returned by `toml_edit`
      so we render manifest errors based on the original source code which will require accurate span information.

# Drawbacks
[drawbacks]: #drawbacks

This will likely permeate cargo's code base.  While we are
fairly positive this has a path to stablization and it won't
extend out for too long, we will be paying for that cost with
little benefit until then.

Then when this is stablized, this increases the surface area of
cargo for the cargo team to maintain and support.


We will not be reserving syntax for `build.rs`, `[lib]`
support, proc-maros, or other functionality to be added later
with the assumption that if these features are needed, a user
should be using a multi-file package.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Scope

The `cargo-script` family of tools has a single command
- Run `.rs` files with embedded manifests
- Evaluate command-line arguments (`--expr`, `--loop`)

This behavior (minus embedded manifests) mirrors what you might expect from a
scripting environment, minus a REPL.  We could design this with the future possibility of a REPL.

However
- The needs of `.rs` files and REPL / CLI args are different, e.g. where they get their dependency definitions
- A REPL is a lot larger of a problem, needing to pull in a lot of interactive behavior that is unrelated to `.rs` files
- A REPL for Rust is a lot more nebulous of a future possibility, making it pre-mature to design for it in mind

Therefore, this eRFC is limited in scope to running single-file
rust packages.

## First vs Third Party

As mentioned, a reason for being first-party is to standardize the convention
for this which also allows greater interop.

A default implementation ensures people will use it.  For example, `clap`
received an issue with a reproduction case using a `cargo-play` script that
went unused because it just wasn't worth installing yet another, unknown tool.

This also improves the overall experience as you do not need the third-party
command to replicate support for every potential feature including:
- `cargo test` and other built-in cargo commands
- `cargo expand` and other third-party cargo commands
- `rust-analyzer` and other editor/IDE integration

While other third-party cargo commands might not immediately adopt single-file
packages, first-party support for them will help encourage their adoption.

This still leaves room for third-party implementations, either differentiating themselves or experimenting with
- Alternative caching mechanisms for lower overhead
- Support for implicit `main`, like doc-comment examples
- Template support for implicit `main` for customizing `use`, `extern`, `#[feature]`, etc
- Short-hand dependency syntax (e.g. `//# serde_json = "*"`)
- Prioritizing other workflows, like runtime performance

# Prior art
[prior-art]: #prior-art

See [Single-file scripts that download their
dependencies](https://dbohdan.com/scripts-with-dependencies)
for a wide view of this space.

Existing Rust solutions:
- [`cargo-script`](https://github.com/DanielKeep/cargo-script)
  - Single-file (`.crs` extension) rust code
    - Partial manifests in a `cargo` doc comment code fence or dependencies in a comment directive
    - `run-cargo-script` for shebangs and setting up file associations on Windows
  - Performance: Shares a `CARGO_TARGET_DIR`, reusing dependency builds
  - `--expr <expr>` for expressions as args (wraps in a block and prints blocks value as `{:?}` )
     - `--dep` flags since directives don't work as easily
  - `--loop <expr>` for a closure to run on each line
  - `--test`, etc flags to make up for cargo not understanding thesefiles
  - `--force` to rebuild` and `--clear-cache`
  - Communicates through scripts through some env variables
- [`cargo-scripter`](https://crates.io/crates/cargo-scripter)
  - See above with 8 more commits
- [`cargo-eval`](https://crates.io/crates/cargo-eval)
  - See above with a couple more commits
- [`rust-script`](https://crates.io/crates/rust-script)
  - See above
  - Changed extension to `.ers` / `.rs`
  - Single binary without subcommands in primary case for ease of running
  - Implicit main support, including `async main` (different implementation than rustdoc)
  - `--toolchain-version` flag
- [`cargo-play`](https://crates.io/crates/cargo-play)
  - Allows multiple-file scripts, first specified is the `main`
  - Dependency syntax `//# serde_json = "*"`
  - Otherwise, seems like it has a subset of `cargo-script`s functionality
- [`cargo-wop`](https://crates.io/crates/cargo-wop)
  - `cargo wop` is to single-file rust scripts as `cargo` is to multi-file rust projects
  - Dependency syntax is a doc comment code fence

Related Rust solutions:
- [Playground](https://play.rust-lang.org/)
  - Includes top 100 crates
- [Rust Explorer](https://users.rust-lang.org/t/rust-playground-with-the-top-10k-crates/75746)
  - Uses a comment syntax for specifying dependencies
- [`runner`](https://github.com/stevedonovan/runner/)
  - Global `Cargo.toml` with dependencies added via `runner --add <dep>` and various commands  / args to interact with the shared crate
  - Global, editable prelude / template
  - `-e <expr>` support
  - `-i <expr>` support for consuming and printing iterator values
  - `-n <expr>` runs per line
- [`evcxr`](https://github.com/google/evcxr)
  - Umbrella project which includes a REPL and Jupyter kernel
  - Requires opting in to not ending on panics
  - Expressions starting with `:` are repl commands
  - Limitations on using references
- [`irust`](https://github.com/sigmaSd/IRust)
  - Rust repl
  - Expressions starting with `:` are repl commands
  - Global, user-editable prelude crate
- [papyrust](https://crates.io/crates/papyrust)
  - Not single file; just gives fast caching for a cargo package

D:
- [dub](https://dub.pm/advanced_usage)
  - `dub hello.d` is shorthand for `dub run --single hello.d`
  - Regular nested block comment (not doc-comment) at top of file with `dub.sdl:` header

Java
- [JEP 330: Launch Single-File Source-Code Programs](https://openjdk.org/jeps/330)
- [jbang](https://www.jbang.dev/)
  - `jbang init` w/ templates
  - `jbang edit` support, setting up a recommended editor w/ environment
  - Discourages `#!` and instead encourages looking like shell code with `///usr/bin/env jbang "$0" "$@" ; exit $?`
  - Dependencies and compiler flags controlled via comment-directives, including
    - `//DEPS info.picocli:picocli:4.5.0` (gradle-style locators)
      - Can declare one dependency as the source of versions for other dependencies (bom-pom)
    - `//COMPILE_OPTIONS <flags>`
    - `//NATIVE_OPTIONS <flags>`
    - `//RUNTIME_OPTIONS <flags>`
  - Can run code blocks from markdown
  - `--code` flag to execute code on the command-line
  - Accepts scripts from `stdin`

Haskell
- [`runghc` / `runhaskell`](https://downloads.haskell.org/ghc/latest/docs/users_guide/runghc.html)
  - Users can use the file stem (ie leave off the extension) when passing it in
- [cabal's single-file haskel script](https://cabal.readthedocs.io/en/stable/getting-started.html#run-a-single-file-haskell-script)
  - Command is just `cabal`, which could run into weird situations if a file has the same name as a subcommand
  - Manifest is put in a multi-line comment that starts with `cabal:`
  - Scripts are run with `--quiet`, regardless of which invocation is used
  - Documented in their "Getting Started" and then documented further under `cabal run`.
- [`stack script`](https://www.wespiser.com/posts/2020-02-02-Command-Line-Haskell.html)
  - `stack` acts as a shortcut for use in `#!`
  - Delegates resolver information but can be extended on the command-line
  - Command-line flags may be specified in a multi-line comment starting with `stack script`

Cross-language
- [`scriptisto`](https://github.com/igor-petruk/scriptisto)
  - Supports any compiled language
  - Comment-directives give build commands
- [nix-script](https://github.com/BrianHicks/nix-script)
  - Nix version of scriptisto, letting you use any Nix dependency

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Through the eRFC process, we particularly want to resolve:

What command should be used in `#!` lines?
- If `cargo`, what precedence does it have compared to
  built-in commands, aliases, and external commands.
- If something else, what to name it?
- If `cargo-<foo>` how to deal with diverging behavior
  between `cargo foo` and `cargo-foo` since `cargo foo` won't
  play nice in a `#!` line across platforms

How to keep build-times down for the best exploratory experience?
- e.g. using a central `CARGO_TARGET_DIR`
- e.g. locking to similar dependencies across scripts for reusing more of the cache in `CARGO_TARGET_DIR``

How the default `RUST_BACKTRACE` setting affects the use cases for single-file
packages if working around it is worth it?

Whether single-file packages should be run within the
environment (`.cargo/config.toml`, `rust-toolchain.toml`) of
the current working directory (like `cargo run`) or a fixed
location like their own directory (more like `cargo install`)

How to embed the manifest within the file?
- How obvious it is for new users when they see it
- How easy it is for newer users to remember it and type it out
- How machine editable it is for `cargo add` and friends
- Needs to be valid Rust code based on the earlier stated design guidelines
- Lockfiles might also need to reuse how we attach metadata to the file

How do we handle the lockfile, balancing single-file
package use case needs (single file, easy copy / paste, etc) with
the expectations of Rust for reproducibility?
- Sharing of single-file projects should be easy
  - In "copy/paste" scenarios, like reproduction cases in issues, how often
    have lockfiles been pertinent for reproduction?
- There is an expectation of a reproducible Rust experience
- Dropping of additional files might be frustrating for users to deal with (in
  addition to making it harder to share it all)
- We would need a way to store the lockfile for `stdin` without conflicting
  with parallel runs
- `cargo` already makes persisting of `Cargo.lock` optional for multi-file
  packages, encouraging not persisting it in some cases
- Newer users should feel comfortable reading and writing single-file packages
- A future possibility is allowing single-file packages to belong to a
  workspace at which point they would use the workspace's `Cargo.lock` file.
  This limits the scope of the conversation and allows an alternative to
  whatever is decided here.
- Read-only single-file packages (e.g. running `/usr/bin/package.rs` without root privileges)

How do we handle the `package.edition` field, balancing
single-file package use case needs (no boilerplate, modern
experience) with the expectations of Rust for reproducibility?
- Matching the expectation of a reproducible Rust experience
- Users wanting the latest experience, in general
- Boilerplate runs counter to experimentation and prototyping
- There might not be a backing file if we read from `stdin`

Smaller questions include:
- Should we support explicit stdin with `-`?  Implicit stdin?
- Should we support workspaces as part of the initial MVP?
- Whether single-file packages need a distinct file extension or not?
- What, if any, file associations should be registered on Windows?
- As single-file packages aren't auto discovered (e.g. `cargo test` being short
  for `cargo test --manifest-path Cargo.toml`), is there a way we can make
  running cargo commands on single-file packages more convenient?

Potential answers to these questions were intentionally left out to help focus
the conversation on the proposed experiment.  For a previous enumeration of
potential answers to these questions, see the [Pre-RFC on
Internals](https://internals.rust-lang.org/t/pre-rfc-cargo-script-for-everyone/18639).

# Future possibilities
[future-possibilities]: #future-possibilities

## Implicit `main` support

Like with doc-comment examples, we could support an implicit `main`.

Ideally, this would be supported at the language level
- Ensure a unified experience across the playground, `rustdoc`, and `cargo`
- `cargo` can directly run files rather than writing to intermediate files
  - This gets brittle with top-level statements like `extern` (more historical) or bin-level attributes

Behavior can be controlled through editions

## A REPL

See the [REPL exploration](https://github.com/epage/cargo-script-mvs/discussions/102)

In terms of the CLI side of this, we could name this `cargo shell` where it
drops you into an interactive shell within your current package, loading the
existing dependencies (including dev).  This would then be a natural fit to also have a `--eval
<expr>` flag.

Ideally, this repl would also allow the equivalent of `python -i <file>`, not
to run existing code but to make a specific file's API items available for use
to do interactive whitebox testing of private code within a larger project.
