- Feature Name: `cargo_target_base_dir`
- Start Date: 2023-01-12
- RFC PR: [rust-lang/rfcs#3371](https://github.com/rust-lang/rfcs/pull/3371)
<!-- - Cargo Issue: [rust-lang/cargo#0000](https://github.com/rust-lang/cargo/issues/0000) -->

# Summary
[summary]: #summary

<!-- One paragraph explanation of the feature. -->

Introduce a new configuration option for cargo to tell it to move the crate/workspace's target directory into a crate/workspace-specific subdirectory of the configured absolute path,
named `CARGO_TARGET_BASE_DIR`.

# Motivation
[motivation]: #motivation

<!-- Why are we doing this? What use cases does it support? What is the expected outcome? -->

The original motivating issue can be found here: [rust-lang/cargo#11156](https://github.com/rust-lang/cargo/issues/11156).

1. Not having to find and clean all `target/` dirs everywhere while not having all projects collide (which is the effect of setting `CARGO_TARGET_DIR` globally)
1. Being able to easily exclude a directory from saves (Apple's Time Machine, ZFS snapshots, BRTS, ...)
1. Allows easily having separate directories for Rust-Analyzer and Cargo itself, allowing concurrent builds
   (technically already doable with arguments/env vars but `CARGO_TARGET_DIR` collides all projects into big target dir, leading to frequent recompilation because of conflicting features and locking builds)
1. Allows using a different disk, partition or mount point for cargo artifacts
1. Avoids having to set `CARGO_TARGET_DIR` for every project to get the same effect as proposed here

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

<!--
Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
- Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain?

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.
-->

For a single project, it is possible to use the `CARGO_TARGET_DIR` environment variable (or the `target-dir` TOML config option or the `--target-dir` command-line flag) to change the position of the `target/` directory used for build artifacts during compilation with Cargo.

While this option is useful for single-project environments (simple CI builds, builds through other build systems like Meson or Bazel), in multi-projects environment, like personal machines or repos with multiple workspaces, it conflates every build directory under the configured path: `CARGO_TARGET_DIR` directly replaces the `<workspace>/target/` directory.

`CARGO_TARGET_BASE_DIR` (or the `target-base-dir` TOML option or the `--target-base-dir` command-line flag) instead acts as a parent for those `target` directories.

Below is an example of the behavior with `CARGO_TARGET_DIR` versus the one with `CARGO_TARGET_BASE_DIR`:

## Example

Consider this directory tree:

```text
/Users/
├─ poliorcetics/
│  ├─ work/
│  │  ├─ work-project/
│  │  │  ├─ Cargo.toml
│  │  │  ├─ crate-1/
│  │  │  │  ├─ Cargo.toml
│  │  │  ├─ crate-2/
│  │  │  │  ├─ Cargo.toml
│  ├─ perso/
│  │  ├─ perso-1/
│  │  │  ├─ Cargo.toml
│  │  ├─ perso-2/
│  │  │  ├─ Cargo.toml

/cargo-cache/
```

#### With `CARGO_TARGET_DIR=/cargo-cache`

`cd /Users/poliorcetics/work/work-project && cargo build` produces artifacts directly in `/cargo-cache/debug/...`

A subsequent `cargo build` in `project-1` will work with the same artifact, potentially having conflicting features for dependencies for example.

A `cargo clean` will delete the entire `/cargo-cache` directory, for all projects at once.

It's possible to produce invalid state in the target dir by having unrelated projects writing in the same place.

It's not possible to have to projects building at once because Cargo locks its target directory during builds.

#### With `CARGO_TARGET_BASE_DIR=/cargo-cache`

`cd /Users/poliorcetics/work/work-project && cargo build` produces artifacts in `/cargo-cache/work-project-<hash>/debug/...`

A `cargo build` in `project-1` will produce new artifacts in `/cargo-cache/project-1-<hash>/debug/...`.

A `cargo clean` will only remove the `/cargo-cache/<project>-<hash>/` subdirectory, not all the artifacts.

In this situation, it's not possible for Cargo to produce invalid state without a `build.rs` deliberately writing outside its target directory.

Two projects can be built in parallel without troubles.

#### With both set

`CARGO_TARGET_DIR` was present long before `CARGO_TARGET_BASE_DIR`: backward compatibility is important, so the first always trumps the second,
there is no mixing going on.

#### Absolute and relative paths

`CARGO_TARGET_DIR` can be either a relative or absolute path, which makes sense since it's mostly intended for a single project, which can then
work from its own position to configure the target directory.

On the other hand `CARGO_TARGET_BASE_DIR` is intended to be used with several projects, possibly completely unrelated to each other. As such,
it does not accept relative paths, only absolute ones. If a compelling use case is present for a relative path, it can added in the future as a
backward-compatible change.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

<!--
This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.
-->

## Setting `CARGO_TARGET_BASE_DIR`

The option is similar to `CARGO_TARGET_DIR` and can be set in the same places. From less to most specific:

- Through the `config.toml`:

  ```toml
  [build]
  target-base-dir = "/absolute/path/to/target/directories"
  ```

- Through the environment variable: `CARGO_TARGET_BASE_DIR="/absolute/path/to/target/directories" cargo build`
- Through the command line flag: `cargo build --target-base-dir /absolute/path/to/target/directories`

The given path must be absolute: setting `CARGO_TARGET_BASE_DIR` to an empty or relative path is an error (when used and not instantly overriden by `CARGO_TARGET_DIR`).

## Resolution order relative to `CARGO_TARGET_DIR`

The resolution order favors `CARGO_TARGET_DIR` in all its forms, in the interest of both backward compatibility and allowing overriding for a singular workspace:

`--target-dir` > `CARGO_TARGET_DIR` > `target-dir = ...` > `--target-base-dir` > `CARGO_TARGET_BASE_DIR` > `target-base-dir = ...`

See the final section for discussion on how to make `CARGO_TARGET_BASE_DIR` the default.

## Naming

In the example in the previous section, using `CARGO_TARGET_BASE_DIR` with `cargo build` produces named subdirectories. The name of those is partially deterministic:
it is the name of the parent directory of the workspace's `Cargo.toml` manifest and an unspecified hash of the absolute path to the workspace's root, so building `work-project/crate-1` will still use the `/cargo-caches/work-project-<hash>/debug/...` directory for a `cargo build` call.

This naming scheme is chosen to be simple for people to navigate but is **not considered stable**: the hashing method (and so the hash) will probably not change often but `cargo` offers no guarantee and may change it in any release. Tools that needs to interact with `cargo`'s target directory should not rely on its value for more than a single invocation of them: they should instead query `cargo metadata` for the actual value each time they are invoked.

In case the parent directory is `/` or `C:\`, the subdirectory name is implementation defined.

The path used for the hashing and naming of the final directory is the one found *after* symlink resolution: `bazel` does it too and I have not found any complaints about this and it has the distinct advantage of allowing to make a symlink to a project somewhere else on the machine (for example to organise work projects) and avoid duplicating the build directory and all its data (which can be quite heavy).

### Symbolic links

In the following situation

```
/Users/
├─ poliorcetics/
│  ├─ projects/
│  │  ├─ actual-crate/
│  │  │  ├─ Cargo.toml
│  │  ├─ symlink-to-crate/ -> actual-crate/
```

When calling `cargo metadata` in the `symlink-to-crate` path, the result contains `"manifest_path": "/Users/poliorcetics/projects/actual-crate/Cargo.toml"` and `"workspace_root":"/Users/poliorcetics/projects/actual-crate"`. This behaviour means that symlinks won't change the final directory used inside `CARGO_TARGET_BASE_DIR`, or in other words: symbolic links are resolved.

## Impact on `cargo ...` calls

When calling `cargo` where `CARGO_TARGET_BASE_DIR` is active, `CARGO_TARGET_DIR` is set by all `cargo` calls that happen in a Cargo workspace, including calls to third-party tools.

In the same vein, `cargo metadata` will fill the target directory information with the absolute path and make no mention of `CARGO_TARGET_BASE_DIR` since it can only be used in a single workspace at once.

### `cargo clean`

Currently, if `CARGO_TARGET_DIR` is set to anything but `target` for a project, `cargo clean` does not delete the `target/` directory if it exists, instead deleting the directory pointed by `CARGO_TARGET_DIR`. The same behavior is used for `CARGO_TARGET_BASE_DIR`: if it set, `cargo clean` will delete `CARGO_TARGET_BASE_DIR/<project>-<hash>` and not `target/`.

# Drawbacks
[drawbacks]: #drawbacks

<!-- Why should we *not* do this? -->

## One more option to find the target directory

This introduces one more option to look at to find the target directory, which may complicate the life of external tools.

This is mitigated by having `CARGO_TARGET_DIR` entirely override `CARGO_TARGET_BASE_DIR`, so an external tool can set it and go on its way.
Also, having `cargo` set `CARGO_TARGET_DIR` when inside a workspace where `CARGO_TARGET_BASE_DIR` is used will help current tools (those not
yet using `cargo metadata`) continue working without trouble.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

<!--
- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?
-->

## Do nothing

It is already possible today to use `CARGO_TARGET_DIR` to remap workspaces and projects but this has two problems:

1) If done globally, the `CARGO_TARGET_DIR` becomes a hodge-podge of every project, which is not often the goal.
2) If done per-project, it is very cumbersome to maintain.
3) [`targo`](https://github.com/sunshowers/targo) by @sunshowers
4) [rust-lang/cargo#11156](https://github.com/rust-lang/cargo/issues/11156)

`targo` and the cargo issue express a need for either remapping or a global target directory that is not shared between different Cargo workspaces.

For those reason, this option has not been retained and the `targo` tool is discussed more in details below.

## Using `XDG_CACHE_HOME` instead of a cargo-specific env-var

Not all OSes use the XDG convention, notably Windows and macOS (though the latter can be somewhat made to) and it is
very easy to define `CARGO_TARGET_BASE_DIR=${XDG_CACHE_HOME:-~/.cache}/cargo_target_base_dir` if wanted by users.

## Just put the directories inside of `.cargo/cache/...`

There are already lots of discussion about `.cargo` and `.rustup` being home to both cache and config files and why this
is annoying for lots of users. What's more, it would not be as helpful to external build tools, they don't care about
bringing the registry cache in their build directory for example.

## Stabilize the naming scheme

I feel this require an hard-to-break naming scheme (a recent hash algorithm should be good enough in 99% of the cases but collisions are always possible), which I don't have the skills nor motivation to design. Instead, I prefer explicitely telling the naming scheme is not to be considered stable and allow more invested people to experiment with the feature and find something solid.

What's more, by explicitely not stabilizing it (and maybe voluntarily changing it between versions sometimes, since a version change recompiles everything anyway ?) we can instead reroute people and tools towards `CARGO_TARGET_DIR` / `cargo metadata` instead, which are much more likely to be suited to their use case if they need the path to the target directory.

## Use only the hash, not the name of the workspace directory

While this solution is just as easy to work with for tools, I think having a somewhat human-readable part in the name make it easier to work with through things like logging or just listing the subdirectories in `CARGO_TARGET_BASE_DIR`. It also makes the path printed by `cargo run` or
the local URL used when calling `cargo doc --open` much clearer.

Ultimately, tools are not negatively affected since they should be using `CARGO_TARGET_DIR` or `cargo metadata` and humans have an easier time working with such paths so I think it's worthwhile to include the workspace's name.

## Just use `targo`

While a very nice tool, [`targo`](https://github.com/sunshowers/targo) is not integrated with `cargo` and has a few shortcomings:

- It uses symlinks, which are not always handled well by other tools. Specifically, since it's not integrated inside `cargo`, it uses a `target` symlink to avoid having to remap `cargo`'s execution using `CARGO_TARGET_DIR` and such,making it less useful for external build tools that would use this functionality. Using such a symlink also means `cargo clean` does not work, it just removes the symlink and not the data.
- It completely ignores `CARGO_TARGET_DIR`-related options, which again may break workflows.
- It needs more metadata to work well, which means an external tool using it would have to understand that metadata too.
- It uses `$CARGO_HOME/targo` to place its cache, making it less useful for external build tools and people wanting to separate caches and configuration.
- It needs to intercept `cargo`'s arguments, making it more brittle than an integrated solution.

Some of those could be fixed of course, and I don't expect `cargo`'s `--target-dir` and `--manifest-path` to change or disappear anytime soon, but still, it could happen. An external tool like `targo` will never be able to
solve some of these or ensure forward compatibility as well as the solution proposed in this RFC.

On the other hand, `targo` is already here and working for at least one person, making it the most viable alternative for now.

### Providing backlinks

`targo` provides backlink (it links from it's own target directory to `<workspace>/target`) as a way for existing tools to continue working despite there being no `CARGO_TARGET_DIR` set for them to find the real target dire.

`cargo` does not for `CARGO_TARGET_DIR` and will not do it either for `CARGO_TARGET_BASE_DIR` : it provides `cargo metadata` that is the blessed way to obtain the actual target directory and when `CARGO_TARGET_BASE_DIR` is used, it will set `CARGO_TARGET_DIR` on all invocation (if not already set) to make it easy to obtain the target directory for simple task (e.g. a test needing to launch another binary in the repo).

## Remapping

[rust-lang/cargo#11156](https://github.com/rust-lang/cargo/issues/11156) was originally about remapping the target directory, not about having a
central one but reading the issue, there seems to be no needs for more than the simple redefinition of the target directory proposed in this document.
In the future, if `CARGO_TARGET_DIR_REMAP` is introduced, it could be used to be the prefix to the target directory like so:

- Set `CARGO_TARGET_DIR_REMAP=/home/user/projects=/tmp/cargo-build`
- Compile the crate under `/home/user/projects/foo/` **without** `CARGO_TARGET_DIR` or `CARGO_TARGET_BASE_DIR` set
- The resulting target directory will be at `/tmp/cargo-build/foo/target`

By making the priority order `CARGO_TARGET_DIR` > `CARGO_TARGET_BASE_DIR` > `CARGO_TARGET_DIR_REMAP` (when all are absolute paths) we would keep backward compatibility. Or we could disallow having the last two set at once, so that they're alternatives and not ordered.

When `CARGO_TARGET_DIR` is relative, the result could be `/tmp/cargo-build/foo/$CARGO_TARGET_DIR`.

Overall, I feel remapping is much harder to implement well and can be added later without interfering with `CARGO_TARGET_BASE_DIR` (and without
this RFC interfering with remapping), though the design space is probably bigger than the one for this RFC.

# Prior art
[prior-art]: #prior-art

<!--
Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.
-->

## `bazel`

The [`bazel`](https://bazel.build) build system has a similar feature called the [`outputRoot`](https://docs.bazel.build/versions/5.4.0/output_directories.html), which is always active and has default directories on all major build/development platforms (Linux, macOS, Windows).

The naming scheme is as follow: `<outputRoot>/_bazel_$USER/` is the `outputUserRoot`, used for all builds done by `$USER`. Below that, projects are identified by the MD5 hash of the path name of the workspace directory (computed after resolving symlinks).

The `outputRoot` can be overridden using `--output_base=...` (this is `$CARGO_TARGET_BASE_DIR`, the subject of this RFC) and the `outputUserRoot` with `--output_user_root=...` (this is close to using `$CARGO_TARGET_DIR`, already possible in today's `cargo`).

It should be noted that `bazel` is integrated with [remote caching](https://bazel.build/remote/caching) and has different needs from `cargo`, the latter only working locally.

**Conclusion**: `bazel` shows that a hash-based workflow seems to work well enough, making an argument for the use of it in `cargo` too. It also uses the current user, to prevent attacks by having compiled a program as root and making the directory accessible to other users later on by also compiling there for them. `cargo` could also do this, though I do not know what happens when `--output_user_root` is set to the same path for two different users.

*Note: I looked at Bazel 5.4.0, the latest stable version as of this writing, things may change in the future or be different for older versions.*

# Unresolved questions
[unresolved-questions]: #unresolved-questions

<!--
- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?
-->

- Do we want to differentiate according to users ? `bazel` is a generic build tool, whereas `cargo` is not, so maybe differentiating on users is not necessary for us ?

## Handle possibly thousands of directories in a single `CARGO_TARGET_BASE_DIR` path

While a single dev machine is unlikely to have enough projects that the simple scheme of `<project name>-<hash>` will produce enough directories to slow down working in `$CARGO_TARGET_BASE_DIR/`, it could still happen, and notably in private CI, which are often less compartimentalized than public ones. Simple cruft over time (i.e, never calling `cargo clean` over years) could also make it happen, if much slower.

A quick design that would probably work and still be relatively navigable by humans is: instead of `project-name-<hash>/`, use `p/r/o/j/e/ct-name-<hash>/`. Here I used an arbitrary limit of 5 subdirectories but really, anything could be used. A problem occurs if a `.` is present in the directory name,but in that case we could simply append it to the symbol before like so: `proj....name/` -> `p/r/o/j..../n/ame-<hash>/`.

If the project name contains a space in the first 5 symbols, maybe replace it like so: `proj name/` -> `p/r/o/j-space/n/ame-<hash>/`.

I don't know of any filesystem that has troubles with `_` or `-` (often used as delimiters) so those have no special handling.

# Future possibilities
[future-possibilities]: #future-possibilities

<!--
Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
-->

- Allowing relative paths: I feel this is counter-productive to the stated goal and have thought of no use for it, but it's entirely possible someone else will.
- Introduce remapping into the concept in some way.

## Use `CARGO_TARGET_BASE_DIR` as the default instead of `target`

This option has several complications I'm not sure how to resolve:

1. How do we decide on good platform defaults ?
  - Subsequently, when platform defaults are decided, how do we ensure a new platform has a good default too ?
2. How do we communicate on said default values ?
3. This would probably break backward compatibility and lots of tools ? We could heavily advertise the option in the Rust book and Cargo's documentation but making it the default is probably not something we will be able (or even willing) to do any time soon.

I see `CARGO_TARGET_BASE_DIR` as `cargo`-specific `XDG_CACHE_DIR` variable: if unset, using a local `target/` is what *all* of the Web has documented for years, it's what people expect, it's what tools expect, it makes it easy to reorganize Rust projects by just moving them around in directories, ...

`CARGO_TARGET_BASE_DIR` is more intended for CI, peoples with specific setups or people wanting to dive just a little deeper in their directory management, but just like `npm` puts `.node_modules/` in the project's workspace directory by default, I think `cargo` should also keep this as the default behaviour.

### We really want to do this, how do we do it ?

Well, first, heavy advertising of the option. Very **heavy** advertising of it and of `cargo-metadata` for all interactions with the cargo `target/` directory. After that, it will be a waiting game:

1. Wait for tools to be updated
2. Wait for MSRVs to catch up to the cargo version introducing the option and its defaults (this could take a year or ten, I have no idea)
3. Add a config option to `cargo` to make it the default behaviour (when there is not already a `target/` dir locally is `CARGO_TARGET_DIR` set), something like `use-user-wide-cargo-target-base-dir-by-default = true` and communicate so that people can try it out
4. Write a post saying "we'll do the change in version X" (where current version is like X-2 to warn 3 months before at least ?) and then only apply the change to directory where there is no `CARGO_TARGET_DIR` and no `target/` dir locally

I expect this to take a long time and `cargo` would need to be very clear about where it puts the target directory, probably through a simple command like `cargo metadata --print-target-dir` to make it easy for CIs and scripts to use it programatically without having to parse JSON all the time like a simple `cargo metadata` would do.
