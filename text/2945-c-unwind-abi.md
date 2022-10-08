- Feature Name: `"C-unwind" ABI`
- Start Date: 2019-04-03
- RFC PR: [rust-lang/rfcs#2945](https://github.com/rust-lang/rfcs/pull/2945)
- Rust Issue: [rust-lang/rust#74990](https://github.com/rust-lang/rust/issues/74990)
- Project group: [FFI-unwind][project-group]

[project-group]: https://github.com/rust-lang/project-ffi-unwind

# Summary
[summary]: #summary

We introduce a new ABI string, `"C-unwind"`, to enable unwinding from other
languages (such as C++) into Rust frames and from Rust into other languages.

Additionally, we define the behavior for a limited number of
previously-undefined cases when an unwind operation reaches a Rust function
boundary with a non-`"Rust"`, non-`"C-unwind"` ABI.

As part of this specification, we introduce the term ["Plain Old Frame"
(POF)][POF-definition]. These are frames that have no pending destructors and
can be trivially deallocated.

This RFC does not define the behavior of `catch_unwind` in a Rust frame being
unwound by a foreign exception. This is something the [project
group][project-group] would like to specify in a future RFC; as such, it is
"TBD" (see ["Unresolved questions"][unresolved-questions]).

# Motivation
[motivation]: #motivation

There are some Rust projects that need cross-language unwinding to provide
their desired functionality. One major example is Wasm interpreters, including
the Lucet and Wasmer projects.

There are also existing Rust crates (notably, wrappers around the `libpng` and
`libjpeg` C libraries) that `panic` across C frames. The safety of such
unwinding relies on compatibility between Rust's unwinding mechanism and the
native exception mechanisms in GCC, LLVM, and MSVC. Despite using a compatible
unwinding mechanism, the current `rustc` implementation assumes that `extern
"C"` functions cannot unwind, which permits LLVM to optimize with the
assumption that such unwinding constitutes undefined behavior.

The desire for this feature has been previously discussed on other RFCs,
including [#2699][rfc-2699] and [#2753][rfc-2753].

## Key design goals

As explained in [this Inside Rust blog post][inside-rust-requirements], we have
several requirements for any cross-language unwinding design.

The ["Analysis of key design goals"][analysis-of-design-goals] section analyzes
how well the current design satisfies these constraints.

* **Changing from `panic=unwind` to `panic=abort` cannot cause undefined
  behavior:** We wish to ensure that changing from `panic=unwind` to
  `panic=abort` never creates undefined behavior (relate to `panic=unwind`),
  even if one is relying on a library that triggers a panic or a foreign
  exception.
* **Optimization with `panic=abort`:** when using `panic=abort`, we
  wish to enable as many code-size optimizations as possible. This
  means that we shouldn't have to generate unwinding tables or other
  such constructs, at least in most cases.
* **Preserve the ability to change how Rust panics are propagated when
  using the Rust ABI:** Currently, Rust panics are propagated using
  the native unwinding mechanism, but we would like to keep the
  freedom to change that.
* **Enable Rust panics to traverse through foreign frames:** Several
  projects would like the ability to have Rust panics propagate
  through foreign frames.  Those frames may or may not register
  destructors of their own with the native unwinding mechanism.
* **Enable foreign exceptions to propagate through Rust frames:**
  Similarly, we would like to make it possible for C++ code (or other
  languages) to raise exceptions that will propagate through Rust
  frames "as if" they were Rust panics (i.e., running destructors or,
  in the case of `unwind=abort`, aborting the program).
* **Enable error handling with `longjmp`:**
  As mentioned above, some existing Rust libraries rely on the ability to
  `longjmp` across Rust frames to interoperate with Ruby, Lua, and other C
  APIs. The behavior of `longjmp` traversing Rust frames is not specified or
  guaranteed to be safe; in the current implementation of `rustc`,
  however, it [is safe][longjmp-pr]. On Windows, `longjmp` is implemented as a
  form of unwinding called ["forced unwinding"][forced-unwinding], so any
  specification of the behavior of forced unwinding across FFI boundaries
  should be forward-compatible with a [future RFC][unresolved-questions] that
  will provide a well-defined way to interoperate with longjmp-based APIs.
* **Do not change the ABI of functions in the `libc` crate:** Some `libc`
  functions may invoke `pthread_exit`, which uses [a form of
  unwinding][forced-unwinding] in the GNU libc implementation. Such functions
  must be safe to use with the existing `"C"` ABI, because changing the types
  of these functions would be a breaking change. 

[inside-rust-requirements]: https://blog.rust-lang.org/inside-rust/2020/02/27/ffi-unwind-design-meeting.html#requirements-for-any-cross-language-unwinding-specification
[longjmp-pr]: https://github.com/rust-lang/rust/pull/48572

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When declaring an external function that may unwind, such as an entrypoint to a
C++ library, use `extern "C-unwind"` instead of `extern "C"`:

```
extern "C-unwind" {
  fn may_throw();
}
```

Rust functions that call a possibly-unwinding external function should either
use the default Rust ABI (which can be made explicit with `extern "Rust"`) or
the `"C-unwind"` ABI:

```
extern "C-unwind" fn can_unwind() {
  may_throw();
}
```

Using the `"C-unwind"` ABI to "sandwich" Rust frames between frames from
another language (such as C++) allows an exception initiated in a callee frame
in the other language to traverse the intermediate Rust frames before being
caught in the caller frames. I.e., a C++ exception may be thrown,
cross into Rust via an `extern "C-unwind"` function declaration, safely unwind
the Rust frames, and cross back into C++ (where it may be caught) via a Rust
`"C-unwind"` function definition.

Conversely, languages that support the native unwinding mechanism, such as C++,
may be "sandwiched" between Rust frames, so that Rust `panic`s may safely
unwind the C++ frames, if the Rust code declares both the C++ entrypoint and
the Rust entrypoint using `"C-unwind"`.

## Other `unwind` ABI strings

Because the `C` ABI is not appropriate for all use cases, we also introduce
these `unwind` ABI strings, which will only differ from their non-`unwind`
variants by permitting unwinding, with the same semantics as `"C-unwind"`:

* `"system-unwind"` - available on all platforms
* `"stdcall-unwind"` and `"thiscall-unwind"` - available only on platforms
  where `"stdcall"` and `"thiscall"` are supported

More `unwind` variants of existing ABI strings may be introduced, with the same
semantics, without an additional RFC.

## "Plain Old Frames"
[POF-definition]: #plain-old-frames

A "POF", or "Plain Old Frame", is defined as a frame that can be trivially
deallocated: returning from or unwinding a POF cannot cause any
observable effects. This means that POFs do not contain any pending destructors
(live `Drop` objects) or `catch_unwind` calls.

The terminology is intentionally akin to [C++'s "Plain Old Data"
types][cpp-POD-definition], which are types that, among other requirements, are
trivially destructible (their destructors do not cause any observable effects,
and may be elided as an optimization).

Rust frames that do contain pending destructors or `catch_unwind` calls are
called non-POFs.

Note that a non-POF may _become_ a POF during execution of the corresponding
function, for instance if all `Drop` objects are moved out of scope, or if its
only `catch_unwind` call is in a code path that will not be executed. The next
section provides an example.

[cpp-POD-definition]: https://en.cppreference.com/w/cpp/named_req/PODType

## Forced unwinding
[forced-unwinding]: #forced-unwinding

This is a special kind of unwinding used to implement `longjmp` on Windows and
`pthread_exit` in `glibc`. A brief explanation is provided in [this Inside Rust
blog post][inside-rust-forced]. This RFC distinguishes forced unwinding from
other types of foreign unwinding.

Since language features and library functions implemented using forced
unwinding on some platforms use other mechanisms on other platforms, Rust code
cannot rely on forced unwinding to invoke destructors (calling `drop` on `Drop`
types). In other words, a forced unwind operation on one platform will simply
deallocate Rust frames without true unwinding on other platforms.

This RFC specifies that, regardless of the platform or the ABI string (`"C"` or
`"C-unwind"`), any platform features that may rely on forced unwinding will
always be considered undefined behavior if they cross
non-[POFs][POF-definition]. Crossing only POFs is necessary but not sufficient,
however, to make forced unwinding safe, and for now we do not specify any safe
form of forced unwinding; we will specify this in [a future
RFC][unresolved-questions].

[inside-rust-forced]: https://blog.rust-lang.org/inside-rust/2020/02/27/ffi-unwind-design-meeting.html#forced-unwinding

## Changes to the behavior of existing ABI strings
[extern-c-behavior]: #changes-to-extern-c-behavior

Prior to this RFC, any unwinding operation that crossed an `extern "C"`
boundary, either from a `panic!` "escaping" from a Rust function defined with
`extern "C"` or by entering Rust from another language via an entrypoint
declared with `extern "C"`, caused undefined behavior.

This RFC retains most of that undefined behavior, with one exception: with the
`panic=unwind` runtime, `panic!` will cause an `abort` if it would otherwise
"escape" from a function defined with `extern "C"`.

This change will be applied to all ABI strings other than `"Rust"`, such as
`"system"`.

## Interaction with `panic=abort`

If a non-forced foreign unwind would enter a Rust frame via an `extern
"C-unwind"` ABI boundary, but the Rust code is compiled with `panic=abort`, the
unwind will be caught and the process aborted.

Conversely, non-forced unwinding from another language into Rust through an FFI
entrypoint declared with `extern "C"` is always undefined behavior, and is not
guaranteed to cause the program to abort under `panic=abort`. As noted
[below][abi-boundaries-and-forced-unwinding], however, when compiling in debug
mode, the compiler may be able to guarantee an abort in this case.

`panic=abort` will have no impact on the behavior of forced unwinding.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## ABI boundaries and unforced unwinding
[abi-boundaries-and-forced-unwinding]: #abi-boundaries-and-forced-unwinding

This table shows the behavior of an unwinding operation reaching each type of
ABI boundary (function declaration or definition). "UB" stands for undefined
behavior. `"C"`-like ABIs are `"C"` itself but also related ABIs such as
`"system"`.

| panic runtime  | ABI          | `panic`-unwind                        | Unforced foreign unwind |
| -------------- | ------------ | ------------------------------------- | ----------------------- |
| `panic=unwind` | `"C-unwind"` | unwind                                | unwind                  |
| `panic=unwind` | `"C"`-like   | abort                                 | UB                      |
| `panic=abort`  | `"C-unwind"` | `panic!` aborts                       | abort                   |
| `panic=abort`  | `"C"`-like   | `panic!` aborts (no unwinding occurs) | UB                      |

In debug mode, the compiler could insert code to catch unwind attempts at
`extern "C"` boundaries and `abort`; this would provide a safe way to discover
(and fix) instances of this form of UB.

## Frame deallocation and forced unwinding

The interaction of Rust frames with C functions that deallocate frames (i.e.
functions that may use forced unwinding on specific platforms) is independent
of the panic runtime, ABI, or platform.

* **When deallocating Rust non-POFs:** this is explicitly undefined behavior.
* **When deallocating Rust [POFs][POF-definition]:** for now, this is not
  specified, and must be considered undefined behavior. However, we do plan to
  specify a safe way to deallocate POFs with `longjmp` or `pthread_exit` in [a
  future RFC][unresolved-questions].

## Additional limitations
[additional-limitations]: #additional-limitations

In order to limit the scope of this RFC, the following limitations are imposed:

* No subtype relationship is defined between functions or function pointers
  using different ABIs.
* Coercions are not defined between `"C"` and `"C-unwind"`.
* As noted in the [summary][summary], if a Rust frame containing a pending
  `catch_unwind` call is unwound by a foreign exception, the behavior is
  undefined for now.
* The behavior of asynchronous exceptions, such as SEH on Windows, interrupting
  Rust code is not defined.

These may be addressed in [future RFCs][future-possibilities].

# Drawbacks
[drawbacks]: #drawbacks

Forced unwinding is treated as universally unsafe across
[non-POFs][POF-definition], but on some platforms it could theoretically be
well-defined. As noted [above](forced-unwind), however, this would make the UB
inconsistent across platforms, which is not desirable.

This design imposes some burden on existing codebases (mentioned
[above][motivation]) to change their `extern` annotations to use the new ABI.

Having separate ABIs for `"C"` and `"C-unwind"` may make interface design more
difficult, especially since this RFC [postpones][unresolved-questions]
introducing coercions between function types using different ABIs. Conversely,
a single ABI that "just works" with C++ (or any other language that may throw
exceptions) would be simpler to learn and use than two separate ABIs.

This RFC preserves an existing inconsistency between the `"Rust"` ABI (which is
the default for all functions without an explicit ABI string) and the other
existing ABIs: no ABI string without the word `unwind` will permit unwinding,
except the `"Rust"` ABI, which will permit unwinding, but only when compiled
with `panic=unwind`. Making other ABIs consistent with the `"Rust"` ABI by
permitting them to unwind by default (and possibly either introducing a new
`"C-unwind"` ABI or an annotation akin to C++'s `noexcept` to explicitly
prohibit unwinding) would also be a safer default, since it would prevent
undefined behavior when interfacing with external libraries that may throw
exceptions.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Other proposals discussed with the lang team
[alternatives]: #other-proposals-discussed-with-the-lang-team

Two other potential designs have been discussed in depth; they are
explained in [this Inside Rust blog post][inside-rust-proposals]. The design in this
RFC is referred to as "option 2" in that post.

"Option 1" in that blog post only differs from the current proposal in the
behavior of a forced unwind across a `"C-unwind"` boundary under `panic=abort`.
Under the current proposal, this type of unwind is permitted, allowing
`longjmp` and `pthread_exit` to behave "normally" with both the `"C"` and the
`"C-unwind"` ABI across all platforms regardless of panic runtime. If
[non-POFs][POF-definition] are unwound, this results in undefined behavior.
Under "option 1", however, all foreign unwinding, forced or unforced, is caught
at `"C-unwind"` boundaries under `panic=abort`, and the process is aborted.
This gives `longjmp` and `pthread_exit` surprising behavior on some platforms,
but avoids that cause of undefined behavior in the current proposal.

The other proposal in the blog post, "option 3", is dramatically different. In
that proposal, foreign exceptions are permitted to cross `extern "C"`
boundaries, and no new ABI is introduced.

[inside-rust-proposals]: https://blog.rust-lang.org/inside-rust/2020/02/27/ffi-unwind-design-meeting.html#three-specific-proposals

## Reasons for the current proposal
[rationale]: #reasons-for-the-current-proposal

Our reasons for preferring the current proposal are:

* Introducing a new ABI makes reliance on cross-language exception handling
  more explicit.
* `panic=abort` can be safely used with `extern "C-unwind"` (there is no
  undefined behavior except with improperly used forced unwinding), but `extern
  "C"` has more optimization potential (eliding landing pads). Having two ABIs
  puts this choice in the hands of users.
  * The single-ABI proposal ("option 3") causes any foreign exception entering
    Rust to have undefined behavior under `panic=abort`, whereas the current
    proposal does not permit the `panic=abort` runtime to introduce undefined
    behavior to a program that is well-defined under `panic=unwind`.
  * This optimization could be made available with a single ABI by means of a
    function attribute indicating that a function cannot unwind (similar to C++'s
    `noexcept`). Such attributes [are already available in nightly
    Rust][nightly-attributes]. However, Rust does not yet support attributes
    for function pointers, so until that feature is added, there would be no
    way to indicate whether function pointers unwind using an attribute.
* This design has simpler forward compatibility with alternate `panic!`
  implementations. Any well-defined cross-language unwinding will require shims
  to translate between the Rust unwinding mechanism and the natively provided
  mechanism. In this proposal, only `"C-unwind"` boundaries would require shims.

## Analysis of key design goals
[analysis-of-design-goals]: #analysis-of-design-goals

This section revisits the key design goals to assess how well they
are met by the proposed design.

### Changing from `panic=unwind` to `panic=abort` cannot cause UB

This constraint is met:

* Unwinding across a "C" boundary is UB regardless
    of whether one is using `panic=unwind` or `panic=abort`.
* Unwinding across a "C-unwind" boundary is always defined,
    though it is defined to abort if `panic=abort` is used.
* Forced exceptions behave the same regardless of panic mode.

### Optimization with panic=abort

Using this proposal, the compiler is **almost always** able to reduce
overhead related to unwinding when using panic=abort. The one
exception is that invoking a "C-unwind" ABI still requires some kind
of minimal landing pad to trigger an abort. The expectation is that
very few functions will use the "C-unwind" boundary unless they truly
intend to unwind -- and, in that case, those functions are likely
using panic=unwind anyway, so this is not expected to make much
difference in practice.

### Preserve the ability to change how Rust panics are propagated when using the Rust ABI

This constraint is met. If we were to change Rust panics to a
different mechanism from the mechanism used by the native ABI,
however, there would have to be a conversion step that interconverts
between Rust panics and foreign exceptions at "C-unwind" ABI
boundaries.

### Enable Rust panics to traverse through foreign frames

This constraint is met.

### Enable foreign exceptions to propagate through Rust frame

This constraint is partially met: the behavior of foreign exceptions
with respect to `catch_unwind` is currently undefined, and left for
future work.

### Enable error handling with `longjmp`

This constraint has been [deferred][unresolved-questions].

### Do not change the ABI of functions in the `libc` crate

This constraint has been [deferred][unresolved-questions].

# Prior art
[prior-art]: #prior-art

C++ as specified has no concept of "foreign" exceptions or of an underlying
exception mechanism. However, in practice, the C++ exception mechanism is the
"native" unwinding mechanism used by compilers.

On Microsoft platforms, when using MSVC, unwinding is always supported for both
C++ and C code; this is very similar to "option 3" described in [the
inside-rust post][inside-rust-proposals] mentioned [above][alternatives].

On other platforms, GCC, LLVM, and any related compilers provide a flag,
`-fexceptions`, for explicitly ensuring that stack frames have unwinding
support regardless of the language being compiled. Conversely,
`-fno-exceptions` removes unwinding support even from C++. This is somewhat
similar to how Rust's `panic=unwind` and `panic=abort` work for `panic!`
unwinds, and under the "option 3" proposal, the behavior would be similar for
foreign exceptions as well. In the current proposal, though, such foreign
exception support is not enabled by default with `panic=unwind` but requires
the new `"C-unwind"` ABI.

## Attributes on nightly Rust and prior RFCs
[nightly-attributes]: #attributes-on-nightly-rust-and-prior-rfcs

Currently, nightly Rust provides attributes, `#[unwind(allowed)]` and
`#[unwind(abort)]`, that permit users to select a well-defined behavior when a
`panic` reaches an `extern "C"` function boundary. Stabilization of these
attributes has [a tracking issue][attributes-tracking-issue], but most
of the discussion about whether this was the best approach took place in two
RFC PR threads, [#2699][rfc-2699] and [#2753][rfc-2753].

The attribute approach was deemed insufficient for the following reasons:

* Currently, Rust does not support attributes on function pointers. This may
  change in the future, but until then, attributes cannot provide any way to
  differentiate function pointers that may unwind from those that are
  guaranteed not to. Assuming that no function pointers may unwind is not
  viable, because that severely limits the utility of cross-FFI unwinding.
  Conversely, assuming that all `extern "C"` function pointers may unwind is
  inconsistent with the no-unwind default for `extern "C"` functions.
* The existence of a compatible unwind mechanism on both sides of a function
  invocation boundary is part of the binary interface for that invocation, so
  the ABI string is a more appropriate part of the language syntax than
  function attributes to indicate that unwinding may occur.
* The ability of a function to unwind must be part of the type system to ensure
  that callers that cannot unwind don't invoke functions that can unwind.
  Although attributes are sometimes part of a function's type, a function's ABI
  string is always part of its type, so we are not introducing any new elements
  to the type system.

[attributes-tracking-issue]: https://github.com/rust-lang/rust/issues/58760
[rfc-2699]: https://github.com/rust-lang/rfcs/pull/2699
[rfc-2753]: https://github.com/rust-lang/rfcs/pull/2753

## Older discussions about unwinding through `extern "C"` boundaries

As mentioned [above][motivation], it is currently undefined behavior for
`extern "C"` functions to unwind. As documented in [this
issue][abort-unwind-issue], the lang team has long intended to make `panic!`
cause the runtime to abort rather than unwind through an `extern "C"` boundary
(which the current proposal [also specifies][extern-c-behavior]).

The abort-on-unwind behavior was [stabilized in 1.24][1.24-release] and
[reverted in 1.24.1][1.24.1-release]; the team originally planned to [stabilize
it again][1.33-stabilization] in 1.33, but ultimately [decided not
to][1.33-discussion]. Community discussion [on discourse][discourse-thread] was
largely concerned with the lack of any stable language feature to permit
unwinding across FFI boundaries, and this contributed to the decision to block
the re-stabilization of the abort-on-unwind behavior until such a feature could
be introduced.

[abort-unwind-issue]: https://github.com/rust-lang/rust/issues/52652
[1.24-release]: https://blog.rust-lang.org/2018/02/15/Rust-1.24.html#other-good-stuff
[1.24.1-release]: https://blog.rust-lang.org/2018/03/01/Rust-1.24.1.html#do-not-abort-when-unwinding-through-ffi
[1.33-stabilization]: https://github.com/rust-lang/rust/pull/55982
[1.33-discussion]: https://github.com/rust-lang/rust/issues/58794
[discourse-thread]: https://internals.rust-lang.org/t/unwinding-through-ffi-after-rust-1-33/9521?u=batmanaod

# Unresolved questions
[unresolved-questions]: #unresolved-questions

The behavior of `catch_unwind` when a foreign exception encounters it is
currently [left undefined][reference-level-explanation]. We would like to
provide a well-defined behavior for this case, which will probably be either to
let the exception pass through uncaught or to catch some or all foreign
exceptions.

We would also like to specify conditions under which `longjmp` and
`pthread_exit` may safely deallocate Rust frames. This RFC specifies that
frames deallocated in this way [must be POFs][reference-level-explanation].
However, this condition is merely necessary rather than sufficient to ensure
well-defined behavior.

Within the context of this RFC and in discussions among members of the
[FFI-unwind project group][project-group], this class of formally-undefined
behavior which we plan to define in future RFCs is referred to as "TBD
behavior".

# Future possibilities
[future-possibilities]: #future-possibilities

The [FFI-unwind project group][project-group] intends to remain active at least
until all ["TBD behavior"][unresolved-questions] is defined. We may also
address some or all of the current proposal's
[limitations][additional-limitations] in future RFCs.

We may want to provide more means of interaction with foreign exceptions. For
instance, it may be possible to provide a way for Rust to catch C++ exceptions
and rethrow them from another thread. Such a mechanism may either be
incorporated into the functionality of `catch_unwind` or provided as a separate
language or standard library feature.

Coercions between `"C-unwind"` function types (such as function pointers) and
the other ABIs are not part of this RFC. However, they will probably be
indispensable for API design, so we plan to provide them in a future RFC.

As mentioned [above][rationale], shims will be required if Rust changes its
unwind mechanism.
