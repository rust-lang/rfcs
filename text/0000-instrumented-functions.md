- Feature Name: instrument-functions
- Start Date: 2026-02-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

This feature provides a means to insert a function call into the prologue or epilogue of each function.
This RFC presents an option to consolidate the various similar, yet mutually exclusive mechanisms to
instrument functions.

## Motivation
[motivation]: #motivation

There are at least three different ways to instrument functions with gcc and clang, and some have
partial support in Rust today:

* mcount, a counting function inserted into the prologue of each function which takes the caller and callee
addresses, and performs interesting things with them. This is historically used with the prof or gprof
utilities available on GNU/Linux or BSD.
* fentry, a derivative of mcount with a slightly different ABI. It is meant to intercept function entry
to inspect or manipulate arguments as well as the traditional mcount features
* [XRay](https://llvm.org/docs/XRay.html), an LLVM project to instrument both entry and exit of functions,
with dynamic enablement.

These features are very similar, and are effectively mutually exclusive (e.g., mcount and fentry).

mcount deserves a little extra background. This feature has existed on many C toolchains for decades, and
gcc/clang have developed extensions to support novel features like Linux's [ftrace](https://docs.kernel.org/trace/ftrace.html).
Those features include tracking the location of instrumentation insertion (`-mrecord-mcount`), and the
ability to generate nop's in place of the call (`-mnop-mcount`).

Linux's ftrace supports using patchable functions or mcount instrumentation. Kernel maintainers may choose one
over the other, and may make different choices depending on the architecture. Rust should implement
function instrumentation which supports mcount.


## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Today, Rust supports experimental disparate options to enable function instrumentation using XRay and mcount,
possibly simultaneously.

This RFC proposes a unified set of options to enable one instrumentation framework, and provide sufficient
language integration to control individual function instrumentation.

Function instrumentation is generally limited to inserting a counting function into the entry and exit
of each function. A counting function is one which takes the caller's and callee's address as arguments,
but may have sufficient visibility to inspect and modify callee arguments, or reroute to an entirely
different function call.

For each target, a counting function is inserted into a specific part of a function's entry or exit. For
targets which support gprof, a specific target defined counting function is expected to be called. For
example, on x86_64-linux `__fentry__` or `mcount` is the expected counting function. The ABI of these
functions is expected to be stable despite their lack of documentation (e.g., glibc provides these symbols).

gcc and clang provide the following options which are used to instrument functions for common usages:
* ftrace: Linux only, `-pg` or `-pg -fentry` with `-mrecord-mcount` and optionally `-mnop-mcount`.
* gprof: `-pg` or `-pg -fentry` with an altered set of crt libraries (see `gcrt1.o` vs `crt1.o` on glibc).
* XRay: clang/llvm only, `-fxray-instrument` which links a special compiler-rt runtime.

The Rust compiler should support flags which offer interoperability with these gcc and clang options as
Rust compiled code will need to link with gcc and clang compiled code which uses these options and
work as expected.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

As of the creation of the RFC, the unstable options `-Zinstrument-mcount` and `-Zinstrument-xray` exist, and no
options exist to enable `fentry` instrumentation.

### Additional options to rustc

`-Zinstrument-function` shall be added. It will take the instrumentation framework as a string argument:
  * `never`: Do not instrument anything. This is the default option.
  * `mcount`: Instrument function entry with the target's mcount function (if supported).
  * `fentry`: Instrument function entry with the target's fentry function (if supported).
  * `xray`: Instrument function entry and exit with XRay (if supported).

The options `-Zinstrument-mcount` and `-Zinstrument-xray` will be removed in favor of
framework specific configuration options described below.

`-Zinstrument-mcount-opts`, `-Zinstrument-fentry-opts`:
  * `record`, `no-record`: Record each call to the counting function in a separate binary section, or not.
  * `call`, `no-call`: Insert a call to the counting function or a nop of equal size.

`-Zinstrument-xray-opts`:
  * `ignore-loops`: Ignore loop behavior when deciding to instrument a function.
  * `instruction-threshold=10`: Set a different instruction threshold for instrumentation.
  * `no-entry`: Do not instrument function entry.
  * `no-exit`: Do not instrument function exit.

Finally, a single builtin attribute will be added to control the insertion of the counting function. The
default options for each framework will be documented and stable.

Each of these features is controlled by applying a set of function attributes to each LLVM generated
function. The implementation will apply the correct set of LLVM function attributes.

Example usage might be:
```shell
$ RUSTFLAGS="-Zinstrument-function=mcount -Zinstrument-mcount-opts=record,call" cargo build

$ RUSTFLAGS="-Zinstrument-function=fentry -Zinstrument-fentry-opts=record,no-call" cargo build

$ RUSTFLAGS="-Zinstrument-function=xray -Zinstrument-xray-opts=ignore-loops" cargo build
```

### Language additions

A single builtin attribute, `instrument_fn`, will be added. It will be applied to functions and methods
only. It will accept two list entries `entry="on|off"` and `exit="on|off"`. Additionally, a simpler form
`#[instrument_fn = "off"]` will disable all instrumentation.

Usage will look like the following:

```rust
#[instrument_fn(entry = "off")]
fn no_entry_instrument() {
}

#[instrument_fn(exit = "off")]
fn no_exit_instrument() {
}

#[instrument_fn(entry = "off", exit = "off")]
fn no_instrument_verbose() {
}

#[instrument_fn = "off"]
fn no_instrument_terse() {
}
```

Likewise, if no instrumentation is enabled, this attribute is quietly ignored.

### Other changes

Supporting gprof requires special linker support. Rust's integration with the linker will need to
facilitate linking the correct crt objects when creating binaries. On glibc targets, this requires
replacing `crt1.o` with `gcrt1.o`.

## Drawbacks
[drawbacks]: #drawbacks

The counting function ABI is not documented for most targets. Using this option could potentially generate
incompatible calls if the ABI requirements of the counting function change.

Likewise, gprof is likely not a compelling argument today with substantially more advanced statistical
profilers like `perf`, and this is not meant to be a code coverage tool.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Patchable function entries have many overlapping features. So much so that the Linux kernel can also use them
to implement instrumentation on many architectures. Though, some architectures still use mcount/fentry even
if patchable entries are available.

Likewise, there may not be reason to bundle all function instrumentation into a single set of options.

## Prior art
[prior-art]: #prior-art

Similar features exist in gcc and clang as noted above. This extends those features into Rust.

When using mcount or fentry with recording and nop insertion, this feature can behave similarly
to patchable-function-entries presented in rfc#3543. There are some minor differences in the details
of how nops are recorded. However, both can be used simultaneously without issue. This is the case
for some Linux kernel configurations (e.g., x86-64 on fedora at the time of writing).

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- Can we stabilize the mcount portions of this RFC? The underlying counting functions have target specific
  ABI which is mostly undocumented, but has historically been stable.

## Future possibilities
[future-possibilities]: #future-possibilities

This RFC assumes only one form of instrumentation would ever be needed at any time. It is conceivable
different parts of a binary could be compiled to use different instrumentation frameworks.

It is possible new frameworks will be added to support instrumenting functions. The command line and
attribute syntax should be expandable to meet those needs.
