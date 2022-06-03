- Feature Name: `cargo_test_coverage`
- Start Date: 2022-06-01
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC aims to improve the process of collecting code coverage data for
Rust libraries. By including Cargo in the process of instrumenting Rust
libraries and running the unit tests, the sequence of steps to get coverage
results will be simplified. This RFC also proposes adding support for cargo
to selectivly choose which crates get instrumented for gathering coverage results.

# Motivation
[motivation]: #motivation

Why are we doing this? What use cases does it support? What is the expected outcome?

The motivation behind this feature is to allow for a simple way for a Rust developer
to run and obtain code coverage results for a specific set of crates to ensure confidence
in code quality and correctness. Currently, in order to get instrumentation based code coverage,
a Rust developer would have to either update the `RUSTFLAGS` environment variable or cargo
manifest keys with `-C instrument-coverage`. This would automatically enable instrumentation
of all Rust crates within the dependency graph, not just the top level crate. Instrumenting
all crates including transitive dependencies does not help the developer ensure test coverage
for the crates they actually want to test. This also adds unnecessary work for both the Rust
compiler and codegen backend, `LLVM` in this case, to instrument all libraries as opposed to
the subset a Rust developer actually cares about.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This section examines the features proposed by this RFC:

## CLI option

A new flag for the `cargo test` command would be added. The new flag `--coverage` would instruct Cargo to
add the Rust compiler flag `-C instrument-coverage`, for the given crate only. This would mean that only the top-level
crate would be instrumented and code coverage results would only run against this crate. As an example, lets take the
following crate `foo`:

```text
/Cargo.toml
  +-- src
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
Next, the tests for the crate `foo` will be run and produce code coverage results for this crate only and ignore
all code coverage for any function defined outside of this crate.

The `cargo test` subcommand also supports the `--package` and `--workspace` flags. This instructs cargo to run the
tests for those specific packages. When combined with the new `--coverage` flag, a Rust developer will be able to
selectively choose which Rust crates will be instrumented and have tests run. Since potentially unit tests from
multiple crates will be run, the code coverage results will include coverage results for a crate that is being
covered by a test from another crate.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As mentioned earlier this RFC proposes adding a new flag to the `cargo test` command. This flag, `--coverage`
would be responsible for setting the `-C instrument-coverage` flag that Cargo would pass on to the rustc invocation of the
top-level crate. In the previous example, `foo` would be the top level crate and `regex` would be upstream an dependency.

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

Once the tests have finished running, cargo would leverage LLVM tooling to analyze profraw files and generate
coverage reports for a Rust developer. These tools would be `llvm-profdata` and `llvm-cov` and can be used by
adding the `llvm-tools-preview` component to the current toolchain, `rustup component add llvm-tools-preview`.
This would be a requirement to use this new feature since these are the tools necessary to analyze coverage results
and generate a code coverage report.

These updates to Cargo would be sufficient enough to ensure that a Rust developer would have control over what crates are
instrumented and code coverage results are generated. This would also allow the Rust developer to no longer have to
set environment variables manually to ensure crates are instrumented for gathering coverage data.

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
Rust flag only for the top level crate and set the `LLVM_PROFILE_FILE` environment
variable to ensure each test run produces a unique `profraw` file. Once the tests have
finished running, cargo would leverage LLVM tooling to analyze profraw files and generate
coverage reports for a Rust developer.

This design does not break any existing usage of Rust or Cargo. This new feature would
be strictly opt-in. A Rust developer would still be able to manually set the
`-C instrument-coverage` Rust flag and instrument all binaries within the dependency
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

### Alternative 2: use a new manifest key instead of a cli option

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

## Specifying multiple crates to instrument for Code Coverage

This would allow for Rust developers to specify each of the crates they want to instrument in
advance and Cargo would be able to pass on the `-C instrument-coverage` flag for only the
crates specified. This would allow a more targeted approach of getting code coverage results
and not for developers to instrument the entire dependency graph. This could either be in the form
of a manifest key in the toml file which would take a `,` separated list of crates to include in
the code coverage analysis or by specifying each crate at the command line using `--coverage:crate_name`.
