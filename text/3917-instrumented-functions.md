- Feature Name: instrument-functions
- Start Date: 2026-02-05
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/3917)
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
to inspect or manipulate arguments as well as the traditional mcount features.
* [XRay](https://llvm.org/docs/XRay.html), an LLVM project to instrument both entry and exit of functions,
with dynamic enablement.

mcount deserves a little extra background. This feature has existed on many C toolchains for decades, and
gcc/clang have developed extensions to support novel features like Linux's [ftrace](https://docs.kernel.org/trace/ftrace.html).
This support was further refined into the fentry feature available on s390x and x86. x86 actively uses
fentry, and there is no interest to convert to patchable entries.


## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Today, Rust supports disparate experimental options to enable function instrumentation using XRay and mcount,
possibly simultaneously.

This RFC proposes a unified set of options to enable one instrumentation framework, and provide sufficient
language integration to control individual function instrumentation.

Function instrumentation is generally limited to inserting a counting function into the entry and exit
of each function. A counting function is one which takes the caller's and callee's address as arguments
and performs some action. The action could be logging, tracing, intercepting, rerouting a function
call, or nothing.

For each target, a counting function is inserted into a specific part of a function's entry or exit. For
targets which support gprof, a specific target defined counting function is expected to be called. For
example, on x86_64-linux `__fentry__` or `mcount` is the expected counting function. The ABI of these
functions is expected to be stable despite their lack of documentation (e.g., glibc provides these symbols).

gcc and clang provide the following options which are used to instrument functions for common usages:
* ftrace: `-pg` or `-pg -fentry` with `-mrecord-mcount` and optionally `-mnop-mcount`.
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

The options `-Zinstrument-mcount` and `-Zinstrument-xray` shall be removed. The user configurable
options shall be specified in a comma-separated list following the framework option, separated by
a colon.

`-Zinstrument-function=mcount`:
  * No options are provided.

`-Zinstrument-function=fentry`:
  * No options are provided.

`-Zinstrument-function=xray`:
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
$ RUSTFLAGS="-Zinstrument-function=mcount" cargo build

$ RUSTFLAGS="-Zinstrument-function=fentry" cargo build

$ RUSTFLAGS="-Zinstrument-function=xray:ignore-loops" cargo build
```

### Language additions

A single builtin attribute, `instrument_fn`, will be added. It will be applied to functions and methods
only. The attribute will accept an "on" or "off" option as `#[instrument_fn = "on|off"]`.

```rust
#[instrument_fn = "on"]
fn always_instrument() {
}

#[instrument_fn = "off"]
fn never_instrument() {
}
```

For XRay, "on" and "off" are equivalent to always and never instrumenting. For mcount and fentry,
"on" has no effect.

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

Similar features exist in gcc and clang as noted above. Likewise, gcc and clang provide some targets
with additional options to record or write nops instead of function calls. However, the utility of
these extensions is limited as the kernel build system contains tooling to implement these features
outside of gcc/clang.

patchable-function-entries also serve a similar role. They have mostly replaced mcount/fentry in
Linux, excepting x86. On x86, patchable entries are used primarily for mitigation of exploits and
control flow integrity, and fentry for tracing. This gives the ability to toggle tracing support
without interfering with mitigation.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- Can we stabilize the mcount portions of this RFC? The underlying counting functions have target specific
  ABI which is mostly undocumented, but has historically been stable.

## Future possibilities
[future-possibilities]: #future-possibilities

It might be desirable to provide finer control over instrumentation. This could be done by extending the
attribute with options, e.g.: `#[instrument_fn(xray="off", mcount="on")]`. Similarly, this extension could
be used to supply individual instrumentation options to a function.
