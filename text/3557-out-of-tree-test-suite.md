- Feature Name: Out-of-tree test suite
- Start Date: 2024-01-10
- RFC PR: [rust-lang/rfcs#3557](https://github.com/rust-lang/rfcs/pull/3557)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

We propose the creation of an external stand-alone test suite and associated tools for verifying the Rust toolchain that also fits into existing Rust development workflows.

# Motivation
[motivation]: #motivation

## Testing the artifacts to be distributed
[testing-the-artifacts-to-be-distributed]: #testing-the-artifacts-to-be-distributed

Industrial use of language toolchains requires assurance that the toolchain that is distributed has been rigorously tested and is guaranteed - even warranted - to work correctly. To achieve this level of assurance, the toolchain is built and packaged for distribution, installed on a completely clean machine, and then tested. This separation of concerns, which mimics the way a user will interact with the toolchain, ensures that the artifacts distributed are the artifacts tested.

Currently, the Rust infrastructure does not support this behavior. Instead, testing validates intermediate states of the Rust toolchain as they exist in the build tree. An intermediate state may contain leftover artifacts from a previous build that could influence the validation. The Rust infrastructure may be directed to produce an malformed or incomplete tool (ex: `rustc` with a missing `libstd`), and yet hold the belief that the tool has successfully passed validation.

Similarly, the Rust infrastructure builds several versions of `rustc`'s native libraries, one for each stage. For a cross target, the Rust infrastructure builds two two distinct versions of a library - one for `rustc` running on the host, and one to link against for the target executable. Testing may validate the wrong version of a library or validate a corrupted / incomplete intermediate state of the library.

Out-of-tree testing solves these problems by testing the release artifacts of the Rust toolchain.

## Testing on multiple hosts
[testing-on-multiple-hosts]: #testing-on-multiple-hosts

Testing a Rust toolchain across a family of host targets (ex: Ubuntu and Red Hat flavors of Linux) requires dedicated build-and-test cycles for each member of the family. It is not possible to build a toolchain on one host and test it on another host of the same family because the Rust infrastructure detects out-of-date, relocated, or missing artifacts, and always performs at least some rebuilding.

Out-of-tree testing solves this problem by allowing build and packaging to be performed on one host platform (ex: Ubuntu) while installation and testing are performed on another, compatible platform (ex: Red Hat).

## Certification
[certification]: #certification

Out-of-tree testing will be essential in enabling tool qualification for certifiable software development in Rust.

Tool qualification involves the demonstration of proper tool operation on a host and a target, where both environments are strictly defined. For DO-178C (avionics safety standard) and ISO-26262 (automotive safety standard), this demonstration involves rigorous and reproducible testing. In particular, evidence must be provided that the version of the tool tested is the version of the tool employed in the certification context.

Any Rust qualification kit would require out-of-tree testing, since out-of-tree testing enables strong assurance that the version of the toolchain delivered is the version of the toolchain tested.

## Authoritative test suite
[authoritative-test-suite]: #authoritative-test-suite

Considering the ongoing work on the Rust Language Specification, an authoritative test suite will be required to gauge the conformance of a tool against the Rust language. This test suite may be derived from existing Rust tests or written from scratch.

Either way, the authoritative test suite should be executable out-of-tree.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

To address the problems we identified, we propose the use of an out-of-tree test suite (OOTTS). The OOTTS would handle all testing of the Rust toolchain, and replace the existing testing-related Rust infrastructure.

The OOTTS would reside in its own dedicated `rust-lang` repository, and would be considered a distinct Rust product, with its own CI and release artifacts. The OOTTS would consist of infrastructure, test driver, sub-suites, and tests. The infrastructure would handle the building and packaging of the OOTTS. The test driver would be configured via a file, and run requested sub-suites against "testables" (either tools or libraries). Existing sub-suites and tests would be moved out from the existing `rust-lang/rust` repository into the OOTTS repository, and possibly be subjected to light triage.

Users and automation would install the OOTTS and invoke its test driver to verify tools and libraries of their choosing.

Rust developers would use the OOTTS in-tree as a git submodule of `rust-lang/rust`. The existing Rust infrastructure would delegate the building, testing, and packaging of the OOTTS to the OOTTS infrastructure and test driver.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Repository
[repository]: #repository

The OOTTS repository would be a sub-repository within the larger `rust-lang` repository. It would house the infrastructure entry point at the top level, the remainder of the infrastructure and the test driver in `src`, and the sub-suites plus tests in `tests`. Other content (copyright, licenses, git-related files, etc) is outside the scope of this RFC.

The structure of the `tests` directory is left-open ended, however it could be organized around the testables, as follows:

    tests
    |
    +- compiler
    |
    +- compiletest
    |
    +- library
    |  |
    |  +- <first library>
    |  | . . .
    |  +- <last library>
    |
    +- tool
       |
       +- <first tool>
       | . . .
       +- <last tool>

Addition and removal of sub-suites and tests, as well as modifications to the test driver itself would fall under the general Rust development process, and employ the same established procedures.

## Infrastructure
[infrastructure]: #infrastructure

The infrastructure of the OOTTS could be modeled after the `rust-lang/rust` infrastructure, with an entry point similar to the x script, and helper tools such as `bootstrap`, `compiletest`, etc.

The process of building the test driver is left intentionally open-ended, as it may be possible to reuse existing machinery from `bootstrap`, or employ a completely different approach. Regardless of the implementation, building should produce a test driver executable for the appropriate host.

The process of producing the OOTTS distributable is also left open-ended, for the same reason as above. The distributable however should contain the test driver executable, dependencies, sub-suites, and tests.

## Configuration
[configuration]: #configuration

The test driver would be configured using a `config.toml` file that follows the schema outlined below. The purpose of the `config.toml` file is to associate user-defined behavior with key events which take place during a testing run - set up, tear down, compilation, and execution. This should allow for the startup of a virtual machine or connection to a physical board prior to a testing run, increase in the file descriptor limit for a particular test, etc.

The `config.toml` file would have the following optional sections and attributes:

    [general]
    set_up = "path_to_executable"
    tear_down = "path_to_executable"

    [sub_suite]
    set_up = "path_to_executable"
    tear_down = "path_to_executable"

    [test]
    pre_compile = "path_to_executable"
    post_compile = "path_to_executable"
    pre_execution = "path_to_executable"
    post_execution = "path_to_executable"

The `[general]` section would capture attributes that are in effect during the whole testing run. The `set_up` attribute would be a path to an executable (bash, python, Rust, etc) that would be invoked prior to the commencement of the testing run. Similarly, the `tear_down` executable would be invoked after the completion of the testing run, even if the testing run failed.

The `[sub_suite]` section (ex: `[tests/ui/unsafe]`) would capture attributes that are relevant only for that sub-suite. The `set_up` executable would be invoked prior to running any test of that sub-suite. The `tear_down` executable would be invoked after all tests of that sub-suite have been run, even if at least one of them failed.

The `[test]` section (ex: `[tests/ui/unsafe/unsafe-trait-impl.rs]`) would capture attributes that are relevant only for that test.

The `pre_compile` executable would be invoked prior to compiling the test, while the `post_compile` executable would be invoked after compiling the test, even if compilation failed. The `pre_execution` executable would be invoked prior to running the test's executable, while the `post_execution` executable would be invoked after running the test's executable, even if the test's executable failed. Specifying `pre_execution` and/or `post_execution` on a test that does not yield an executable has no effect on the testing run.

When performing testing runs for more than one target, all `config.toml` sections would be prefixed by the target triplet (ex: `[x86_64-unknown-linux-gnu.general]`, `[x86_64-unknown-linux-gnu.tests/ui/unsafe]`).

## Test driver
[test-driver]: #test-driver

The test driver would read the `config.toml` file and consume CLI arguments, locate sub-suites and individual tests, and perform a testing run. Specific testables (ex: `rustc`, `std`) would be supplied to the test driver in the form of CLI arguments. This should allow for automation to test a release `rustc`, for a Rust developer (aided by the Rust infrastructure) to test `liballoc` of a particular stage, etc.

The set of CLI arguments is left somewhat open-ended, however the following arguments should be supported:

- sub-suites and tests - Paths to the sub-suites and tests to include in the testing run.
- `--config=<path>` - Path to the `config.toml` file. A missing `--config` CLI argument could default to the current directory.
- `--exclude=<path>` - Path to an individual sub-suite or test to exclude from the testing run.
- `--host="host"` - The triplet of the host.
- `--target="target"` - The triplet of the target.
- `--testable=<path>` - Path to the tool or library to test.

It is somewhat unclear whether one should supply a path to a library when testing that library. Alternatively, the test driver could accept multiple CLI arguments for each tool (ex: `--cargo=<path>`, `--clippy=<path>`, etc) and `--testable` would be transformed into `--testables=<list>`, where `<list>` is a list of testables (ex: `cargo,clippy,rustc,std`).

## Testing run
[testing-run]: #testing-run

The runtime behavior of a testing run would be as follows:

1. Read `config.toml`, prepare internal data structures, locate tests.
1. Invoke the `[general]`'s set_up executable. If this fails, emit accumulated output, stop the testing run.
1. Start iterating over the sub-suites and tests.
1. If the current sub-suite matches a `[sub_suite]`, then invoke the `[sub_suite]`'s `set_up` executable. If this fails, continue with the next sub-suite.
1. If the current test matches a `[test]`, then
    1. Invoke the `[test]`'s pre_compile executable. If this fails, continue with the next test.
    1. Compile the test.
    1. Record its output.
    1. Invoke the `[test]`'s post_compile executable. If this fails, continue with the next test.
    1. If the test is an executable test:
        1. Invoke the `[test]`'s `pre_execution`` executable. If this fails, continue with the next test.
        1. Run the test's executable.
        1. Record its output.
        1. Invoke the `[test]`'s post_compile executable. If this fails, continue with the next test.
    1. Compare the output of the test against its oracle.
1. If the current sub-suite matches a `[sub_suite]`, then invoke the `[sub_suite]`'s `tear_down` executable. If this fails, fall through.
1. Invoke the `[general]`'s tear_down executable. If this fails, fall through.
1. Emit accumulated output.

## Output
[output]: #output

The test driver would record each test's output in a structured data format such as JSON, and would make it available as a file and/or emit it to stdout/stderr.

## Testing the test driver
[testing-the-test-driver]: #testing-the-test-driver

Testing the test driver itself would involve unit tests, similar to those found in the Rust libraries, and a helper harness.

The harness would invoke the test driver against specialized test suites to verify the behavior of the tool when performing a testing run, and then compare the output against an oracle. For example, to check that the test driver invokes the `[general]`'s `set_up` executable, a dedicated test suite would supply a ready-made `config.toml` that contains a path to a mock executable. The executable itself could simply leave a trace behind, such as an empty file, to signal that it has been invoked. The test harness would then invoke the test driver, passing the path to the `config.toml` file and the test suite directory, and check for the presence of the empty file.

## Use cases
[use-cases]: #use-cases

We envision support for the following use cases:

### Out-of-tree installation
[out-of-tree-installation]: #out-of-tree-installation

**Actor(s)**: Anyone.
**Output**: A stand-alone OOTTS installation.
**Description**:

1. The actor uses `rustup` to download the OOTTS distributable.

    $ rustup toolchain install ???_channel --component ootts
    or
    $ rustup component add ootts

1. `rustup` installs the OOTTS in the Rust root directory. The test driver executable is made visible on the `PATH`.

Alternatively, the actor could download the OOTTS distributable from the Rust Forge, and install it manually.

### Out-of-tree testing
[out-of-tree-testing]: #out-of-tree-testing

**Actor(s)**: Anyone.
**Output**: Test results.
**Prerequisites**: A stand-alone OOTTS installation.
**Description**:

1. The actor produces a `config.toml` file, if necessary.

1. The actor runs the OOTTS, passing in relevant `arguments` such as the target, testables, sub-suites, etc.

    $ test-driver run <arguments>

1. The actor examines the output of the testing run.

### In-tree installation
[in-tree-installation]: #in-tree-installation

**Actor(s)**: Rust developer, automation.
**Output**: An in-tree OOTTS installation.
**Description**:

1. The actor clones the `rust-lang/rust` repository. Since the OOTTS repository is included as a git submodule, it is readily available.

### In-tree building
[in-tree-building]: #in-tree-building

**Actor(s)**: Rust developer, automation.
**Output**: Test driver executable.
**Prerequisites**: An in-tree OOTTS installation.
**Description**:

1. The actor uses the `rust-lang/rust` infrastructure to initiate an OOTTS build.

    rust$ ./x build ootts

Internally, the `rust-lang/rust` infrastructure delegates to the OOTTS infrastructure, effectively executing:

    rust/ootts$ ./infra build

### In-tree testing
[in-tree-testing]: #in-tree-testing

**Actor(s)**: Rust developer, automation.
**Output**: Test results.
**Prerequisites**: Test driver executable.
**Description**:

1. The actor produces a `config.toml` file, if necessary.

1. The actor uses the `rust-lang/rust` infrastructure to run the OOTTS, passing in relevant arguments such as the target, testables, sub-suites, etc.

    rust$ ./x test <arguments>

Internally, the `rust-lang/rust` infrastructure delegates to the OOTTS test driver, effectively executing:

    rust/build/<target>/ootts: ./test-driver run <arguments>

1. The actor examines the output of the testing run.

### In-tree packaging
[in-tree-packaging]: #in-tree-packaging

**Actor(s)**: Release manager, automation.
**Output**: Release artifact.
**Prerequisites**: An in-tree OOTTS installation, test driver executable.
**Description**:

1. The actor uses the `rust-lang/rust` infrastructure to package the OOTS.

    rust$ ./x dist ootts

Internally, the `rust-lang/rust` infrastructure delegates to the OOTTS infrastructure, effectively executing:

    rust/ootts$ ./infra dist

# Drawbacks
[drawbacks]: #drawbacks

Producing an OOTTS as presented by us would require significant effort - new infrastructure would need to be developed, a test driver would need to be implemented, existing tests would need to be relocated and triaged, existing Rust infrastructure would need to be updated, CI and release workflows would need to be updated.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The design we presented borrows many elements from the existing Rust infrastructure, in order to preserve the familiar "feel" and developer workflows, and to potentially leverage existing machinery.

We admit that certain parts of the design have been left open-ended as we do not possess sufficient Rust infrastructure expertise in order to arrive at the best possible realization of our ideas.

It should be noted that at one point we considered the following design:

The OOTTS would not be usable in-tree. Instead, a Rust developer would perform an OOTTS out-of-tree installation, and update the `config.toml` of their `rust-lang/rust` clone to indicate the path to the test driver. Development would then proceed as usual, however a testing run initiated by ``./x test` would require a full `build-dist-install` cycle of a testable prior to invoking the test driver.

Even though this design is closer to the install-then-test paradigm, it severely slows down the Rust developer workflows by effectively carrying out a Rust toolchain "mini" release just to test a small change in a tool or a library. Instead, we opted for a design that allows for the OOTTS to be used in-tree.

# Prior art
[prior-art]: #prior-art

In preparing the design, we took inspiration from the existing Rust infrastructure, DejaGnu, and JUnit 5.

DejaGnu is a framework for testing programs, developed by the GNU Project. DejaGnu consists of a "main script" called `runtest`, several configuration files, and test suites. DejaGnu's configuration files customize various aspects of the framework and tests, and are organized in a hierarchical fashion, where a more "local" configuration file inherits from the "general" configuration file. In our design, we opted for a single `config.toml` configuration file, where the level of customization is specified by a section.

JUnit 5 is a unit testing framework for the Java programming language, developed by Kent Beck and Erich Gemma. JUnit 5 consists of a testing harness and foundational base classes that must be extended by the developer to obtain "test cases". JUit 5 allows a test case to define behavior which is executed before and after running a single or all test functions of a test case, by marking methods with the `@BeforeAll`, `@BeforeEach`, `@AfterAll`, and `@AfterEach` annotations. In our design, we opted for `config.toml` attributes `set_up` and `tear_down` as equivalents to `@BeforeAll` and `@AfterAll` both at the general and sub-suite level, and attributes `pre_xxx` and `post_xxx` as equivalents to `@BeforeEach` and `@AfterEach` at the individual test level.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

We identified several outstanding questions that would affect the design of the OOTTS and the effort to materialize it.

**Should the OOTTS repository be a sub-repository of rust-lang, or should it live directly under rust-lang/rust?**

From what we observed, "major" tools such as `cargo` and `rust-analyzer` have dedicated `rust-lang` sub-repositories. Given that OOTTS is envisioned to be an independent tool with its own release, it makes sense for it to live in a sub-repository. On the other hand, due to the possibility of reusing infrastructure, it may be more appropriate for OOTTS to live directly in `rust-lang/rust/tools`.

**How should the tests directory of the OOTTS be organized?**

Currently, the various sub-suites within `rust-lang/rust` are somewhat dispersed - directory `tests` contains compiletest tests, directory `library/alloc/tests` contains all `liballoc` tests, directory `src/tools/clippy/tests` contains all Clippy tests, etc.

Relocating all tests under OOTTS could be an opportunity to introduce a uniform hierarchical structure, similar to the testables-based layout we presented.

**Should various sub-suites and tests be standardized?**

Currently, compiletest tests differ from library and tool tests. Compiletest tests employ special test headers that offer flexibility, but require a dedicated tool to process. The remaining tests on the other hand employ test functions, which ultimately leverage `libtest`.

Relocating all tests under OOTTS could be an opportunity to standardize all tests, most likely using the powerful compiletest's test headers. Standardization however would involve a lot of work, some of which could be automated.

**How should the OOTTS infrastructure be implemented?**

We proposed that the OOTTS infrastructure be based on the existing Rust infrastructure, employing the usual `Builder`, `Step`, etc. Depending on the choice of OOTTS repository, large chunks of the existing Rust infrastructure may need to be refactored, and made usable by both infrastructures.

**What is the best way to configure the OOTTS test driver?**

We proposed that the OOTTS driver be configured via a `config.toml` file, where specific sections define behavior for various events. Depending on the degree and amount of customization, it may be more reasonable to employ multiple configuration files, similar to DejaGnu.

**What are the test driver CLI arguments?**

Currently, ``./x test` accepts a large number of CLI arguments. We already outlined several "must have" CLI arguments that the test driver should support, but given that the OOTTS is supposed to be a full testing replacement, most or all `./x test` CLI arguments may need to be migrated.

# Future possibilities
[future-possibilities]: #future-possibilities

We foresee the following future extensions to the OOTTS:

**Testing support for no_std targets**

Currently, it is not possible to test a no_std target for two reasons:
- Test functions (and perhaps `compiletest`?) depend on `libtest`, which depends on `libstd`, and there is no `libstd` available on a `no_std` target.
- There is no convenient support for IO, networking, file systems, etc. on bare metal targets.

The OOTTS could be extended to contain a "testing `libstd`", which is used exclusively for testing `no_std` targets. When the test driver compiles a test, the test would be linked against the testing `libstd`.

**Language conformance testing**

The OOTTS could contain the Rust language conformance test suite, once the work on the Rust Language Specification has been completed.

**General testing framework**

The OOTTS could be greatly generalized to become a language-independent testing framework.
