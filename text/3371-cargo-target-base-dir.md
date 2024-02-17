- Feature Name: `cargo_target_dir_templates`
- Start Date: 2023-01-12
- RFC PR: [rust-lang/rfcs#3371](https://github.com/rust-lang/rfcs/pull/3371)
<!-- - Cargo Issue: [rust-lang/cargo#0000](https://github.com/rust-lang/cargo/issues/0000) -->

# Summary
[summary]: #summary

<!-- One paragraph explanation of the feature. -->

Introduce templating to `CARGO_TARGET_DIR` to have `cargo` adapts its target directory dynamically depending on (at least) the manifest's path.

# Motivation
[motivation]: #motivation

<!-- Why are we doing this? What use cases does it support? What is the expected outcome? -->

The original motivating issue can be found here: [rust-lang/cargo#11156](https://github.com/rust-lang/cargo/issues/11156).

1. Not having to find and clean all `target/` dirs everywhere while not having all projects collide (which is the effect of setting `CARGO_TARGET_DIR` globally)
1. Being able to easily exclude a directory from backups (Apple's Time Machine, ZFS and btrfs snapshots, ...)
1. Allows easily having separate directories for Rust-Analyzer and Cargo itself, allowing concurrent builds (technically already doable with arguments/env vars but `CARGO_TARGET_DIR` collides all projects into big target dir, leading to frequent recompilation because of conflicting features and locking builds)
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

For a single project, it is possible to use the `CARGO_TARGET_DIR` environment variable (or the `target-dir` TOML config option or the `--target-dir` command-line flag) with either an absolute or relative path to change the position of the `target/` directory used for build artifacts during compilation with Cargo.

While this option is useful for single-project environments (simple CI builds, builds through other build systems like Meson or Bazel), in multi-projects environment, like personal machines or repos with multiple workspaces, it conflates every build directory under the configured path: `CARGO_TARGET_DIR` directly replaces the `<workspace>/target/` directory.

Templating introduces one new templating key for `CARGO_TARGET_DIR`, in the same spirit as [the index configuration format][icf]:

- `{manifest-path-hash}`: a hash of the manifest's absolute path as a path. This is **not** an absolute path.

It can be used like this: `CARGO_TARGET_DIR="$HOME/.cache/cargo-target-dirs/{manifest-path-hash}"`.

When compiling `/home/ferris/src/cargo/` with user `ferris`, `manifest-path-hash` would be something like `ab/cd/<rest of hash>` and the artifacts would be found in `/home/ferris/.cache/cargo-target-dirs/ab/cd/<rest of hash>/...`. Note the hash used and the path derived from that for `{manifest-path-hash}` are implementation details and the values here are just example.

Below is an example of the behavior with untemplated versus templated forms:

[icf]: https://doc.rust-lang.org/cargo/reference/registry-index.html#index-configuration

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

A subsequent `cargo build` in `perso-1` works with the same artifact, potentially having conflicting features for dependencies for example.

A `cargo clean` deletes the entire `/cargo-cache` directory, for all projects at once.

It's possible to produce invalid state in the target dir by having unrelated projects writing in the same place.

It's not possible to have to projects building at once because Cargo locks its target directory during builds.

#### With `CARGO_TARGET_DIR=/cargo-cache/{manifest-path-hash}`

`cd /Users/poliorcetics/work/work-project && cargo build` produces artifacts in `/cargo-cache/<manifest-path-hash>/debug/...` (where `manifest-path-hash` is a directory or several chained directories unique to the workspace, with an unspecified naming scheme).

A `cargo build` in `perso-1` produces new artifacts in `/cargo-cache/<manifest-path-hash>/debug/...`.

A `cargo clean` only removed the `/cargo-cache/<manifest-path-hash>/` subdirectory, not all the artifacts for all other projects that are also in the cache.

In this situation, it's much less likely for Cargo to produce invalid state without a `build.rs` deliberately writing outside its target directory.

Two projects can be built in parallel without troubles.

#### Absolute and relative paths

`CARGO_TARGET_DIR` can be either a relative or absolute path, which makes sense since it's mostly intended for a single project, which can then work from its own position to configure the target directory, and that stays the case with templates.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

<!--
This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.
-->

## Templating values of `CARGO_TARGET_DIR`

Templating does not interfere with the resolution order of `CARGO_TARGET_DIR`. From less to most specific:

- Through the `config.toml`:

  ```toml
  [build]
  target-base-dir = "/absolute/path/to/cache/{manifest-path-hash}"
  ```

- Through the environment variable: `CARGO_TARGET_DIR="/absolute/path/to/cache/{manifest-path-hash}" cargo build`
- Through the command line flag: `cargo build --target-dir "/absolute/path/to/cache/{manifest-path-hash}"`

## Naming

In the example in the previous section, using a templated `CARGO_TARGET_DIR` with `cargo build` produces named subdirectories. The name of those is computed from the full and canonicalized path to the manifest for the workspace, so building `work-project/crate-1` will still use the directory for the whole workspace during a `cargo build` call.

This naming scheme is **not considered stable**: the method will probably not change often but `cargo` offers no guarantee and may change it in any release. Tools that needs to interact with `cargo`'s target directory should not rely on its value for more than a single invocation of them: they should instead query `cargo metadata` for the actual value each time they are invoked.

The path used for the naming of the final target directory is the one found *after* symlink resolution: `bazel` does it too and I have not found any complaints about this and it has the distinct advantage of allowing to make a symlink to a project somewhere else on the machine (for example to organise work projects) and avoid duplicating the build directory and all its data (which can be quite heavy).

To prevent collisions by craftings paths, the `<manifest-path-hash>` directory will be computed from a hash of the workspace manifest's full path (and possibly other data, for example `bazel` uses its version and the current user too).

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

When calling `cargo metadata` in the `symlink-to-crate` path, the result contains `"manifest_path": "/Users/poliorcetics/projects/actual-crate/Cargo.toml"` and `"workspace_root":"/Users/poliorcetics/projects/actual-crate"`. This behaviour means that symlinks won't change the final directory used inside `{manifest-path-hash}`, or in other words: symbolic links are resolved.

### Handle possibly thousands of directories in a single templated `CARGO_TARGET_DIR` path

While a single dev machine is unlikely to have enough projects that the naming scheme of `<manifest-path-hash>` will produce enough directories to slow down working in `$CARGO_TARGET_DIR/`, it could still happen, and notably in private CI, which are often less compartimentalized than public ones. Simple cruft over time (i.e, never calling `cargo clean` over years) could also make it happen, if much slower.

To prevent this, `cargo` splits the hash into something like `$CARGO_TARGET_DIR/hash[:2]/hash[2:4]/hash[4:]/...`. Since the naming scheme is considered an implementation detail, if this prove insufficient it could be changed in a subsequent version of `cargo`.

## Impact on `cargo ...` calls

When calling `cargo` with a builtin call (e.g., `build`, `check` or `test`) where a templated `CARGO_TARGET_DIR` is active, `cargo` will first resolve the effective `CARGO_TARGET_DIR` and then proceed with the command as if `CARGO_TARGET_DIR` had been set directly. For third party tools (`cargo-*`), where cargo does not know about the relevant `Cargo.toml`, the tool will have to use [`cargo_metadata`](https://docs.rs/cargo_metadata), as is already expected today, to learn about the effective target directory.

In the same vein, `cargo metadata` fills the target directory information with the absolute path and make no mention of the template in `CARGO_TARGET_DIR` since it can only be used with a single workspace at once.

### `cargo clean`

Currently, if `CARGO_TARGET_DIR` is set to anything but `target` for a project, `cargo clean` does not delete the `target/` directory if it exists, instead deleting the directory pointed by `CARGO_TARGET_DIR`. The same behavior is used for the templated version: if it set, `cargo clean` deletes `/path/to/<manifest-path-hash>/` and not `target/`.

### Providing forward links

[`targo`][tg] provides forward link (it links from `<workspace>/target` to its own target directory) as a way for existing tools to continue working despite there being no explicit `CARGO_TARGET_DIR` set for them to find the real target directory.

`cargo` does not provide them for regular (untemplated) `CARGO_TARGET_DIR`. This is not a limitation when using the environment variable set globally, since all processes can read it, but it is one when this config is only set on specific calls or via `target-dir` in the config, meaning others tools cannot easily pick it up (and most external tools don't use `cargo-metadata`, which makes them all broken by default, but fixing this situation is not this RFC's purpose).

When a templated `CARGO_TARGET_DIR` is used (in any form) (and not overriden, for example a templated env var overriden by a precise `--target-dir` option), it *will* use a forward link by adding a `target` symlink to the real target directory. This `target` symlink will be in the exact place the real target directory would have been if the templated `CARGO_TARGET_DIR` wasn't set at all.

This has a two big advantages: not breaking external tools and giving easy access to artifacts produced by `cargo build/test/doc` to users (they're in the habit of typing `./target/debug/my-bin`, this would continue working with forward links).

A config option (CLI, `config.toml` and env var), `link-target-dir`, will be introduced to deactivate this behaviour, but it will `true` by default, for the reasons provided in favor of forward links just above.

#### Detailed working of forward links

When creating a forward link `cargo` will first attempt to create a symbolic link (regardless of the platform). If that fails, it will attempt zero or more platform-specific solutions, like junction points on NTFS. If that fails too, a warning will be emitted but this will not prevent the rest of the action to go on: regular calls like `cargo check/clippy/build/test` likely won't need this forward link and after the user has been warned they could either resolve the problem themselves or ignore it, depending on their own use case and domain-specific knowledge.

### Providing backlinks

Backlinks are metadata in templated `CARGO_TARGET_DIR` that links target directories back to the workspace they came from.

[`targo`][tg] uses them in its own form of the feature and `cargo` uses them too for templated `CARGO_TARGET_DIR`.

While details of the stored data are left to the implementation (there is no need for `cargo` to expose this data directly, though it could be exposed through `cargo metadata` in the future, see the relevant section below), one could imagine using it to clean target directories whose corresponding workspace does not exist anymore when calling something like `cargo clean --all-workspaces` (doing it automatically is not possible, else any workspace on external disks would have its target directory cleaned up each time the disk is unmounted, which is way too aggressive a default).

# Drawbacks
[drawbacks]: #drawbacks

<!-- Why should we *not* do this? -->

## One more option to find the target directory

This introduces one more option to look at to find the target directory, which may complicate the life of external tools.

This is mitigated by the forward link provided by default by `cargo` when using the templated form of `CARGO_TARGET_DIR`.

## Hitting windows path length limits

Depending on what naming scheme is used (e.g., a very long hash), we could hit the Windows path length limits if not careful.

A mitigation for this is recommending a short prefix (in `CARGO_TARGET_DIR`) and using a hash that doesn't include that many characters but those are only mitigations and do not fully fix the underlying problem.

## Forward links

There a few cases where a symlink instead of a real dir will break programs: at least SQLite 3 can be configured to raise an error if the database is behind a symlink anywhere in its opening path, it's probably other programs can also be configured to check this (or do it by default). Since templated `CARGO_TARGET_DIR` won't become a default in this RFC, we are not breaking any existing use cases.

## Transition period

During the transition period, any `CARGO_TARGET_DIR` that was defined as containing `{manifest-path-hash}` will change meaning. `cargo`, for at least one stable version of Rust, should provide warnings about this and point to either this RFC or its documentation to explain why the incompatiblity arised and how to fix it. In practice, paths with `{` or `}` in it are unlikely, even more with the exact key used by cargo here, so maybe no one will ever see the warning, but it's better than silently breaking workflows.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

<!--
- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?
-->

## Do nothing

It is already possible today to use `CARGO_TARGET_DIR` to remap workspaces and projects but this has a few problems:

1) If done globally, the `CARGO_TARGET_DIR` becomes a hodge-podge of every project, which is not often the goal.
2) If done per-project, it is very cumbersome to maintain.
3) [`targo`][tg] by @sunshowers
4) [rust-lang/cargo#11156](https://github.com/rust-lang/cargo/issues/11156)
5) The upcoming `cargo script` command needs someplace to put its cache and having a dedicated directory for that would be nice.

[`targo`][tg] and the cargo issue express a need for either remapping or a global target directory that is not shared between different Cargo workspaces.

For those reason, this option has not been retained and the [`targo`][tg] tool is discussed more in details below.

## Just put the directories inside of `.cargo/cache/...`

There are already lots of discussion about `.cargo` and `.rustup` being home to both cache and config files and why this is annoying for lots of users. What's more, it would not be as helpful to external build tools, they don't care about bringing the registry cache in their build directory for example.

## Stabilize the naming scheme

This require an hard-to-break naming scheme (a recent hash algorithm should be good enough in 99% of the cases but collisions are always possible), which is something the `cargo` team probably does not want to offer guarantees about. Instead, explicitely telling the naming scheme is not to be considered stable allows more invested people to experiment with the feature and find something solid if stability proves itself necessary.

What's more, by explicitely not stabilizing it (and maybe voluntarily changing it between versions sometimes, since a version change recompiles everything anyway ?) `cargo` can instead reroute people and tools towards untemplated `CARGO_TARGET_DIR` / `cargo metadata` instead, which are much more likely to be suited to their use case if they need the path to the target directory.

## Just use `targo`

While a very nice tool, [`targo`][tg] is not integrated with `cargo` and has a few shortcomings:

- It uses symlinks, which are not always handled well by other tools. Specifically, since it's not integrated inside `cargo`, it uses a `target` symlink to avoid having to remap `cargo`'s execution using `CARGO_TARGET_DIR` and such,making it less useful for external build tools that would use this functionality. Using such a symlink without setting the `CARGO_TARGET_DIR` env var also means `cargo clean` does not work, it just removes the symlink and not the data.
- It completely ignores `CARGO_TARGET_DIR`-related options, which again may break workflows.
- It needs more metadata to work well, which means an external tool using it would have to understand that metadata too.
- It uses `$CARGO_HOME/targo` to place its cache, making it less useful for external build tools and people wanting to separate caches and configuration.
- It needs to intercept `cargo`'s arguments, making it more brittle than an integrated solution.
- Its naming scheme is a base58-encoded blake3 hash of the workspace directory ([source]), not taking into account the use case of thousands of target directories within `$CARGO_HOME/targo`.
- It uses the workspace root dir and not manifest, which means a `targo script` would share cache between all the scripts (`cargo script`) in a directory, which may not be the desired effect.

Some of those could be fixed of course, and I don't expect `cargo`'s `--target-dir` and `--manifest-path` to change or disappear anytime soon, but still, it could happen. An external tool like [`targo`][tg] will never be able to solve some of these or ensure forward compatibility as well as the solution proposed in this RFC.

On the other hand, [`targo`][tg] is already here and working for at least one person, making it the most viable alternative for now.

[source]: https://github.com/rust-lang/cargo/issues/11156#issuecomment-1285951209

## Remapping

[rust-lang/cargo#11156](https://github.com/rust-lang/cargo/issues/11156) was originally about remapping the target directory, not about having a central one but reading the issue, there seems to be no needs for more than the simple redefinition of the target directory proposed in this document. In the future, if `CARGO_TARGET_DIR_REMAP` is introduced, it could be used to be the prefix to the target directory like so:

- Set `CARGO_TARGET_DIR_REMAP=/home/user/projects=/tmp/cargo-build`
- Compile the crate under `/home/user/projects/foo/` **without** `CARGO_TARGET_DIR` set
- The resulting target directory will be at `/tmp/cargo-build/foo/target`

By making the priority order `CARGO_TARGET_DIR` > `CARGO_TARGET_DIR_REMAP` (when both are absolute paths) we would keep backward compatibility. Or we could disallow having the two set at once, so that they're alternatives and not ordered.

When `CARGO_TARGET_DIR` is relative, the result could be `/tmp/cargo-build/foo/$CARGO_TARGET_DIR`.

Overall, I feel remapping is much harder to implement well and can be added later without interfering with templates in `CARGO_TARGET_DIR` (and without this RFC interfering with remapping), though the design space is probably bigger than the one for this RFC.

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

The `outputRoot` can be overridden using `--output_base=...` (this is the untemplated `$CARGO_TARGET_DIR` when it is used with a template) and the `outputUserRoot` with `--output_user_root=...` (this is close to using `$CARGO_TARGET_DIR`, already possible in today's `cargo`).

It should be noted that `bazel` is integrated with [remote caching](https://bazel.build/remote/caching) and has different needs from `cargo`, the latter only working locally.

**Conclusion**: `bazel` shows that a hash-based workflow seems to work well enough, making an argument for the use of it in `cargo` too. It also uses the current user, to prevent attacks by having compiled a program as root and making the directory accessible to other users later on by also compiling there for them. `cargo` could also do this, though it is not clear what happens when `--output_user_root` is set to the same path for two different users.

*Note: Bazel 5.4.0 was used as the reference, the latest stable version as of this writing, things may change in the future or be different for older versions.*

# Unresolved questions
[unresolved-questions]: #unresolved-questions

<!--
- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?
-->

- Do we want to add forward links for untemplated `CARGO_TARGET_DIR` too in the process of this RFC ?
- Do we want to differentiate according to users ? `bazel` is a generic build tool, whereas `cargo` is not, so maybe differentiating on users is not necessary for the latter ?

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

- Introduce remapping into the concept in some way.
- Introduce a form of garbage collection. Expanded upon this [Zulip thread](https://rust-lang.zulipchat.com/#narrow/stream/246057-t-cargo/topic/last-use.20tracking).

## Adding `XDG_CACHE_HOME`, `HOME` or `CARGO_HOME` as templates

Not all OSes use the XDG convention, notably Windows and macOS (though the latter can be somewhat made to) and it is very easy to define `CARGO_TARGET_DIR=${XDG_CACHE_HOME:-~/.cache}/cargo-target-directories/{manifest-path-hash}` if wanted by users.

It won't work in the `config.toml` but it will work with the environment variable and the command line option, both of which override the TOML config.

It is certainly possible to add at least `{home}` and `{cargo_home}` but it can be done in the future without interfering at all with `{manifest-path-hash}`, making it relatively easy to defer and add if there is demand.

## Use templated `CARGO_TARGET_DIR` as the default instead of `target`

This option has several complications I'm not sure how to resolve:

1. How do we decide on good platform defaults ?
  - Subsequently, when platform defaults are decided, how do we ensure a new platform has a good default too ?
  - `CARGO_HOME` is already criticized for being both a *cache* and a *config* home (using the XDG spec semantics), adding more local cache to it in the form of `CARGO_HOME/target-base-dir/` would not improve the situation and should probably not be done, but, if no good alternatives are found, there is precedent to use it for this.
2. How do we communicate on said default values ?
3. This would probably break backward compatibility and lots of tools ? We could heavily advertise the option in the Rust book and Cargo's documentation but making it the default is probably not something we will be able (or even willing) to do any time soon. Note that having forward links active by default (see relevant section earlier in the RFC) will help offset a lot of the problems here.

### We really want to do this, how do we do it ?

Well, first, advertising of the option and its behaviour, as well as the forward link behaviour (so people know we're not just breaking tools for fun). After that, it becomes necessary to test it quite heavily to really ensure nothing has broken irremediably.

1. Introduce it as the default behaviour in nightly, wait for a few stable releases so that beta has it too and it can start being used for a few months after that so that esoteric setups can also try it.
2. Write a post saying "we'll do the change in version X" (where current version is like X-2 to warn 3 months before at least ?) and then only apply the change to projects where there is no untemplated `CARGO_TARGET_DIR` config set.

### A user still wants the current behaviour and not this RFC's

- We could use a config option to entirely deactivate it
- `CARGO_TARGET_DIR=target` would still be available
- We could add special behaviour like `CARGO_TARGET_BASE_DIR=""` meaning "use current" directory

The first two are probably enough, the third is a bandaid.

### Expose template metadata

`cargo` will use backlinks in an implementation-defined form to keep track in the templated `CARGO_TARGET_DIR` of the relation from a target directory to its source workspace.

In the future, we could envisage letting external tools and users access this data in a well-defined form through `cargo metadata`.

[tg]: https://github.com/sunshowers/targo
