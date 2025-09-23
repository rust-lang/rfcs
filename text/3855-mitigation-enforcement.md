- Feature Name: `mitigation_enforcement`
- Start Date: 2025-09-13
- RFC PR: [rust-lang/rfcs#3855](https://github.com/rust-lang/rfcs/pull/3855)
- Rust Issue: None

# Summary
[summary]: #summary

Introduce the concept of "mitigation enforcement", so that when compiling
a crate with mitigations enabled (for example, `-C stack-protector`),
a compilation error will happen if the produced artifact would contain Rust
code without the same mitigations enabled.

This in many cases would require use of `-Z build-std`, since the standard
library only comes with a single set of enabled mitigations per target.

Mitigation enforcement should be disableable by the end-user via a compiler
flag.

# Motivation
[motivation]: #motivation

Memory unsafety mitigations are important for reducing the chance that a vulnerability
ends up being exploitable.

While in Rust, memory unsafety is less of a concern than in C, mitigations are
still important for several reasons:

1. Some mitigations (for example, straight line speculation mitigation,
   [`-Z harden-sls`]) mitigate the impact of Spectre-style speculative
   execution vulnerabilities, that exist in Rust just as well as C.
2. Many Rust programs also contain large C/C++ components, that can have
   memory vulnerabilities.
3. Many Rust programs use `unsafe`, that can introduce memory unsafety
   and vulnerabilities.

Mitigations are generally enabled by passing a flag to the compiler (for
example, [`-Z harden-sls`] or [`-Z stack-protector`]). If the compilation
process of a program is complex, it is very easy to end up accidentally
not passing the flag to one of the constituent object files.

This can have one of several consequences:

1. In some cases (for example `-Z fixed-x18 -Z sanitizer=shadow-call-stack`),
   the mitigation changes the ABI, and linking together code with different
   mitigation settings leads to undefined behavior such as crashes even
   in the absence of an attack. In these cases, the sanitizer should be a
   [target modifier] rather than using this RFC.
2. For "Spectre-type" mitigations (e.g. `harden-sls`), if there is some reachable
   code in your address space without a retpoline, attackers can execute a
   Spectre attack, even if there is 0 UB in your code.
3. For "CFI-type" mitigations (e.g. kcfi), if there is reachable code in your
   address space that does not have that sanitizer enabled, attackers can use it to
   leverage an already-existing memory vulnerability into ROP execution, even
   if the memory vulnerability is in a completely different part of the code than
   the part that has the mitigation disabled
4. For "local" mitigations (e.g. stack protector, or C's `-fwrapv` - which I don't think
   Rust has), the mitigation protects the code when it is in the right place 
   relative to the bug - a stack protector helps basically when it protects the buffer
   that overflows, and it does not matter which other functions have a stack protector.

To avoid these consequences, teams that write software with high security needs - for
example, browsers and the Linux kernel - need to have a way to make sure that the
programs they produce have the mitigations they want enabled.

On the other hand, for teams that write software in a more messy environment, it
can be hard to chase down all dependencies, and especially for "local" mitigations,
being able to enable them on an object-by-object basis is the only thing that allows
for the mitigations to actually be deployed. Especially important is progressive
deployment - it's much easier to introduce mitigations 1 crate at a time than
to introduce mitigations a whole program at a time, even if the end goal is
to introduce the mitigations to the entire program.

[target modifier]: https://github.com/rust-lang/rfcs/pull/3716
[`-Z harden-sls`]: https://github.com/rust-lang/compiler-team/issues/869
[`-Z stack-protector`]: https://github.com/rust-lang/rust/issues/114903
[example by Alice Ryhl]: https://rust-lang.zulipchat.com/#narrow/channel/131828-t-compiler/topic/Target.20modifiers.20and.20-Cunsafe-allow-abi-mismatch/near/483871803

## Supported mitigations

The following mitigations could find this feature interesting

### Already Stable

1. `-C control-flow-guard`
   This is a "CFI-type" mitigation on Windows, and therefore having it enabled
   only partially makes it far less protective.

   However, it is already stable, and we would need a `-C control-flow-guard-enforce`
   or `-C deny-partial-mitigations=control-flow-guard` (or bikeshed) to make
   it enforcing.

   We could also make it enforcing over an edition boundary.
2. `-C relocation-model`
   If position-dependent code is compiled into a binary, then ASLR will
   not be able to be enabled.

   This is less of a problem in Rust than in C, since position-independent
   code is a rarely-changed default in Rust, and it can easily be checked
   via [`hardening-check(1)`] or similar tools since it's easily visible
   in the ELF header.

   However, we might still want to introduce a `-C enforce-position-independent` or
   `-C deny-partial-mitigations=position-independent`.

   As far as I can tell, there is no way to disable `relro` via stable Rust
   compilation flags.
3. `-C overflow-checks`
   This can be thought of as a mitigation in some sense. It might make sense
   to add a way to make it enforcing, but I don't think it makes sense to
   make it enforcing by default since that is contrary to the normal
   use of turning overflow checks only for the crate under development.

   However, we might still want to introduce a `-C enforce-overflow-checks` or
   `-C deny-partial-mitigations=overflow-checks`. It probably does not
   make sense to make it enforcing over an edition boundary, since the
   desired default there is not to enforce.

### Currently Unstable (as of rustc 1.89)

This RFC is not the place to make a decision of exactly which unstable mitigations
should have enforcement enabled - that should take place as a part of their
stabilization.

However, it would be good to see that enforcement fits well with sanitizers.

1. [`-Z branch-protection`]/[`-Z cf-protection`] - control flow
   protection in ARM or Intel, respectively. Would probably want this.
   It uses [`.note.gnu.property`](#notegnuproperty-1) which makes it active only
   if every object in the address space uses it, which makes it easy to detect via a
   [`hardening-check(1)`]-style tool, but since it is not the default, this
   would make it easier to make sure it is enabled.
2. `-Z ehcont-guard` - I couldn't find documentation of that (Windows) feature,
   but it looks relevant.
2. [`-Z indirect-branch-cs-prefix`] - a part of retpoline (Spectre)
   mitigation on x86.
3. [`-Z no-jump-tables`] - CFI-type mitigation?
   soon to be stabilized as [`-C jump-tables`]. Do we want to hold that
   stabilization as well?
3. [`-Z retpoline`] and `-Z retpoline-external-thunk` - Spectre-type mitigation
4. [`-Z sanitizer`]-based mitigations. A fairly large use case.
   As far as I can tell, enforcement makes sense for
   `-Zsanitizer=cfi, -Zsanitizer=memtag, -Zsanitizer=shadow-call-stack`
   and does not make sense for
   `-Zsanitizer=address, -Zsanitizer=dataflow, -Zsanitizer=hwaddress, -Zsanitizer=leak, -Zsanitizer=memory, -Zsanitizer=thread`
4. [`-Z stack-protector`] - stack smashing protection, local-type mitigation
5. [`-Z ub-checks`]: as far as I can tell, it is not intended as a mitigation,
   but since it prevents some UB, it might be thought of as one

[`-Z branch-protection`]: https://github.com/rust-lang/rust/issues/113369
[`-Z cf-protection`]: https://github.com/rust-lang/rust/issues/93754
[`-Z indirect-branch-cs-prefix`]: https://github.com/rust-lang/rust/issues/116852
[`-Z no-jump-tables`]: https://github.com/rust-lang/rust/issues/116592
[`-C jump-tables`]: https://github.com/rust-lang/rust/pull/145974
[`-Z retpoline`]: https://github.com/rust-lang/rust/issues/116852
[`-Z sanitizer`]: https://github.com/rust-lang/rust/issues/89653
[`-Z stack-protector`]: https://github.com/rust-lang/rust/issues/114903
[`-Z ub-checks`]: https://github.com/rust-lang/rust/issues/123499

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When you use a mitigation, such as `-C stack-protector=strong`, if one of your
dependencies does not have that mitigation enabled, compilation will fail.

> Error: your program uses the crate `foo`, that is not protected by
> `-C stack-protector=strong`.
>
> Recompile that crate with the mitigation enabled, or use
> `-C stack-protector=strong-noenforce` to allow creating an artifact
> that has the mitigation only partially enabled.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Every flag value that enables a mitigation for which enforcement is desired is split
into 2 separate values, "enforcing" and "non-enforcing" mode. The enforcing mode
is the default, non-enforcing mode is constructed by adding `-noenforce` to the
name of the value, for example `-C stack-protector=strong-noenforce` or
`-C sanitizer=shadow-call-stack-noenforce`.

> It is possible to bikeshed the exact naming scheme.

> Every new mitigation would need to decide whether it adopts this scheme,
> but mitigations are expected to adopt it.

Every crate gets a metadata field that contains the set of mitigations it has enabled.

When compiling a crate, if the current crate has a mitigation
with enforcement turned on, and one of the dependencies does not
have that mitigation turned on (whether enforcing or not), a
compilation error results.

If a mitigation has multiple "levels", a stricter level at a dependency is
compatible with a looser level at the current (dependent) crate, but not
vice-versa - for example, if the standard library crates were compiled with
`-C stack-protector=all` (not discusser whether that is a wise idea), they would be
compatible with every configuration of user crates.

The error happens independent of the target crate type (you get an error
if you are building an rlib, not just the final executable).

For example, with `-C stack-protector`, the compatibility table will be
as follows:

|  Dependency\Current | none | none-noenforce | strong |     strong-noenforce     |  all  |      all-noenforce       |
| ------------------- | ---- | -------------- | ------ | ------------------------ | ----- |   --------------------   |
| none                |  OK  |      OK        | error  | OK - dependent noenforce | error | OK - dependent noenforce |
| none-noenforce      |  OK  |      OK        | error  | OK - dependent noenforce | error | OK - dependent noenforce |
| strong              |  OK  |      OK        |   OK   |            OK            | error | OK - dependent noenforce |
| strong-noenforce    |  OK  |      OK        |   OK   |            OK            | error | OK - dependent noenforce |
| all                 |  OK  |      OK        |   OK   |            OK            |   OK  |             OK           |
| all-noenforce       |  OK  |      OK        |   OK   |            OK            |   OK  |             OK           |

If a program has multiple flags of the same kind, the last flag wins, so e.g.
`-C stack-protector=strong-noenforce -C stack-protector=strong` is the same as
`-C stack-protector=strong`.

# Drawbacks
[drawbacks]: #drawbacks

The `-noenforce` syntax is ugly, and the
`-C allow-partial-mitigations=stack-protector` syntax is either order-dependent
or does not allow for easy appending.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Syntax alternatives

### -C stack-protector=none-noenforce

The option `-C stack-protector=none-noenforce` is the same as
`-C stack-protector=none`. I am not sure whether we should have both, but
it feels that orthogonality is in favor of having both.

### -C allow-partial-mitigations

Instead of having `-C stack-protector=strong-noenforce`, we could have the
syntax be `-C stack-protector=strong -C allow-partial-mitigations=stack-protector`.

Some people feel that syntax is prettier. In that case, we have 2 options:

#### Without order dependency

This is the simplest to implement. With that,
`-C stack-protector=strong -C allow-partial-mitigations=stack-protector -C stack-protector=strong`
is the same as `-C stack-protector=strong -C allow-partial-mitigations=stack-protector`.

This is unfortunate, because `-C stack-protector=strong -C allow-partial-mitigations=stack-protector` is
a pretty good default for distributions to set. If a distribution sets that, and an application
believes they are turning on enforcing stack protection by using `-C stack-protector=strong`,
the application will not be getting enforcement due to the distribution setting
`-C allow-partial-mitigations=stack-protector`.

On the other hand, maybe there is not actually desire to add
`-C stack-protector=strong -C allow-partial-mitigations=stack-protector` as a default?

Maybe it is actually possible to ship a `-C stack-protector=strong` standard library and
add a `-C stack-protector=strong` default, since the enforcement check only works
"towards roots"?

#### With order dependency

With a small amount of implementation effort, we could have `-C stack-protector=strong` reset the
`-C allow-partial-mitigations=stack-protector` state, so that
`-C stack-protector=strong -C allow-partial-mitigations=stack-protector -C stack-protector=strong`
is equivalent to `-C stack-protector=strong`.

This would work quite well, but I am not sure that rustc wants to have order between different
kinds of CLI arguments.

## Limiting the set of crates that are allowed to bypass enforcement

You could have a syntax like `-C stack-protector=strong-noenforce=std+alloc+core` or
`-C allow-partial-mitigations=stack-protector=std+alloc+core`,
or some other syntax (using `+` since `,` should have a different level of precedence),
which would only allow the mitigation to be partial on a specified set of crate names.

This is different from `-C pretend-mitigation-enabled`, since it reflects a decision
made by the application writer (dependent crate) rather than the library writer.

This can be done in a later stabilization that the core of the feature.

### Impacts to syntax choices

The `-C stack-protector=strong-noenforce=std+alloc+core` syntax feels ugly.

If we decide that the noenforce syntax is important to allow for flag concatenation, we can
certainly have both the `noenforce` syntax and the `allow-partial-mitigations` syntax,
with `noenforce` disabling enforcement for all crates while `allow-partial-mitigations`
disables it only for specific crates.

## Interaction with `-C unsafe-allow-abi-mismatch` / `-C pretend-mitigation-enabled`

The proposed rules do not interact with `-C unsafe-allow-abi-mismatch` at all, so if
you have a "sanitizer runtime" crate that is compiled with the following options:

> -C no-fixed-x18 -C sanitizer=shadow-call-stack=off -C unsafe-allow-abi-mismatch=fixed-x18 -C unsafe-allow-abi-mismatch=shadow-call-stack

Then dependencies will need to use it via `-C sanitizer=shadow-call-stack-noenforce`
rather than `-C sanitizer=shadow-call-stack`, otherwise they will get an error.

As far as I can need, there is no current demand for that sort of sanitizer runtime,
but if that is desired, it might be a good idea to add a
`-C pretend-mitigation-enabled=shadow-call-stack`, and possibly to make
`-C unsafe-allow-abi-mismatch` do that for crates that are target modifiers.

## Defaults

We want that the most obvious way to enable mitigations (e.g.
`-C stack-protector=strong` or `-C sanitizer=shadow-call-stack`) to turn on
enforcement, since that will set people up to a pit of success where mitigations
are enabled throughout.

However, we do want an easy way for distribution owners (for example,
Ubuntu) to turn on mitigations in a non-enforcing way, as is done
today e.g. [by Ubuntu with `-fstack-protector-strong`]. Distributions can't easily
add a new mitigation in an enforcing way, as that will cause widespread
breakage, but they can fairly easily turn a mitigation on in a non-enforcing way.

We do want the combination of defaults to combine in a nice way - if the
distributioon sets `-C stack-protector=strong-noenforce`, and the user adds
`-C stack-protector=strong`, we want the result to be stack-protector set
to strong and enforcing.

On the other hand, maybe there is not actually desire to add
`-C stack-protector=strong -C allow-partial-mitigations=stack-protector` as a default,
which would make this less interesting?

Maybe it is actually possible to ship a `-C stack-protector=strong` standard library and
add a `-C stack-protector=strong` default, since the enforcement check only works
"towards roots"?

[by Ubuntu with `-fstack-protector-strong`]: https://wiki.ubuntu.com/ToolChain/CompilerFlags

## The standard library

One big place where it's very easy to end up with mixed mitigations is the
standard library. The standard library comes compiled with just a single
set of mitigations enabled (as of Rust 1.88: none), and without `-Z build-std`,
it is only possible to use the mitigation settings in the shipped standard
library.

If we find out that some mitigations have a positive cost-benefit ratio
for the standard library (probably at least [`-Z stack-protector`]), we
probably want to ship a standard library supporting them by default, but
in a way that still allows people to compile code without mitigations,
if that fulfills their cost/benefit ratios better.

## Why not target modifiers?

The [target modifier] feature provides a similar goal of preventing mismatches in compiler
settings.

There are several issues with using target modifiers for mitigations:

### The name unsafe-allow-abi-mismatch

The name of the flag that allows mixing target modifiers, `-C unsafe-allow-abi-mismatch`,
does not make sense for cases that are not "unsafe ABI mismatches". It also uses the
word "unsafe", which we prefer not to use except in cases that can result in actual
unsoundness.

### The behavior of unsafe-allow-abi-mismatch

The behavior of `-C unsafe-allow-abi-mismatch` is also not ideal for mitigations.

The flag marks a crate as basically having a "wildcard target modifier", which allows it
to compile with crates with any value of the target modifier.

This is quite good for the original use case - it's an "I know what I am doing" flag
that allows "runtime" crates to be mixed in even if they subtly play with the
ABI rules - for example, in a kernel, where floating point execution is mostly
forbidden, there are a few compilation units using real floats. To call into them, first
you call some special functions that make the floating point registers usable, and then
you call into the CU safely by not having any floats in the signature of the function on
the boundary ([example by Alice Ryhl]).

However, for mitigations, the expected case for disabling mitigations is less people
knowing what they are doing, and more people that don't agree with the performance/security
tradeoff they bring. In that case, we should allow the executable-writer to be aware
of the tradeoff being made, rather than letting libraries in the middle decide it
for them.

## Why not an external tool?

This is somewhat hard to do with an external tool, since there is
no way of looking at a binary and telling what mitigations its components
have.

There are howevever some external tools that do check for mitigations,
but they have limitations:

1. [`hardening-check(1)`] exists, but its check for stack smashing protection only
   checks that at least 1 function has stack cookies, rather than checking that
   every interesting function has it enabled.
2. The Linux kernel has [`objtool`], which checks for some other mitigations (for
   example, retpolines). It however needs to access the `.o` object files
   rather than to the final linked executable or shared library - which
   requires its user to control the linking process - and also has hardcoded
   limitations that make it only suitable for the Linux kernel, rather than
   being useful as a general-purpose tool.
   
[`hardening-check(1)`]: https://manpages.debian.org/testing/devscripts/hardening-check.1.en.html
[`objtool`]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/tools/objtool/Documentation/objtool.txt?id=5cd64d4f92683afa691a6b83dcad5adfb2165ed0

## .note.gnu.property

The `.note.gnu.property` field contains a number of properties
(for example, [`GNU_PROPERTY_AARCH64_FEATURE_1_BTI`]) that are used to indicate
that the compiled code contains certain mitigations, for example BTI
(`-Zbranch-protection=bti`).

When linking multiple objects, the linker sets the resulting property to be the
logical AND of the properties of the constituent objects.

For protections such as BTI, the mitigation can only be turned on if all code
within the compiled binary supports it - if one of the object files doesn't,
the loader has to leave the mitigation turned off entirely. The ELF loader uses
the value of the property within the loaded executable to decide whether
to turn on the mitigation.

If it could be arranged, using `.note.gnu.property` could allow mitigation tracking
to propagate across languages - with the final compilation step intentionally erroring
out if the property is not enabled. However, this is also a disadvantage - adding a
new property to `.note.gnu.property` requires cooperation from the target owners.

Therefore, it might be useful as a future step with cooperation from the target owners,
but is not good if we want to be able to add new enforced mitigations without requiring
cooperation from all platforms.

[`GNU_PROPERTY_AARCH64_FEATURE_1_BTI`]: https://docs.rs/object/0.37/object/elf/constant.GNU_PROPERTY_AARCH64_FEATURE_1_BTI.html

# Prior art
[prior-art]: #prior-art

## The panic strategy

The Rust compiler already *has* infrastructure to detect flag mismatches: the
flags `-Cpanic` and `-Zpanic-in-drop`. The prebuilt stdlib comes with different
pieces depending on which strategy is used, although panic landing flags are
not entirely removed when using `-Cpanic=abort`, as only part of the prebuilt
stdlib is switched out.

## Target modifiers

## .note.gnu.property

The `.note.gnu.property` section discussed previously is an example of C code
detecting mismatches of a flag at link time.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities

A possible future extension could be to provide a mechanism to enforce
mitigations across C code and Rust code. This would be an interesting
extension, but it would require cross-language effort that will
take a long period of time to finish. Similarly, another possible
future extension could be to catch mitigation mismatches when using
dynamic linking.
