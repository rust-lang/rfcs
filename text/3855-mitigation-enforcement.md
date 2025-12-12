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
   memory vulnerabilities. The exploitation of these vulnerabilities
   is made easier by the presence of mitigation-less Rust code within
   the same address-space.
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

   However, it is already stable, and we would need a `-C deny-partial-mitigations=control-flow-guard`
   or `-C control-flow-guard-enforce` (or bikeshed) to make
   it enforcing.

   We could also make it enforcing over an edition boundary.
2. `-C relocation-model`
   If position-dependent code is compiled into a binary, then ASLR will
   not be able to be enabled.

   This is less of a problem in Rust than in C, since position-independent
   code is a rarely-changed default in Rust, and it can easily be checked
   via [`hardening-check(1)`] or similar tools since it's easily visible
   in the ELF header.

   However, we might still want to introduce a `-C deny-partial-mitigations=position-independent` or
   `-C enforce-position-independent`.

   As far as I can tell, there is no way to disable `relro` via stable Rust
   compilation flags.
3. `-C overflow-checks`
   This can be thought of as a mitigation in some sense. It might make sense
   to add a way to make it enforcing, but I don't think it makes sense to
   make it enforcing by default since that is contrary to the normal
   use of turning overflow checks only for the crate under development.

   However, we might still want to introduce a `-C deny-partial-mitigations=overflow-checks` or
   `-C enforce-overflow-checks`. It probably does not
   make sense to make it enforcing over an edition boundary, since the
   desired default there is not to enforce. This probably merits a separate
   RFC/FCP.

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
2. [`-Z ehcont-guard`] - Similar for [`-Z branch-protection`]/[`-Z cf-protection`],
   but for Windows exception handlers - if every object in the address space
   uses it, `NtContinue` is only allowed to jump to valid exception handlers.
   This means it is easy to detect via a [`hardening-check(1)`]-style tool, but
   it probably makes sense to include it.
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
   `-Zsanitizer=address, -Zsanitizer=dataflow, -Zsanitizer=hwaddress, -Zsanitizer=leak, -Zsanitizer=memory, -Zsanitizer=thread` (`-Z sanitizer=address` should probably be a [target modifier])
4. [`-Z stack-protector`] - stack smashing protection, local-type mitigation
5. [`-Z ub-checks`]: as far as I can tell, it is not intended as a mitigation,
   but since it prevents some UB, it might be thought of as one

[`-Z branch-protection`]: https://github.com/rust-lang/rust/issues/113369
[`-Z cf-protection`]: https://github.com/rust-lang/rust/issues/93754
[`-Z ehcont-guard`]: https://learn.microsoft.com/en-us/cpp/build/reference/guard-enable-eh-continuation-metadata?view=msvc-170
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
> `-C allow-partial-mitigations=stack-protector` to allow creating an artifact
> that has the mitigation only partially enabled.
>
> It is possible to disable `-C allow-partial-mitigations=stack-protector` via
> `-C deny-partial-mitigations=stack-protector`.

Other flags that can be mitigations, for example `-C overflow-checks=on`,
permit partial mitigations by default, but it is possible to make sure
your dependencies have the same mitigation setting as you by passing
`-C deny-partial-mitigations=overflow-checks`. That flag can be
overridden by `-C allow-partial-mitigations=overflow-checks`.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

For every mitigation-like option, the compiler CLI flags determines whether that
mitigation allows or denies partial mitigations. This can be turned on
via `-C allow-partial-mitigations=<mitigation>` and turned off by
`-C deny-partial-mitigations=<mitigation>` (for example,
`-C allow-partial-mitigations=stack-protector` and
`-C deny-partial-mitigations=stack-protector`).

These flags act like every other compiler flag, with the last flag winning if there are multiple
values for the same mitigation.

There is no "resetting" of the allow/deny status if the mitigation is overriden, but see
the alternative [with order dependency].

For example,
```
-Callow-partial-mitigations=stack-protector -Callow-partial-mitigations=overflow-checkd
-Callow-partial-mitigations=kcfi -Cdeny-partial-mitigations=overflow-checks -Csanitizer=kcfi
```

Will allow partial mitigations for stack-protector (since there's an allow)
and kcfi (since enabling the sanitizer does not "reset" the allow status),
but deny partial mitigations for `overflow-checks` (since the later deny overrides
the allow). If we decide to go [with order dependency], then in that example
kcfi would deny partial mitigations, since the `-Csanitizer=kcfi` would reset the
`-C allow-partial-mitigations=kcfi`.

[with order dependency]: #with-order-dependency

The default of allow/deny is mitigation-dependent, but can also depend on edition (for
example, it might be best to make `-C control-flow-guard` only deny-by-default from the next
edition), and if we decide to add mitigation enforcement for overflow checks,
it would probably be best to make it allow-by-default.

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
`-C stack-protector=all` (not discussing whether that is a wise idea), they would be
compatible with every configuration of user crates.

The error happens independent of the target crate type (you get an error
if you are building an rlib, not just the final executable).

For example, with `-C stack-protector`, the compatibility table will be
as follows:

|   Dependency\Current   | none | none+allow | strong |   strong + allow partial    |  all  |    strong + allow partial   |
| ---------------------- | ---- | ---------- | ------ | --------------------------- | ----- | --------------------------- |
| none                   |  OK  |      OK    | error  | OK - current allows partial | error | OK - current allows partial |
| none + allow partial   |  OK  |      OK    | error  | OK - current allows partial | error | OK - current allows partial |
| strong                 |  OK  |      OK    |   OK   |             OK              | error | OK - current allows partial |
| strong + allow partial |  OK  |      OK    |   OK   |             OK              | error | OK - current allows partial |
| all                    |  OK  |      OK    |   OK   |             OK              |   OK  |               OK            |
| all + allow partial    |  OK  |      OK    |   OK   |             OK              |   OK  |               OK            |

# Drawbacks
[drawbacks]: #drawbacks

The `-C allow-partial-mitigations=stack-protector` syntax
does not allow for easy appending, unless made order-dependent.

The `-noenforce` syntax is ugly.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Syntax alternatives

### `-C my-mitigation-noenforce`

Instead of `-C allow-partial-mitigations`, it is possible to split every flag value that enables
a mitigation for which enforcement is desired is split into 2 separate values, "enforcing" and
"non-enforcing" mode. The enforcing mode is the default, non-enforcing mode is constructed by
adding `-noenforce` to the name of the value, for example `-C stack-protector=strong-noenforce` or
`-C sanitizer=shadow-call-stack-noenforce`.

If a program has multiple flags of the same kind, the last flag wins, so e.g.
`-C stack-protector=strong-noenforce -C stack-protector=strong` is the same as
`-C stack-protector=strong`.

This is uglier, but acts nicer if distributors want to set flags by default,
see [with order dependency](#with-order-dependency).

#### -C stack-protector=none-noenforce

The option `-C stack-protector=none-noenforce` is the same as
`-C stack-protector=none`. I am not sure whether we should have both, but
it feels that orthogonality is in favor of having both.

### With order dependency

With the way the flag is specified now,
`-C stack-protector=strong -C allow-partial-mitigations=stack-protector -C stack-protector=strong`
is the same as `-C stack-protector=strong -C allow-partial-mitigations=stack-protector`.

This is unfortunate, because `-C stack-protector=strong -C allow-partial-mitigations=stack-protector` is
a pretty good default for distributions to set. If a distribution sets that, and an application
believes they are turning on enforcing stack protection by using `-C stack-protector=strong`,
the application will not be getting enforcement due to the distribution setting
`-C allow-partial-mitigations=stack-protector`.

With a small amount of implementation effort, we could have `-C stack-protector=strong` reset the
`-C allow-partial-mitigations=stack-protector` state, so that
`-C stack-protector=strong -C allow-partial-mitigations=stack-protector -C stack-protector=strong`
is equivalent to `-C stack-protector=strong`.

This would work quite well, but I am not sure that rustc wants to have order between different
kinds of CLI arguments.

## Limiting the set of crates that are allowed to bypass enforcement

You could have a syntax like `-C allow-partial-mitigations=stack-protector=@stdlib+foo`,
`-C stack-protector=strong-noenforce=@stdlib+foo`, or some other syntax (using `+` since
`,` should have a different level of precedence), which would only allow the mitigation
to be partial on a specified set of crate names.

`@stdlib` is used here to stand for all the sysroot crates, since the user does not
want to specify them all (should we bikeshed the syntax?).

This is different from `-C pretend-mitigation-enabled` (which would be a mitigation
equivalent of `-C unsafe-allow-abi-mismatch`), since it reflects a decision
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

As far as I can see, there is no current demand for that sort of sanitizer runtime,
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
distribution sets `-C stack-protector=strong -C allow-partial-mitigations=stack-protector`,
and the user adds `-C stack-protector=strong`, we want the result to be stack-protector set
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
set of mitigations enabled (as of Rust 1.88: [PIC], [NX],
[`-z relro -z now`]), and without `-Z build-std`, it is only possible to use
the mitigation settings in the shipped standard library.

If we find out that some mitigations have a positive cost-benefit ratio
for the standard library (probably at least [`-Z stack-protector`]), we
probably want to ship a standard library supporting them by default, but
in a way that still allows people to compile code without mitigations (for
example, ship a `libstd` with `-C stack-protector=strong`, but allow users
to compile their own code with `-C stack-protector=none` using that
`libstd`) if that fulfills their security/performance tradeoff better.

[PIC]: https://en.wikipedia.org/wiki/Position-independent_code
[NX]: https://en.wikipedia.org/wiki/Executable-space_protection
[`-z relro -z now`]: https://www.redhat.com/en/blog/hardening-elf-binaries-using-relocation-read-only-relro

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

### Target modifier, enforced mitigation, neither, or both?

For every single mitigation-like flag:

1. If mixing values of the flag can cause unsound behavior, sanitizer false
   positives, or crashes, it should be a target modifier.
2. If mixing values of the flag can hurt a production program's security
   posture, it should be an enforced mitigation.
3. If the flag is a sanitizer intended for use only in fuzzing and testing, and
   mixing values of it does not lead to unsound behavior, it should be
   neither.
4. If mixing values of the flag hurt a production program's
   security posture in some cases, and lead to unsound behavior in
   other cases, it should be both. I am not aware of a current flag that
   fits this pattern.

## `-emit=component-info`

One possible "big" alternative would be emitting a component info file, via
`-emit=component-info`. For example:

```json
{
   "components": [
      { 
         "type": "crate",
         "name": "foo",
         "mitigations": ["a"]
      }
   ]
}
```

We could then have Cargo run over that file and look for missing mitigations.

Without support in Cargo, this would be not good enough since it would
take an annoying enough amount of effort for people to scan the generated
json files.

It is important to avoid this sort of JSON being bikeshed-land.

An advantage is that people could write their own custom policies, which would
take some pressure off bikesheds for things like overflow checks.

In some sense, this is moving complexity from rustc to Cargo, which I'm not sure
reduces it overall.

A disadvantage is that it would be possible to enable a mitigation in rustc but
not in Cargo (e.g. by setting `RUSTFLAGS`), which would make it non-enforced.

A big disadvantage of this is that it adds much complexity and deeper-than-we-want
integration to projects that are *not* using Cargo, since they have to
implement the scanning integration themselves.

Even if this alternative is not selected, some sort of `-emit=component-info` does
feel like a good feature, though one that deserves a separate RFC. There does not
seem to be a *conflict* between `-emit=component-info` and
`-C allow-partial-mitigations`.

## Why not an external tool?

This is somewhat hard to do with an external tool, since there is
no way of looking at a binary and telling what mitigations its components
have.

There are however some external tools that do check for mitigations,
but they have limitations:

1. [`hardening-check(1)`] exists, but its check for stack smashing protection only
   checks that at least 1 function has stack cookies, rather than checking that
   every interesting function has it enabled.
2. The Linux kernel has [`objtool`], which checks for some other mitigations (for
   example, retpolines). It however needs to access the `.o` object files
   rather than the final linked executable or shared library - which
   requires its user to control the linking process - and also has hardcoded
   limitations that make it only suitable for the Linux kernel, rather than
   being useful as a general-purpose tool.
3. Fedora has a tool called [`annobin`], which is able to parse the
   [`.gnu.build.attributes`](#gnubuildattributes-1) section, which compilers
   can use to indicate mitigation support. That does fit our needs, but is
   specific to the GNU/Linux world - it is desirable to have a solution that
   works on all platforms.

[`hardening-check(1)`]: https://manpages.debian.org/testing/devscripts/hardening-check.1.en.html
[`objtool`]: https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/tools/objtool/Documentation/objtool.txt?id=5cd64d4f92683afa691a6b83dcad5adfb2165ed0
[`annobin`]: https://sourceware.org/cgit/annobin

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

## .gnu.build.attributes

This is a [Fedora feature]. It actually behaves pretty similarly to how we expect
mitigation enforcement to work - a compiler plugin written by Fedora makes
C/C++ compilers output a `.gnu.build.attributes` section indicating which
mitigations are present or absent. The linker aggregates all of these sections,
and `annocheck` can be used to check whether an object file declares an
interesting mitigation as missing.

This is currently a Fedora-ism as far as I can tell. It can be used with non-Fedora
linkers (of course, in that case, C code will probably not indicate which mitigations
it is missing).

It might be a good idea to coordinate with Fedora and have rustc emit this metadata,
but since it only works on Linux platforms, probably better to not solely rely on it.

[Fedora feature]: https://fedoraproject.org/wiki/Toolchain/Watermark

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
