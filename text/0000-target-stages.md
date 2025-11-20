- Feature Name: `target_stage`
- Start Date: 2025-10-25
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Communication between compiler activities via the incremental system to avoid more recompilations than necessary.

# Motivation
[motivation]: #motivation

The current model for incremental recompilations doesn't share progress between compiler activities, leading to unnecessary rebuilds. Users notice redundant compilations, as
"Changes in workspaces trigger unnecessary rebuilds" was submitted as [a big complaint in compiler performance][perf-survey].

Introducing a concept of truly incremental compilation helps for these cases which share a common base and would help with:

- `target` directory size; artifacts are shared between activities with a common base (e.g. `check` and `build` on the same crate and same flags)
- Compilation times, as e.g. `check` -> `build` allows building to start right from type-checking, and allows `check` to be skipped altogether.

The proposed solution is to add a notion of "compiler target stages", which would be served into `rustc` usually via Cargo with a `--target-stage=<ast|macro-expansion|hir|analyze-hir|...>`
flag. This flag would be denoted a special role in the compiler and it would have a special place in the incremental system.

The flag denotes which target the compiler can go up to, and serves as an early indicator of how the build was performed. This moves the burden of stage-indication from the compiler flags, and gives a reliant way for future compilations to evaluate if they should run, load a dependency graph into memory, or start from scratch altogether.

Disconnecting compiler flags from an early recompilation judgement allows us to almost completely overlap the various
compiler activities of different levels, and allow for more flexibility in activities other than `build` and `check`,
such as `clippy` or other existing and future projects that may need to partially build with the Rust compiler.

---

While not all cases overlap with "lower" compiler-adjacent activites (`check` would be lower than `build`) and those
cases would need a recompilation anyway, this allows us to [perform further optimizations][#further-optimizations] than just holding the two workflows. 

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation


The `--target-stage` flag is an option that instructs the compiler to only go so far when compiling your code. This
allows the compiler to perform numerous optimizations to avoid redundant recompilations.

Users don't usually have to worry about this flag, as it's handled automatically by Cargo via `cargo check` and `cargo build`.
For an explanation for RFC readers, see [Motivation](#motivation).

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There are two parts to this feature, one handled in the compiler itself and another handled by the incremental system
and heavily interacting with Cargo.


The easiest part is handled by the compiler, where "safe points to exit" are designated (such as in between of parsing
to AST and macro-expansion, or lowering THIR to MIR). Depending on the input that the `--target-stage` option takes,
we exit at one point or another.

Along with the public portion of the RFC, we also have a private new construct, called "Stage dependencies".
"Stage dependencies" are designated fields that, while they are handled by `Session`, they are not taken into account
when hashing, and thus, their values are not always loadbearing.

This means that stage dependencies only affect the hash of the necessary stage that depends on them.
Stage dependencies are also overlapping in the majority of cases, we'll look into it later.

For example, let's say that the user calls `cargo check` two times, with different `-Cstrip` values.
`cargo check` (in the local module) will not be affected by this change, as `-Cstrip` is a stage dependency on `linking` and beyond.
Being that `cargo check` doesn't change behaviour depending on the linking behaviour, it should **not** be recompiled.

Extracting compilation flags from hashing (and thus, from recompilation algorithms) allows us to also perform some post-processing,
like unifying lints or handling the reorder of flags in a command line.

---

Stage dependencies are mixed with target stages and the overlapping nature of the new incremental system, allowing us to re-use bases
and only process/store the parts that actually differ.

## Non-overlapping cases

Not all cases overlap, for example, while `-Cdebug-assertions` is marked as "codegen", it's a commonly used option for
detecting the profile of the compilation in `cfg`s. That is the reason that some stage dependencies are can affect
several stages and be enabled/disabled for some stages independently.

In the example of `-Cdebug-assertions`, the attribute parser would check for the presence of `#[cfg(debug_assertions)]`,
before and after macro expansion, and "enable" the stage dependency for all following stages if it's present at any
time during that parsing.

Note that stage dependencies are tracked even if `--target-stage` dictates to stop at that step, because future compilations
with higher target-stages can benefit from those stage dependencies being tracked.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?
- Potentially buggy release artifacts, if the compiler isn't careful enough, it might reutilize an artifact that it shouldn't and result in a buggy binary without any human error by the user.
- Coordination between teams and sub-projects is lengthy and a serious effort. I can expect that both Cargo and the compiler will be greatly involved, at least in some portion.
- A potential for bigger `target` sizes / quicker rise in `target` size, if the user uses multiple "bases", using each one with multiple, different "data dependencies". This is only theorical and depends on the final algorithm used, as the intent is for `target` sizes to be reduced.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- The current incremental system is an alternative. But users find that unnecessary rebuilds are a problem.
- This flag would override the currently unstable and untracked `no_analysis` option. Althought this flag currently
exists, it doesn't see any real use inside the compiler, nor does it have a tracking issue. It doesn't operate with
Cargo, and isn't designed to be as loadbearing as this RFC proposes.


# Prior art
[prior-art]: #prior-art

A few examples of what this can include are:

- [GCC IncrementalCompiler Project][gccincr], currently without a Delivery Date (last update; 2008).
- Clang has the [Clang-Repl project][clang-repl], a C++ interpreter which supports incremental compilation
- [Zig Incremental compilation][zig-incr], doesn't have a functional incremental compilation system for many use-cases.
- [Go (incremental?) compiler][go-page], supposedly incremental but I cannot find documentation on this or an implementation. 
- [Ocaml incremental compiler][ocaml-incr], the code is available, but not documented to an extensive degree.
- [This Project Goal][project-goal], that talks about a similar concept, but with a more primitive approach.

It seems that Rust currently has the most documented incremental compilation system.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- New design of the incremental directory.
- How will these target stages be handled when saving a dependency graph?
- Where will the stage dependencies be stored? Should we store them along the dependency graph?
- How does this effort interact with the [Relink, Don't Rebuild][rdr] project goal?

# Future possibilities
[future-possibilities]: #future-possibilities

This effort helps push forward towards a query-driven compiler, where all big "stages" of a compilation are
querified.

It also allows for better flexibility when interacting with 3rd party programs, and ad-hoc programs
to `rustc_driver` like Clippy and thus, allowing the share of cache between these.

<!-- More information will be provided for this section in the future -->

[perf-survey]: https://blog.rust-lang.org/2025/09/10/rust-compiler-performance-survey-2025-results/#incremental-rebuilds
[rdr]: https://rust-lang.github.io/rust-project-goals/2025h2/relink-dont-rebuild.html?highlight=relink%2C%20don#relink-dont-rebuild
[gccincr]: https://gcc.gnu.org/wiki/IncrementalCompiler
[clang-repl]: https://clang.llvm.org/docs/ClangRepl.html
[zig-incr]: https://github.com/ziglang/zig/issues/21165
[go-page]:https://go.dev
[ocaml-incr]: https://ocaml.org/manual/5.4/api/compilerlibref/type_CamlinternalMenhirLib.IncrementalEngine.html
[project-goal]: https://github.com/rust-lang/rust-project-goals/pull/367
