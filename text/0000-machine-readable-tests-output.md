- Feature Name: machine\_readable\_tests\_output
  - Start Date: 2015-09-17
  - RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Replace current test output with machine-readable one.

# Motivation

Currently when testing Rust provide only human-readable output which is nice
in most cases as we are humans or other humanoids. But from time to time we need
to feed machine with results of our tests (i.e. some kind of CI that will report
which tests failed or something like that) and there is no way to do that in
"civilised" way. We need to parse non-machine-readable output which isn't nice.

# Detailed design

Replace current, human-readable, output with JSON-based output based on
[TAP-J][tap-j]. For compatibility with existing work flow there should be added
built-in parser for that protocol which will output in current format.

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be
interpreted as described in [RFC 2119][rfc2119].

## Protocol description

Output of test MUST be stream of JSON objects that can be parsed by external
tools.

### Structure

Output is stream of lines containing JSON objects separated by new line. Each
object MUST contain `type` field which designates `suite`, `test`, `bench` or
`final`. Any document MAY have `extra` field which contains an open mapping
for additional, unstandardized fields.

### Suite

  ```json
{
  "type": "suite",
  "name": "Doc-Test",
  "build": "2015-08-21T10:03:20+0200",
  "count": 13,
  "rustc": "2a89bb6ba033b236c79a90486e2e3ee04d0e66f9"
}
```

Describes tests suite. It MUST appear only once at the beginning of each stream.

Fields:

| Name    | Description                                                    |
| ----    | -------------------------------------------------------------- |
| `type`  | MUST be a `suite`                                              |
| `build` | MUST be ISO8601 timestamp of build.                            |
| `name`  | OPTIONAL suite name i.e. "Tests", "Benchmarks" or "Doc-Tests". |
| `count` | MUST be count of all tests (including ignored).                |
| `rustc` | MUST be version of Rust compiler used to build test suite.     |

### Test

```json
{
  "type": "test",
  "subtype": "should_panic",
  "status": "ok",
  "label": "octavo::digest::md5::tests::test_md5",
  "file": "src/digest/md5.rs",
  "line": 684,
  "stdout": "",
  "stderr": "",
  "duration": 100
}
```

Describes test. Each test MUST produce only one test object.

Fields:

| Name         | Description                                                                    |
| ----         | -----------                                                                    |
| `type`       | MUST be `test`.                                                                |
| `subtype`    | SHOULD be one of `test`, `bench` or `should_panic`. Defaults to `test`.        |
| `status`     | MUST be one of `ok`, `fail` or `ignore`.                                       |
| `reason`     | MUST describe of failure if `status` is `fail`. Otherwise MUST NOT be present. |
| `label`      | MUST be unique identifier of test.                                             |
| `file`       | OPTIONAL file name containing test.                                            |
| `line`       | OPTIONAL line in `file` that contains test.                                    |
| `stdout`     | OPTIONAL output of standard output for given test.                             |
| `stderr`     | OPTIONAL output of standard error for given test.                              |
| `duration`   | OPTIONAL test execution duration in nanoseconds. MUST be present in benchmark. |
| `throughput` | OPTIONAL test throughput in case of benchmark test.                            |

### Final

```json
{
  "type": "final",
  "results": {
    "ok": 10,
    "fail": 0,
    "ignore": 2
  }
}
```

Finish test suite and closes stream. Parser MUST ignore all other input unless
it is new suite.

Fields:

| Name      | Description                                                                                                                                 |
| ----      | -----------                                                                                                                                 |
| `type`    | MUST be `final`.                                                                                                                            |
| `results` | MUST be object containing only 3 fields: `ok`, `fail` and `ignore` which describe how many test passed, failed or was ignored respectfully. |


# Drawbacks

This is breaking change in tooling and will require new tool that will provide
current functionality for compatibility reasons, but IMHO small pain for big gain.

# Alternatives

Do nothing.

# Unresolved questions

Test object fields:

- Should there be any additional fields? How should benchmark test description
  look like?
- Maybe there should be additional, optional field `nspi` containing times
  for each iteration that would help with statistical analysis of benchmarks.

Object separator: is newline enough? Maybe something that will be simpler to parse.

Format: should additional formats be available? Is support only for JSON enough?

[tap-j]: https://github.com/rubyworks/tapout/wiki/TAP-Y-J-Specification
[rfc2119]: https://www.ietf.org/rfc/rfc2119.txt
