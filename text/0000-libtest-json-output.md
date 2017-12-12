- Feature Name: libtest_json_output
- Start Date: 2017-12-04
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Add a machine-readable JSON-output mode for Rust's libtest.

# Motivation
[motivation]: #motivation

Adding a machine-readable output for Rust's built-in tests will make external tool integration easier.

Having this feature is not intended to replace the proposed custom test runners, but to enrich
the default set of features offered by rust out-of-the-box.

The proposed format is not intended to be the end-all be-all generic output format for all test-runner in the Rust ecosystem.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Test binaries compiled `cargo test` or `rustc --test` allow the user to specify an output format using the `--format` flag.

Using this flag, the format can be selected from one of the following:
 - `pretty`: The default setting; print test results in a detailed manner.
 - `terse`: Equivalent to `-q`; display one character per test instead of one line.
 - `json`: Print to stdout the test result in json format. Each line is a complete JSON document describing one event.

Each JSON object starts with the "type" property, which is one of the following:

 - `suite`: Events regarding the whole testing suite; consists of a header specifying the amount of tests to be performed,
 	and a footer with results summary.
 - `test`: A change in a test's state, along with its name. Can be one of the following:
 	- `started`: Printed when a tests starts running.
 	- `ok`
 	- `failed`: Printed along with the test's stdout, if non empty.
 	- `ignored`
 	- `allowed_failure`
 	- `timeout`
 - `bench`: Benchmark results, specifying the median time and the standard deviation

The events are printed as-they-come, each in its own line, for ease of parsing.

**Examples**:
Notice how most output look mostly identical to their pretty-printed versions.

Suite Events:
```json
{ "type": "suite", "event": "started", "test_count": "2" }

{ "type": "suite", "event": "failed", "passed": 1, "failed": 1, "allowed_fail": 0, "ignored": 0, "measured": 0, "filtered_out": "0" }
```

Test Events:
```json
{ "type": "test", "event": "started", "name": "ignored" }
{ "type": "test", "event": "ignored", "name": "ignored" }

{ "type": "test", "event": "started", "name": "will_succeed" }
{ "type": "test", "event": "ok", "name": "will_succeed" }

{ "type": "test", "event": "started", "name": "will_fail" }
{ "type": "test", "event": "failed", "name": "will_fail", "output": "thread 'will_fail' panicked at 'assertion failed: false', f.rs:12:1\nnote: Run with `RUST_BACKTRACE=1` for a backtrace.\n" }

```

Benchmarks:
```json
{ "type": "bench", "name": "bench_add_two", "median": 39, "deviation": 2 }
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Currently libtest's output is tightly coupled with its test result.
In order to implement this RFC, there's a need to refactor libtest in order to account for different output formats.

The following trait is introduced to mediate between the test runner and the desired output format:
```rust
trait OutputFormatter {
    fn write_run_start(&mut self, len: usize) -> io::Result<()>;
    fn write_test_start(&mut self,
        test: &TestDesc,
        align: NamePadding,
        max_name_len: usize) -> io::Result<()>;
    fn write_timeout(&mut self, desc: &TestDesc) -> io::Result<()>;
    fn write_result(&mut self, desc: &TestDesc, result: &TestResult) -> io::Result<()>;
    fn write_run_finish(&mut self, state: &ConsoleTestState) -> io::Result<bool>;
}
```

This trait is but an implementation detail, and is not meant to be exposed outside of libtest.

Using this trait and the CLI option `--format`, libtest can be easily extended in the future to support other output formats.

# Drawbacks
[drawbacks]: #drawbacks

- This proposal adds a new API to which the toolchain must adhere, increasing the chance of accidental breakage in the future.

# Rationale and alternatives
[alternatives]: #alternatives

- Simply not doing this. 
	There are proposals for custom test runners, which IDE's can use in order to programatically run code.
	This solution is more complex then JSON, and requires the use of Rust on the IDE's side, which is not always the case:
		- VS-Code is JS
		- InteliJ Rust is Kotlin (?)

# Unresolved questions
[unresolved]: #unresolved-questions

- If serde is used to serialize the output, should we expose the structs used for serialization as an API of libtest?

# Prior art
[prior-art]: #prior-art
 - https://github.com/rust-lang/rfcs/pull/1284
 - http://jsonlines.org
 - https://firefox-source-docs.mozilla.org/mozbase/mozlog.html#data-format
 - https://github.com/dart-lang/test/blob/master/doc/json_reporter.md
 - https://testanything.org/tap-version-13-specification.html & https://github.com/rubyworks/tapout/wiki/TAP-Y-J-Specification