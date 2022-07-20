- Feature Name: `cargo_test_coverage`
- Start Date: 2022-06-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC aims to improve the process of collecting code coverage data for
Rust libraries. By including Cargo in the process of instrumenting Rust
libraries and running the unit tests, the sequence of steps to get coverage
results will be simplified. This RFC also proposes adding support for Cargo
to selectivly choose which crates get instrumented for gathering coverage results.

# Motivation
[motivation]: #motivation

Why are we doing this? What use cases does it support? What is the expected outcome?

The motivation behind this feature is to allow for a simple way for a Rust developer
to run and obtain code coverage results for a specific set of crates to ensure confidence
in code quality and correctness. Currently, in order to get instrumentation based code coverage,
a Rust developer would have to either update the `RUSTFLAGS` environment variable or Cargo
manifest keys with `-C instrument-coverage`. This would automatically enable instrumentation
of all Rust crates within the dependency graph, not just the current crate. Instrumenting
all crates including transitive dependencies does not help the developer ensure test coverage
for the crates they actually want to test. This also adds unnecessary work for both the Rust
compiler and codegen backend, `LLVM` in this case, to instrument all libraries as opposed to
the subset a Rust developer actually cares about. This support is currently limited to the
`LLVM` codegen backend and will have no effect when another codegen backend is used.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This section examines the features proposed by this RFC:

## CLI option

Three new flags will be added for the `cargo test` command, `--coverage`, `--coverage-format` and `--coverage-output-directory`
The `--coverage` flag would instruct Cargo to add the rustc flag `-C instrument-coverage`, for the current
crate only. Cargo also supports two types of workspaces, a workspace with a root package and one without called a
virtual manifest. In the case where the `cargo test --coverage` command is run from a Cargo workspace which has a root package,
the `-C instrument-coverage` rustc flag would only be enabled for the root package. In the case where the `cargo test --coverage`
command is run from a Cargo workspace without a root package, the `-C instrument-coverage` rustc flag would be enabled for
all default members of the workspace. This would mean that only the crates selected would be instrumented and code
coverage results would only be collected for those crates and not all crates in the dependency graph.

As an example, let's take the following crate `foo`:

```text
/Cargo.toml
/src
    +-- lib.rs
```

Where crate `foo` has a dependency on the `regex`:
```toml
[dependencies]
regex = "*"
```

And `lib.rs` contains:

```rust
use regex::*;

pub fn match_regex(re: Regex, text: &str) -> bool {
    let Some(caps) = re.captures(text) else {
        return false
    };

    let matches = caps.iter().filter_map(|s| s).collect::<Vec<Match>>();
    matches.len() > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_match() {
        let result = match_regex(Regex::new(".*").unwrap(), "Hello World");
        assert_eq!(result, true);
    }

    #[test]
    fn find_no_match() {
        let result = match_regex(Regex::new("a+").unwrap(), "Hello World");
        assert_eq!(result, false);
    }
}
```

Now running `cargo test --coverage` would build the `foo` crate and instrument all libraries created by this crate.
Each test executable run will generate a unique `*.profraw` file. At the end of all test execution, Cargo will be
responsible for merging the generated `profraw` files, and by default would generate code coverage results in HTML
format for simple viewing within a browser. The code coverage results produced would be only for the crate `foo`
and ignore all code coverage for any function defined outside of this crate.

LLVM supports exporting code coverage results in a number of formats. `--coverage-format` would be responsible for selecting
the code coverage results format and `--coverage-output-directory` would allow a Rust developer to select the
output directory for the code coverage report. The default for `--coverage-format` is `html` and the default for
`--coverage-output-directory` would be `target/coverage/`

The `cargo test` subcommand also supports the `--package` and `--workspace` flags. This instructs Cargo to run the
tests for those specific packages. When combined with the new `--coverage` flag, a Rust developer will be able to
selectively choose which Rust crates will be instrumented and have tests run. Since potentially unit tests from
multiple crates will be run, the code coverage results will include coverage results for a crate that is being
covered by a test from another crate.

The `--coverage` flag will also support a comma separated list of crates so that a Rust developer can control
the exact set of crates which will be instrumented during a build and test run.

For example, let's take the following workspace with default members:

```text
/Cargo.toml
/member1
  +-- src
    +-- lib.rs
/member2
  +-- src
    +-- lib.rs
/member3
  +-- src
    +-- lib.rs
```

Where crate `foo` has a virtual manifest:
```toml
[workspace]
members = ["path/to/member1", "path/to/member2", "path/to/member3"]
default-members = ["path/to/member2"]
```

Running the following cargo invocation would only instrument the workspace member
`member2` and run the tests for `member2`:

```
cargo test --coverage
```

To instrument a different workspace member, the following command is also supported:

```
cargo test --coverage=member1,member3
```

This cargo invocation will instruct Cargo to run the tests for the default workspace member `member2`, but
will only instrument the crates `member1` and `member3` to collect coverage results for. This flexibility
allows a Rust developer to see just how much a given crate is covered by tests from another crate.

With this feature, existing custom Cargo commands can leverage Cargo to do all of the heavy work to instrument specific
crates and generate coverage results in a multitude of formats. This will allow in more flexibilty in generating
for custom commands that generate code coverage for crates today.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As mentioned earlier, this RFC proposes adding multiple flags to the `cargo test` command. The flag `--coverage`
would be responsible for setting the `-C instrument-coverage` flag that Cargo would pass on to the rustc invocation of the
current crate. In the previous example, `foo` would be the crate being instrumented and `regex` would be an upstream dependency.

Using the `--coverage` flag, Cargo would only set the `-C instrument-coverage` flag for the crate `foo`. If the `RUSTFLAGS`
environment variable had already been set to include the `-C instrument-coverage` flag, then Cargo would still pass that
flag to all crates within the dependency graph, including the `regex` crate and any transitive dependencies.

This should not break any existing workflows and is strictly an opt-in feature.

To use this new feature do the following:

`cargo test --coverage`

This flag would also be responsible for setting the `LLVM_PROFILE_FILE` environment variable which is read by LLVM
to generate a `.profraw` file for each test executable that is run. Once again if the environment variable is already set,
then Cargo would not make any changes and would leave the value as is to use the user-defined file name. If the environment
variable is not set, Cargo would set it to ensure a unique naming scheme is used for each `.profraw` file that would be
generated.

Once the tests have finished running, Cargo would leverage LLVM tooling to analyze profraw files and generate
coverage reports for a Rust developer. These tools would be `llvm-profdata` and `llvm-cov` and can be used by
adding the `llvm-tools-preview` component to the current toolchain, `rustup component add llvm-tools-preview`.
This would be a requirement to use this new feature since these are the tools necessary to analyze coverage results
and generate a code coverage report. If the component is not installed for the current toolchain, an error will occur
with a message stating the the `llvm-tools-preview` component is required to generate a code coverage report.

For example, a Rust developer can invoke the following Cargo command to generate an HTML coverage report
for the current crate or `default-members` in a workspace:

```
cargo test --coverage
```

This will generate a code coverage report in HTML format in the `target/coverage/` directory. If a workspace
does not have `default-members`, then all members would be instrumented.

Run the following cargo CLI to choose a specific set of crates to instrument and override the defaults:

```
cargo test --coverage:crate1,crate3,foo
```

To override the default output format and directory, the `--coverage-format` and `--coverage-output-directory`
can be passed to the cargo CLI:

```
cargo test --coverage --coverage-format=lcov --coverage-output-directory=src/coverage
```

This will generate a code coverage report in lcov format in the `src/coverage/` directory.

The supported options for the `--coverage-format` option are:

1. `html`
2. `lcov`
3. `json`

These updates to Cargo would be sufficient enough to ensure that a Rust developer would have control over what crates are
instrumented, the format in which code coverage results are generated and where they are stored. This would also allow
the Rust developer to no longer have to set environment variables manually to ensure crates are instrumented
for gathering coverage data and can generate code coverage results with a single command.

# Drawbacks
[drawbacks]: #drawbacks

A drawback of this feature would be that Cargo would need to enable the `LLVM_PROFILE_FILE`
environment variable in order to ensure unique profile data is generated for each test
executable. I am not aware of any other Cargo features that set environment variables today
so this could potentially create new issues in Cargo.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale

This design provides a simple mechanism to integrate collecting code coverage
results for a given crate. Allowing Cargo to be part of the coverage process
would reduce the need for setting environment variables manually. Simply running
`cargo test --coverage` would automatically run a build setting the `-C instrument-coverage`
rustc flag only for the crates selected and set the `LLVM_PROFILE_FILE` environment
variable to ensure each test run produces a unique `profraw` file. Once the tests have
finished running, Cargo would leverage LLVM tooling to analyze profraw files and generate
coverage reports for a Rust developer.

This design does not break any existing usage of Rust or Cargo. This new feature would
be strictly opt-in. A Rust developer would still be able to manually set the
`-C instrument-coverage` rustc flag and instrument all binaries within the dependency
graph. Since this is a Cargo specific feature, the Rust compiler will not need any updates.

## Alternatives

### Alternative 1: leave the existing feature

Supporting this alternative would mean that no changes would be necessary to either Cargo
or Rust. Getting instrumentation-based code coverge is still supported and would continue
to work as it does today.

The drawback for this option is that it would require setting the flag for all crates in
the dependency graph, including upstream dependencies. This would also instrument all
binaries and report on coverage for functions and branches that are not defined by the
current crate with the potential of skewing coverage results.

### Alternative 2: use a new manifest key instead of a CLI option

Supporting this alternative would mean that changes would need to be made to the existing
manifest format to possibly include a new section and/or key. A new `coverage` key could
be added to the target section, `coverage = true`. This still has the added benefit of not
requiring any changes to the Rust compiler itself and the feature could be scoped to Cargo only.

The drawback for this option is that it could potentially add clutter to the `Cargo.toml`
files. Adding this new section to every crate that a Rust developer wants to have instrumented
would only add to all the data that is already contained within a toml file.

### Alternative 3: use a RUSTC_WRAPPER program to selectively choose which crates to profile

Supporting this alternative would mean that there wouldn't need to be any changes to Cargo
at all.

This would require creating a RUSTC_WRAPPER program specifically for selecting which crates to profile.
This means more boiler plate code for each Rust developer that simply wants to profile their own crate.
I believe the feature this RFC proposes would both be a cleaner solution long term and more in line
with the Cargo workflow of potentially reading these kinds of behaviors from the `Cargo.toml` manifest
file.

### Alternative 4: use existing custom subcommands to run code coverage analysis

Supporting this alternative would mean that there wouldn't need to be any changes to Cargo
at all.

There are multiple custom subcommands that exist today to achieve instrumenting Rust libraries
analyzing code coverage metrics. A couple of examples would be cargo-llvm-cov and cargo-tarpaulin.
Both of these custom subcommands are available via crates.io and support running tests with code
coverage enabled and creating a coverage report to be viewed by a Rust developer.

Cargo-llvm-cov currently leverages the `cargo-config` subcommand which is still unstable. To do so,
cargo-llvm-cov sets the `RUSTC_BOOTSTRAP` environment variable to allow its usage from a stable
toolchain. This is not a recommended approach especially for Rust developers that want to use
such tools in production code.

Tarpaulin has a great set of features for collecting and analyzing coverage metrics but it only
supports x86_64 processors running Linux which limits how this can be used by Rust developers
working on other platforms such as Windows.

# Prior art
[prior-art]: #prior-art

## VSInstr

Visual Studio ships with a tool `vsinstr.exe` which has support for instrumenting binaries after
they have already been built. Since LLVMs instrumentation-based code coverage hooks into each object
file it generates this scenario is a bit different than the feature this RFC proposes. `vsinstr` does
allow for excluding namespaces of functions to skip over so that everything within a binary does not
get instrumented.

 - [vsinstr](https://docs.microsoft.com/en-us/previous-versions/visualstudio/visual-studio-2017/profiling/vsinstr?view=vs-2017)

## gcov based coverage

Rust also has support for another type of code coverage, a GCC-compatible, gcov-based coverage implementation.
This can be enabled through the `-Z profile` flag. This uses debuginfo to derive code coverage. This is different
than the source-based code coverage which allows for a more precise instrumentation to be done.

 - [Source-Based Code Coverage](https://blog.rust-lang.org/inside-rust/2020/11/12/source-based-code-coverage.html)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Are there any drawbacks from having Cargo set the `LLVM_PROFILE_FILE` environment variable that LLVM uses to name each
of the generated `profraw` files? This is used to ensure each test run generates unique profiling data as opposed to overwriting
the previous run.

# Future possibilities
[future-possibilities]: #future-possibilities

## A single command to generate coverage results and open in a browser

Running `cargo test --coverage --open` would greatly simplify the experience of viewing coverage results.
With a simple command, a Rust developer can run all of their tests with source-based instrumentation
enabled, and view the coverage results in a browser. This would need a new flag `--open` flag to be
added to the `cargo test` command and would only be valid if the `--coverage` flag was enabled. This
would produce a very similar experience to how `cargo doc --open` works today. `cargo doc` compiles all
of the documentation for a crate and the `--open` flag automatically opens a browser showing all of the
generated documentation. This would be a great feature to have when generating code coverage.
