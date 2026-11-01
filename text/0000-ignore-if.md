- Feature Name: `ignore_if`
- Start Date: 2022-01-08
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3221)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

This RFC extends the `#[ignore]` annotation to accept named arguments
and most importantly `if` parameter which specifies a predicate to
ignore `#[test]` function based on a run time check.


# Motivation
[motivation]: #motivation

There are situations in which a test may need to be skipped based on
the run time environment it is executed in.  For example:

* A library provides a `memcpy` function with `memcpy_generic` and
  `memcpy_sse` implementations chosen at run time.  To have good code
  coverage, the library defines `test_memcpy_generic` and
  `test_memcpy_sse` tests.  Executing the latter test on machines
  without SSE support should neither pass nor fail the test since
  inability to run the test doesn’t indicate lack of bugs nor reveal
  a bug in the implementation.

* Like above, a library provides a `filecopy` function with various
  implementations dependent on kernel version and multiple test
  functions some of which require sufficiently new OS kernel.

* Like above, a library provides a `download` function which executes
  `wget` or `curl` depending on which is available on the system and
  separate tests for each variant.

* A project splits tests into fast and slow.  By default, `cargo test`
  runs only fast tests while slow tests are executed if some specific
  environment variable is set, e.g. `RUN_SLOW_TESTS=true cargo test`.
  Not running the slow tests should not result in `cargo test` failure
  but seeing the slow test as passed gives incorrect impression that
  they had been run.

The `#[ignore]` directive already provides a mechanism for ignoring
tests, but it works at compile time making it insufficient for the
above situations.  One could argued that the first three cases could
be handled by a compile-time check, alas this is not the case because
build environment may be completely different from the environment the
tests are run on.  For example,

* when cross-compiling, compiler has no access to the actual machine
  the tests will run on,

* compilation may happen in a build farm whose nodes differ from hosts
  the tests will run on,

* compilation may happen inside of a container with a limited
  environment lacking some software or access to some hardware.

The last case is one which could theoretically be handled at compile
time and it’s the approach [`test-with`
crate](https://crates.io/crates/test-with) takes but that requires
compiling the code multiple times and doing clean build each time
(e.g. `cargo clean; RUN_SLOW_TESTS=true cargo test`).


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

In addition to `#[ignore]` and `#[ignore = "reason"]` syntax, the
`ignore` attribute supports two named parameters: `reason` and `if`.
`reason` parameter offers an alternative syntax for giving reason the
test is ignored.  `if` parameter takes a predicate function as the
value and causes the test to be ignored if the predicate returns true
when the test program is run.  For example:

```rust
fn missing_avx_support() -> bool {
    !std::is_x86_feature_detected!("avx")
}

#[test]
#[ignore(if = missing_avx_support, reason = "missing AVX support")]
fn test_memcpy_avx() {
    // ...
}
```

Multiple `ignore` annotations can be specified.  If any of them have
no `if` predicate the test is unconditionally ignored and none of the
predicates (if any) are called.  Otherwise, the test is ignored if any
of the predicates return true.  For example:

```rust
fn missing_avx_support() -> bool {
    !std::is_x86_feature_detected!("avx")
}

fn missing_fma_support() -> bool {
    !std::is_x86_feature_detected!("fma")
}

#[test]
#[ignore(if = missing_avx_support, reason = "missing AVX support")]
#[ignore(if = missing_fma_support, reason = "missing FMA support")]
fn test_feature() {
    // ...
}

fn missing_feature() -> bool {
    panic!("This is never called")
}

#[test]
#[ignore(if = missing_feature)]
#[ignore]
fn test_another_feature() {
    // ...
}
```

If multiple tests use the same predicate function, the test harness
caches results of check such that predicates won’t be called more than
once.  This means that even when the predicate function is impure, if
multiple tests use it either all or none of them will be ignored.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

There are two places this feature would require modifications to.

First of all, handling of the `ignore` directive.  To be completely
honest, I’ve skimmed through the code handling the annotation and
didn’t really understood what it was doing.  There is `ignore`
handling in `expand_test_or_bench` function but it doesn’t even handle
reason so I really don’t get what is going on there.

The other change is in libtest and shouldn’t be too complex.  Namely,
the `ignore` field of `TestDesc` would need to be changed to
`std::lazy::Lazy<bool, IgnorePredicat>` where:

```rust
struct IgnorePredicate {
    ignore: bool,
    funcs: Vec<std::sync::Arc<std::lazy::Lazy<bool>>>
}

impl std::ops::FnOnce<()> for IgnorePredicate {
    type Output = bool;
    extern "rust-call" fn call_once(self, _args: ()) -> bool {
        self.ignore || self.funcs.into_iter().any(|pred| *pred)
    }
}
```

When constructing `TestDesc` the predicate functions would need to be
collected with a help of a temporary hash map from function pointer to
`Arc<std::lazy::Lazy<bool>>` so that predicates are called just once
when used by multiple tests.

With this approach the property of predicates being called at most
once would be fulfilled and since reading the `ignore` field would
work (almost the same) as before the feature would integrate easily
with libtest.  In particular it would work with `--ignored` and
`--include-ignored` flags.


# Drawbacks
[drawbacks]: #drawbacks

As always, adding a new feature means that it needs to be maintained.
However, with `#[ignore]` attribute already present, inability to
decide at run-time whether test should be ignored is an obvious
omission.  In a way, libtest supporting `#[ignore]` invited this
request for the feature described in this RFC.

Another concern might be that adding the feature interferes with new
features in the future.  However, because the proposal is to make
`ignore` attribute accept named parameters, it is future-proof as new
named parameters can be added if desired.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

As typically is the case, there are many alternative ways to approach
the issue.  Some are just matter of taste and are covered in the
‘Bike-shedding’ subsection below.  While this RFC proposes a certain
specific syntax, the author isn’t really concerned with how exactly
the syntax looks.  The other approaches to solve the issue are listed
further down this section.

## Bike-shedding

### Separate directive

Rather than changing `ignore` attribute, alternative approach is to
introduce a new `ignore_if` directive which takes predicate as an
argument, e.g.:

```rust
fn missing_avx_support() -> bool {
    !std::is_x86_feature_detected!("avx")
}

#[test]
#[ignore_if = missing_avx_support]
fn test_memcpy_avx() {
    // ...
}
```

It’s not clear however how reason would be specified with this syntax.
It would be rather confusing if `ignore` allowed it to be given but
`ignore_if` didn’t.  Having `ignore_if` accept named arguments with
optional `reason` would work but at that point we might just as well
stick to `ignore` directive.  Alternatively, the predicate function
could return the reason, e.g.:

```rust
fn missing_avx_support() -> Option<String> {
    (!std::is_x86_feature_detected!("avx"))
        .then(|| "Missing AVX support".into())
}

#[test]
#[ignore_if = missing_avx_support]
fn test_memcpy_avx() {
    // ...
}
```

While this would work, it means that people who don’t care about the
reason would be forced to deal with it.  At the moment users may be
completely oblivious to reason and this RFC proposes that it remains
so.  To mitigate that, the predicate could be allowed to return one of
various types (similarly how `termination_trait_lib` allows `main` to
return anything implementing `Termination`).  However, that
complicates the feature and is an unnecessary complication for initial
implementation.

### Naming

There’s a matter of naming the argument.  Rather than `if` it could be
called `unless` with the result of the check negated.  Other options
are also available such as `predicate` but those are less
self-documenting.

### Predicate function vs expression

Rather than accepting a predicate function the `if` parameter could
accept an expression.  For example:

```rust
#[test]
#[ignore(if = !std::is_x86_feature_detected!("avx"))]
fn test_memcpy_avx() {
    // ...
}
```

This would allow avoiding writing functions for simple checks but is
harder to implement (especially considering that this RFC proposes
that predicates are guaranteed to be called at most once) and doesn’t
really offer any additional features so this proposal chose to go with
the simpler function pointer route.

## Returning ignored status

Rather than having test functions declared as ignored via a directive,
the check could be made within the function and communicated to the
test harness by returning specified value.  For example:

```rust
#[test]
fn test_memcpy_avx() -> std::process::ExitCode {
    if !std::is_x86_feature_detected!("avx") {
        return std::process::ExitCode(125);
    }
    // ...
    std::process::ExitCode::SUCCESS;
}
```

or:

```rust
#[test]
fn test_memcpy_avx() -> std::test::TestResult  {
    if !std::is_x86_feature_detected!("avx") {
        return std::test::TestResult::IGNORED;
    }
    // ...
    std::test::TestResult::SUCCESS;
}
```

This has a few potential problems.

Most importantly, it wouldn’t play nice with `--include-ignored` and
`--ignored` options.  Normally, those flags allow running tests even
if they are marked `#[ignore]`.  User may choose to run such a test
because they are testing a fix or want to see if the requirements
predicates check are still valid.  By having the test return ‘ignore’
value user would be unable to force-run an ignored test. Since the
`--include-ignored` and `--ignored` options exist, a solution that
work with them should be prioritised.

Secondly, the approach with `ExitCode` would require definition of
a ‘magic’ integer which indicates test has been ignored.  This is not
uncommon amongst tools which call arbitrary commands to perform tests,
but doesn’t feel idiomatic for Rust where we’d rather leverage the
type system for our needs. Using `std::test::TestResult` type would
address that particular issue (and internally could be implemented by
having a private `TestTermination` trait which is implemented for
everything `Termination` is plus `TestResult`).

Both of those alternatives would require features which aren’t
currently stable.  Using `ExitCode` would be blocked on
`process_exitcode_placeholder` feature while defining custom public
`TestResult` type would require test module to be stabilised which, as
far as I understand, is not going to happen.

There is also a minor disadvantage that making a test conditionally
ignored involves more changes than with the proposed `#[ignore(if=…)]`
syntax.  Namely, in addition to adding the check, the signature of the
function must be altered and all return points of the function must be
modified to return `Ok(())`.

## Panicking

Rather than returning a value, the test could exit by panicking with
a special message (`[1]` in example below).  To avoid having a magic
string pattern, a better option would be panicking with a special
object (`[2]` below).  Or finally, to make things more convenient
a custom function (`[3]` below) or macro (`[4]` and `[5]` below) could
be defined instead.

```rust
#[test]
fn test_memcpy_avx() {
    if !std::is_x86_feature_detected!("avx") {
        /* [1] */ panic!("IGNORE: missing AVX support");
        /* [2] */ std::panic::panic_any(
                      std::test::IgnoreTest::new("missing AVX support"));
        /* [3] */ std::test::ignore("missing AVX support");
        /* [4] */ ignore!("missing AVX support");
    }
    /* [5] */ ignore_if!(!std::is_x86_feature_detected!("avx"),
                         "missing AVX support");
    // ...
}
```

Like before, this approach does not integrate with `--ignored` option.
The `std::test::ignore` function, `ignore!` and `ignore_if!` macros
could be made to respect `--include-ignored` by not interrupting the
test.  However, that would be surprising to the test authors (who
would expect test to terminate if condition isn’t met) as well as
users (who would observe inconsistent behaviour with `--ignored`
flag).

In addition, the second and third variants would require test module
to be stabilised which might not be feasible.

## Passing an argument

Tests could be made to accept an argument which allows marking test as
skipped.  For example:

```rust
#[test]
fn test_memcpy_avx(mut test: std::test::TestRun) {
    if !std::is_x86_feature_detected!("avx") {
        test.skip("missing AVX support");
    }
    // ...
}
```

Under the hood, the method would be implemented by panicking as
described above.  The advantage over simply panicking would be that
the introducing the `TestRun` object would offer simple way for any
future extensions.

Like before, the issue is lack of support for `--ignored` option and
requirement for `std::test` to be stabilised..


# Prior art
[prior-art]: #prior-art

The feature has recently been discussed in at least two places: [on
Rust Programming Language Internals
Forum](https://internals.rust-lang.org/t/pre-rfc-skippable-tests/14611)
and [Rust GitHub
Issue](https://github.com/rust-lang/rust/issues/68007).  There’s also
a [`test-with` crate](https://crates.io/crates/test-with) which
addresses similar issue but because of lack of `ignore_if` performs
all its checks at compile time (which is not sufficient as described
in Motivation section).

The feature is available in many existing languages and test
frameworks.  Frameworks can be divided into two broad classes: ones
which run external test programs and ones which are integrated within
the source code and provide a test harness.  Implementations are
usually very similar so this section concentrates on only a handful
examples showing existing approaches.

## Frameworks running external test programs

The commonality in this category is limited ways in which a test can
indicate its result.  Since the harness executes the test as external
process and has no visibility into its internal state, it can only
inspect test’s exit code and output.

Because of this limitation, such frameworks may not be the best to
compare Rust to.  On the other hand, thinking about Cargo as a build
system, having a way to interpret result of an arbitrary executable as
tests would allow Cargo to run and correctly interpret arbitrary test
commands.  But even without such Cargo-level feature, there is a need
for libtest to support skipping tests conditionally.

### Autoconf and Automake

GNU Automake and GNU Autoconf are tools which automate generating
build and configuration scripts for software.  Automake supports
running commands as a test suite via its
[`TESTS`](https://www.gnu.org/software/automake/manual/html_node/Scripts_002dbased-Testsuites.html)
variable.  Any program in the list which returns with a status code of
77 is considered to have been skipped.  Similarly, Autoconf supports
skipping tests through the
[`AT_SKIP_IF`](https://www.gnu.org/savannah-checkouts/gnu/autoconf/manual/autoconf-2.69/html_node/Writing-Testsuites.html#index-AT_005fSKIP_005fIF-2289)
macro.  This uses the same 77 return code expectation.

### CMake

CMake, a C++ build system, supports ‘test properties’ named
[`SKIP_RETURN_CODE`](https://cmake.org/cmake/help/latest/prop_test/SKIP_RETURN_CODE.html)
and
[`SKIP_REGULAR_EXPRESSION`](https://cmake.org/cmake/help/latest/prop_test/SKIP_REGULAR_EXPRESSION.html)
which cause the test to be skipped if it exits with the indicated
return code or its output matches the given regular expression.  This
state is not reported as success or failure but as a third state of
‘skipped’ (rendered on CDash in a "Not Run" column).

```cmake
add_test(NAME skip COMMAND …)
set_test_properties(skip PROPERTIES
  SKIP_RETURN_CODE 125
  SKIP_REGULAR_EXPRESSION "SKIPME")
```

```c++
#include <stdio.h>
int main(int argc, char* argv[]) {
    puts("SKIPME"); // Will cause the test to be skipped.
    return 125;     // As will this; either is sufficient, both are available.
}
```

### Test Anything Protocol (TAP)

[The Test Anything Protocol
(TAP)](https://testanything.org/tap-specification.html) is definition
of a text-based interface used by Perl test modules.  It works by
parsing output of a test and allows marking tests as skipped via `#
skip` comment, for example:

```text
1..5
ok 1
ok 2
not ok 3
ok 4 # skip missing SSE2 support
ok 5 # skip missing AVX support
```

## Source-code level frameworks

Test frameworks which work on source-level have greater visibility
into the state of the test and have many more options of communicating
with it.  As such they offer more integrated ways for tests to
indicate they should be skipped.

### Emacs Lisp Regression Testing (ERT)

In [Emacs Lisp Regression Testing
(ERT)](https://www.gnu.org/software/emacs/manual/html_node/ert/index.html)
tests [can be skipped at
run-time](https://www.gnu.org/software/emacs/manual/html_node/ert/Tests-and-Their-Environment.html#index-skipping-tests)
by using `skip-unless` form.  For example:

```lisp
(ert-deftest test-dbus ()
  "A test that checks D-BUS functionality."
  (skip-unless (featurep 'dbusbind))
  ...)
```

Skipped tests are counted separately as neither passed nor failed.

```text
Selector: test-dbus
Passed:  0
Failed:  0
Skipped: 1
Total:   1/1
```

Under the hood this is implemented by generating a signal (what other
languages would call an exception) which is caught by the test
harness.  This would be akin to using `std::panic::panic_any`.

### Golang

Golang [provides a `testing` package](https://pkg.go.dev/testing) and
all tests are run with `testing.T` object passed to them.  That object
has `Skip` and `SkipNow` methods which can be used to [skip tests at
run time](https://pkg.go.dev/testing#hdr-Skipping).  For example:

```go
func TestTimeConsuming(t *testing.T) {
    if testing.Short() {
        t.Skip("skipping test in short mode.")
    }
    ...
}
```

Under the hood the methods mark the test as having been skipped and
stop its execution by calling `runtime.Goexit` which terminates the
current goroutine (which is akin to stopping a thread).  Tests which
both fail and skip the test (e.g. call `t.SkipNow()` as well as
`t.FailNow()`) are considered failed.

### Pytest

In Python’s [pytest](https://docs.pytest.org/) framework test can be
conditionally skipped with one of two methods:

- by adding [`pytest.mark.skipif`
  annotation](https://docs.pytest.org/en/6.2.x/reference.html#pytest-mark-skipif)
  takes a boolean argument which specifies whether test should be
  skipped and a required reason argument; or

- by calling [`pytest.skip`
  function](https://docs.pytest.org/en/6.2.x/reference.html#pytest-skip)
  which takes a reason argument and under the hood throws an internal
  `Skipped` exception.

The first method is analogous to this proposal with the difference
that pytest went the path of having two separate marks: `skip` for
unconditional skipping a test and `skipif` for doing it conditionally.
The second method is analogous to the `std::test::skip` alternative
discussed above.

Skipped tests are marked as such and not counted towards passed or
failed tests.

```text
collected 1 item

test_example.py s                                               [100%]

========================= 1 skipped in 0.00s =========================
```

## Prior art for the attribute syntax

While not related to testing itself, it’s worth to look at precedent
in the syntax proposed by this RFC.  [Serde](https://serde.rs/),
a popular serialising library, supports customisation with [`serde`
annotation](https://serde.rs/field-attrs.html).  The annotation takes
named parameters some of which accept further value.  Most notably,
the [`skip_serializing_if`
argument](https://serde.rs/attr-skip-serializing.html) takes
a predicate function as value, for example:

```rust
#[serde(deny_unknown_fields)]
pub struct Notification {
    jsonrpc: Version,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}
```

The method is passed as a string rather than plain path because the
parameter was introduced before [`unrestricted_attribute_tokens`
feature](https://github.com/rust-lang/rust/pull/57367) was stabilised
and it was simply not possible to pass a path to an annotation.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

- Should there be a new directive instead?  If so, what’s the syntax
  for specifying reason.

- What’s the name of the parameters?  `if`, `unless`, `predicate` or
  something else entirely?

- Should predicates be guaranteed to be called in order if multiple
  `ignore` directives were specified?

- How, if at all, does this interact with custom test frameworks?


# Future possibilities
[future-possibilities]: #future-possibilities

By making `#[ignore]` accept named parameters this proposal opens
possibility for other extensions to the attribute if those are ever
desired.  As such this proposal is rather future-proof in this regard.

There are potential ‘syntactic sugar’ changes.  For example:
* allowing the predicate to be expressions.  This could be implemented
  by checking whether the value of the parameter is just a path or
  a more complex stream of tokens;
* providing a default for the reason.  For example `#[ignore(if =
  missing_avx)]` could set the reason to ‘because of missing_avx’;
* supporting both `if` and `unless` parameters such that user can pick
  whichever works better in given situation; and
* supporting more complex conditions, e.g. `unless_var = VAR` could
  ignore test unless environment variable is set.

In the future the predicate could also support giving the reason
rather than just being a boolean function.  In this case the reason
parameter would be ignored.  This could be implemented by having
a private `IgnorePredicate` trait with an `is_ignored(self, &str) ->
Option<String>` function implemented for `bool` and `Option<String>`
types.
