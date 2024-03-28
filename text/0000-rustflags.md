- Feature Name: `cargo_cli_rustflags`
- Start Date: 2022-09-01
- RFC PR: [rust-lang/rfcs#3310](https://github.com/rust-lang/rfcs/pull/3310)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC aims to improve the experience of enabling Rust compiler flags for specific crates when building Rust projects
by adding a new option, `--rustflags <RUSTFLAGS>`. This would have the same effect as `cargo rustc -- <RUSTFLAGS>` but would
also be available for use by other subcommands such as `bench, build, check, doc, fix, install, package, run` and `test`.
This allows a Rust project to be built and tested for instance without forcing a new compilation and losing the rustflags that
were set when invoking `cargo rustc`. This option sets `<RUSTFLAGS>` for the current crate only and not all crates in the
dependency graph.

# Motivation
[motivation]: #motivation

Today, there currently exists multiple ways of specifying `RUSTFLAGS` to pass to invocations of the Rust compiler.
All of the existing ways have the limitation of not being able to specify which invocation of rustc the compiler flag
is set for. When a Rust developer tries to enable a Rust compiler option for the current crate being built, they will also
have this compiler option set for all dependencies. With the feature proposed by this RFC, a Rust developer can simply use
the cargo CLI to pass rustflags for the current crate without having to worry about how other crates in dependency graph
might be affected.

Some of the existing support for rustflags in Cargo are:

1. `CARGO_ENCODED_RUSTFLAGS` environment variable
2. `RUSTFLAGS` environment variable
3. `target.*.rustflags` from the config (.cargo/config)
4. `target.cfg(..).rustflags` from the config (.cargo/config)
5. `host.*.rustflags` from the config (.cargo/config) if compiling a host artifact or without `--target`
6. `build.rustflags` from the config (.cargo/config)

These currently all override one another in the order specified, meaning if `CARGO_ENCODED_RUSTFLAGS` is set, none of the other rustflags
options are even considered. The same is true if `CARGO_ENCODED_RUSTFLAGS` is not set but `RUSTFLAGS` is, and so on down the list. All of
these options work in a very similar manner that they are applied for not just the current crate but also all dependencies for said crate
including transitive dependencies. In the case of the `target.*.rustflags` or `host.*.rustflags` these only apply if the target triple
specified in the manifest file matches the target triple used as the host or target.

Another supported way of setting rustflags in cargo is the `profile.rustflags` manifest key that can be set in a `Cargo.toml`. This
works in a slightly different manner than the ways mentioned previously in that it is appended to the set of rustflags calculated from
the environment variables and cargo config settings. This also has support for crate specific rustflags on a per profile basis. This
currently depends on the [profile-rustflags](https://doc.rust-lang.org/cargo/reference/unstable.html#profile-rustflags-option) unstable
option. This restricts setting rustflags for crates to the profile requested. The `--rustflags` option works for the current crate
regardless of the currently requested profile.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC proposes adding a new cargo flag, `--rustflags <RUSTFLAGS>`, which would instruct cargo to pass the specified rustflags when
invoking rustc for the current crate being compiled. This allows setting a Rust compiler flag for local crates only and not forcing this
upon dependencies including transitive dependencies.

## An example: code coverage

The Rust compiler supports instrumenting Rust built libraries to measure code coverage for a given crate through tests.
In order to instruct rustc to instrument a given crate, a user would need to pass the Rust compiler flag `-C instrument-coverage`
to the invocation of rustc when building said crate. This can be done via the `RUSTFLAGS` environment variable but this would have
the side effect of enabling this flag for every crate in the dependency graph including upstream dependencies, transitive dependencies,
as well as the standard libraries. There are a couple of other options for setting Rust compiler flags but most of them have the
same issue as using the `RUSTFLAGS` environment variable.

Another way of setting a rustc flag for a specific crate is through the `cargo rustc` subcommand. A Rust compiler flag can be passed
to the invocation of rustc by cargo for the current crate being built by setting it as an argument directly to rustc.

For example:

```
cargo rustc -- -C instrument-coverage
```

This example will pass the flag `-C instrument-coverage` directly to rustc but only for the current crate. Running another cargo command
after this will cause a new build of the crate without the flag. For example running `cargo test` will cause the crate to be re-compiled
without the rustc flag `-C instrument-coverage` specified. This would cause tests to be run without first instrumenting any of the
libraries thus losing out on collecting any code coverage.

## Other examples: debuginfo, saving temp files

Other examples of this include having the ability to set the debuginfo level, (`-C debuginfo=<val>`), for only a single crate to save on
binary size. One scenario where this is helpful is debugging tests that are not having the expected behavior. This feature will allow a
Rust developer to set the debuginfo level for the current crate and have full debugging symbols but still allow all other dependencies to
be fully optimized. This will save on both compilation time and binary size.

As with the previous example, being able to specify only saving the temporary files, (`-C save-temps`), for the current crate as opposed
to all crates in the dependency graph. This will save on disk space by not having all of the unnecessary temp files being saved on each
invocation of the Rust compiler.

There are some unstable flags that would also benefit from this feature such as the `-Z unpretty=<val>` option. This could be used to
expand the input source to allow for easier debugging of macros and proc macros. This will allow a single crate to have its input
expanded and lessen the amount of source expanded which would allow for Rust developer to better understand how a macro is affecting
their source code.

## --rustflags

The `--rustflags` option will allow a Rust developer to pass any set of rustc flags to the root crate being built. This will allow a
simple command such as `cargo test` to have the option of setting the `-C instrument-coverage` flag for a single crate and run the unit
tests ensuring coverage data is collected for this crate and this crate only. For example, let's take crate `foo`:

Cargo.toml:
```toml
[package]
name = "foo"
version = "0.1.0"
```

cargo CLI (Output lines have been removed for simplicity):
```
cargo test --rustflags -C instrument-coverage ;

Compiling foo v0.1.0 (...)
     Running `rustc --crate-name foo --edition=2021 src/lib.rs --crate-type lib ... -C instrument-coverage`
     Running `rustc --crate-name foo --edition=2021 src/lib.rs --test ... -C instrument-coverage`
    Finished test [unoptimized + debuginfo] target(s) in 1.35s
     Running `target/debug/deps/foo-669448d9b4043564`

running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Running this command will build the crate `foo` with the flag `-C instrument-coverage` passed to the invocation of rustc
for the crate `foo` only. Upstream dependencies would not be instrumented as well as the standard libraries, which is not the case
now, saving on compilation time.

## --rustflags for a workspace

Cargo supports two types of [workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html), a workspace with a root package and
a workspace with a virtual manifest, meaning a "primary" package does not exist.

For workspaces that contain a root package, any `--rustflags <RUSTFLAGS>` options set will be passed to the invocation of the Rust
compiler for the root crate only. If the `workspace.default-members` manifest key has been set, then only the members listed as a
default member will have the `--rustflags` values passed to the invocation of rustc.

Cargo.toml:
```toml
[package]
name = "foo"

[workspace]
members = ["bar", "baz"]
default-members = ["bar"]
```

cargo CLI (Output lines have been removed for simplicity):
```
cargo build --rustflags -C instrument-coverage -C strip=symbols ;

Compiling bar v0.1.0 (.../foo/bar)
    Running `rustc --crate-name bar ... -C instrument-coverage -C strip=symbols`
Finished dev [unoptimized + debuginfo] target(s) in 0.98s
```

For workspaces that do not contain a root package, any `--rustflags <RUSTFLAGS>` options set will be passed to the invocation
of the Rust compiler for all members unless `workspace.default-members` manifest key has been set. In that case, only the default
members being compiled will have the rustflag options specified passed through to rustc. For example:

Cargo.toml:
```toml
[workspace]
members = ["foo", "bar"]
```

cargo CLI (Output lines have been removed for simplicity):
```
cargo build --rustflags -C instrument-coverage ;

Compiling foo v0.1.0 (.../foo/foo)
Compiling bar v0.1.0 (.../foo/bar)
    Running `rustc --crate-name foo ... -C instrument-coverage`
    Running `rustc --crate-name bar ... -C instrument-coverage`
Finished dev [unoptimized + debuginfo] target(s) in 0.98s
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As mentioned above a new Cargo option, `--rustflags <RUSTFLAGS>`, would be added to several of the existing cargo subcommands.
Those subcommands would be, `bench, build, check, doc, fix, install, package, run` and `test`. The `--rustflags <RUSTFLAGS>` option
will require the use of a [value terminator](https://docs.rs/clap/3.2.18/clap/builder/struct.Arg.html#method.value_terminator), `;`.
Cargo under the covers uses the `clap` crate to parse the command line invocation and set the relevant options passed to it. Since all
rust compiler flags start with a `-`, the value terminator makes it possible for the `clap` parser to allow values that begin with a `-`
and still allow other cargo options to follow the the `;`. Without using the `;` value terminator at the end of the `<RUSTFLAGS>` list,
any cargo flags that come after this would be interpreted as more rustflags, leading to potential errors from the Rust compiler.

This feature also support quoting a rustflag in the same manner the Rust compiler does. This means that any `<RUSTFLAGS>` which would
need to wrap its value with quotes can be done so via the `--rustflags` option. This would allow supporting flags that may contain spaces
or commas.

For each rustc flag specified by a Rust developer, Cargo will pass the flag through to the invocation of rustc for the current crate.
This includes all invocations of rustc for a given crate including all targets, such as, lib, bin, examples and the test targets
being built. For example, a given crate `foo` that contains a lib, bin, examples and test target:

Cargo.toml:
```toml
[package]
name = "foo"
version = "0.1.0
```

cargo CLI (Output lines have been removed for simplicity):
```
cargo test --rustflags -C instrument-coverage ;

Compiling foo v0.1.0 (foo)
    Running `rustc --crate-name foo --edition=2021 src/lib.rs ... --crate-type lib -C instrument-coverage`
    Running `rustc --crate-name foo --edition=2021 src/lib.rs --test ... -C instrument-coverage`
    Running `rustc --crate-name bin1 --edition=2021 src/bin/bin1.rs ... -C instrument-coverage`
    Running `rustc --crate-name example --edition=2021 examples/example.rs... --crate-type bin -C instrument-coverage`
    Running `rustc --crate-name foo --edition=2021 src/main.rs --test ... -C instrument-coverage`
Finished test [unoptimized + debuginfo] target(s) in 1.51s
    Running `.../debug/deps/foo-669448d9b4043564`

running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    Running `.../target/debug/deps/bin1-88f2f72473b4679f`

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

    Running `.../target/debug/deps/foo-53f9cd70087d575a`

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

Doc-tests foo
    Running `rustdoc --edition=2021 --crate-type lib --crate-name foo --test ...`

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

In the example above, the only invocation which does not include the user specified rustc flags is the invocation of
`rustdoc`. Since `rustdoc` flags are treated different than normal `rustflags`, the flags specifed through `--rustflags <RUSTFLAGS>`
will not be passed to the invocation of `rustdoc`.

## Build scripts

The new `--rustflags <RUSTFLAGS>` feature will not be passed to the invocation of rustc for build scripts that are being compiled and run
on the host. This is currently out of scope for this RFC since rustc flags are treated differently for build scripts depending on cargo
configuration settings as well as the target specified.

## Integration with existing RUSTFLAGS

As mentioned above, there exists numerous ways of setting rustflags in cargo. The values passed to the `--rustflags` option would
be appended to the set of rustflags calculated from the options listed above only for the current crate or the set of crates in the
workspace. All upstream and transitive dependencies will still use the rustflags calculated from the environment variables and cargo
config only.

The `profile.rustflags` manifest key is appended to the set of rustflags calculated from the environment variables and cargo config settings, this behavior will not change with the addition of the `--rustflags <RUSTFLAGS>` flag.

# Drawbacks
[drawbacks]: #drawbacks

There currently exists multiple ways of setting Rust compiler flags when building a Rust project with Cargo. As we mentioned
earlier, there about 7 different ways that already exist today and this RFC is proposing to add yet another option. This could
lead to confusion about the best way to set Rust compiler flags in the community.

Another drawback of supporting this new option is that it would make it easier for a Rust developer to enable rustflags that could
impact ABI, and would be unsound to only compile a single crate with. A couple of examples of these kinds of rustflags are the
`-C soft-float` and `-C target-feature` rustc flags. This potential issue is not limited to the feature being proposed in this RFC,
it is currently possible to cause this by using the `cargo rustc` subcommand and passing one of the options listed above. Another way
of causing this issue is by using the `profile.<PROFILE>.package.<PACKAGE>.rustflags` manifest key to enable one of these rustflags
for only a specific package.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale

This design provides a simple mechanism to specify the set of Rust compiler flags
a given crate should be built with. This design also has the benefit of not forcing
all crates in the dependency graph, including upstream dependencies and transitive
dependencies, to be built in the same manner. As with the given example above, setting
the rustc flag `-C instrument-coverage` forces the compiler to do an extra amount
of work to instrument all of the libraries in a given crate. If this flag was passed
for all transitive dependencies, that would only add to the amount of work that needs
to be done by the compiler. With this new feature, the rustc flags set via the `--rustflags`
cargo option would only affect the root crate or the members of the current workspace.

## Alternatives

### Alternative 1: existing RUSTFLAGS manifest keys

Accepting this alternative would mean changes would not be made to either cargo or rustc. The existing behavior
does allow for a way to set custom Rust compiler flags that would be passed to the compiler at the invocation of rustc
by cargo. This would still leave Cargo in a place where a Rust developer would not be able to enable/disable rustc flags
on a crate by crate basis. As with the example of running code coverage for a Rust project, this would mean all libraries
would be instrumented leading to larger code coverage files and more compilation time spent by the compiler.

### Alternative 2: new build.<crate_name>.rustflags manifest key

This alternative proposes another way to implement such a feature that would allow a Rust developer to specify through cargo
configuration files exactly which crate enables which Rust compiler flag. There already exists a `rustflags` key under the
`[build]` section of the cargo configuraiton file and this would add to that existing pattern of selectively enabling
rustc flags.

This approach has the benefit of extensibility. This meaning it can easily be expanded upon to support crate specific rustflags
for the root crate being built as well dependencies in the dependency graph by updating the crate name in the manifest section.
For instance, supporting `build.foo.rustflags` would also make it simpler to support `build.<dependency>.rustflags` as well.

#### Note

The `profile-rustflags` cargo feature makes it possible to support this syntax since it is used in a very similar way. The
`profile-rustflags` feature has support for setting crate specific rustflags for a profile. This syntax can simply be
re-used to support crate specific rustflags as mentioned above. This feature is currently unstable but has been widely available
for some time. Combining this with the recently stabilized `--config` cli option makes it possible to override config values
on demand on the command line. I believe this option may include the fewest amount of changes to Cargo and would leverage existing
syntax that Rust developers may already be familiar with.

An issue with this approach is that changes to the config file is more cumbersome than simply adding commands at
the CLI. This approach I believe would also make the feature more complicated in cases where a Rust developer wants to
enable a specific flag for a set of crates such as the crates within a given workspace. This approach would mean each crate
would need a separate section in the cargo configuration file to enable said rustc flag.

### Alternative 3: override existing RUSTFLAGS value with --rustflags

The feature proposed by this RFC suggests combining the RUSTFLAGS calculated by cargo from environment variables and manifest files
with the values specified by the `--rustflags` cargo option. An alternative would be to override any of Cargos default logic for
collecting rustflags when the `--rustflags` option is set for a given crate. This allows for the possibility for contradicting
`RUSTFLAGS` to be set by the user via the `--rustflags` option and have it only apply for that crate but leave `RUSTFLAGS` unotuched
for dependencies of said crate.

# Prior art
[prior-art]: #prior-art

As mentioned above there are multiple ways of setting rustflags that exist today but none of the options are suitable for
selectivly enabling flags on a crate by crate basis. The `RUSTFLAGS` environment variable as well as the `<section>.rustflags`
manifest key force a user to opt in to enabling the given rustc flag for all crates in the dependency graph.

`cargo rustc` allows a user to enable a Rust compiler flag specifically for the root crate being built but any subsequent runs
of a cargo subcommand forces a re-compilation and wipes out the enabled flags. This also doesn't work when trying to set rustflags
for a simple `cargo run` and especially when running tests via `cargo test`.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. Should the cargo feature `--rustflags <RUSTFLAGS>` be dependent on the existing unstable cargo feature `-Z target-applies-to-host`
to determine whether or not the rustc flags specified by the user on the cargo CLI should be passed to the invocation of rustc for
build scripts defined in a given crate?

# Future possibilities
[future-possibilities]: #future-possibilities

## Crate specific --rustflags

A natural extension to this feature would be to add support for specifying specific crates a Rust compiler flag should be enabled for.
For example, if in a given workspace there exists 2 default members, `foo` and `bar`, having the ability to set rustc flags only for the
crate `foo` and not for the crate `bar`. An example of said feature would be:

cargo CLI (Output lines have been removed for simplicity):
```
cargo build --rustflags foo:-C strip=debuginfo ;
```

This would result in the `foo` crate stripping away all debuginfo from the generated binary and/or `PDB`. This would not have the same
effect on the `bar` crate or any other upstream dependencies.

## --rustflags support for dependencies

Allowing a Rust developer to manually specify which rustflags are passed to upstream dependencies seems like a natural extension of this
feature. As with the above mentioned future possibilities, `--rustflags <crate_name>:<RUSTFLAGS>`, would be sufficient for adding support
for specifying rustc flags for upstream dependency. If a crate is selected which does not exist, or which has not been pulled in as a
dependency, then an warning would be raised notifying the user that the specified rustc flag was unused. A simple use case for this would
be allowing the instrumentation of targeted upstream dependencies or local dependencies through the use of the `-C instrument-coverage`
rustc feature.

## --rustflags support for build scripts

The feature proposed by this RFC does not extend any support to passing Rust compiler flags specified by the `--rustflags` feature to
invocations of rustc when compiling build scripts. There is support today for setting rustc flags for build scripts depending on certain
configuration settings such as, whether the host and target triples match, and/or if the unstable `-Z target-applies-to-host` flag has
been enabled and the `[host]`/`[build]` sections have the `rustflags` manifest key set.

Allowing a mechanism for setting rustc flags for build scripts via the `--rustflags` cargo feature would extend the flexibility of
setting compiler flags from the cargo CLI.
