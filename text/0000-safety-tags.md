- Feature Name: safety_tag
- Start Date: 2025-07-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC introduces a concise safety-comment convention for unsafe code in libstd-adjacent crates:
tag every unsafe function and call with `#[safety { SP1, SP2 }]`.

Safety tags refine today’s safety-comment habits: a featherweight syntax that condenses every
requirement into a single, check-off reminder.

The following snippet [compiles] today, but we expect Clippy and Rust-Analyzer to enforce tag checks
and provide first-class IDE support.

[compiles]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2024&gist=2f49a8b255b8c066ffd5e3157a70b821

```rust
#![feature(custom_inner_attributes)]
#![clippy::safety::r#use(invariant::*)] // 💡

pub mod invariant {
    #[clippy::safety::tag]
    pub fn ValidPtr() {}
}

#[clippy::safety { ValidPtr }] // 💡
pub unsafe fn read<T>(ptr: *const T) {}

fn main() {
    #[clippy::safety { ValidPtr }] // 💡
    unsafe { read(&()) };
}
```

# Motivation
[motivation]: #motivation

To avoid the misuse of unsafe code, Rust developers are encouraged to provide clear safety comments
for unsafe APIs. While these comments are generally human-readable, they can be ambiguous and
laborious to write. Even the current best practices in the Rust standard library are somewhat ad hoc
and informal. Moreover, safety comments are often repetitive and may be perceived as less important
than the code itself, which makes them error-prone and increases the risk that reviewers may
overlook inaccuracies or missing safety requirements.

For instance, a severe problem may arise if the safety requirements of an API change over time:
downstream users may be unaware of such changes and thus be exposed to security risks. Therefore, we
propose to improve the current practice of writing safety comments by making them checkable through
a system of safety tags. These tags are designed to be:

* Compatible with existing safety documentation: Safety tags should be expressive enough to
  represent current safety comments, especially as rendered in today's rustdoc HTML pages.
* Usable by compiler tools for safety checking: If no safety tags are provided for an unsafe API,
  lints should be emitted to remind developers to provide safety requirements. If a safety tag is
  declared for an unsafe API but not discharged at a callsite, lints should be emitted to alert
  developers about potentially overlooked safety requirements.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Syntax of Safety Tags

Syntax of a safety tag is defined as follows:

```text
SafetyTag -> `#` `[` `clippy::safety` `{` Tags `}` `]`

Tags -> Tag (`;` Tag)*

Tag -> ID (`,` ID)* (`:` LiteralString)?

ID -> SingleIdent | SimplePath
```

Here are some tag examples:

```rust
#[clippy::safety { SP }]
#[clippy::safety { SP1, SP2 }]

#[clippy::safety { SP1: "reason" }]
#[clippy::safety { SP1: "reason"; SP2: "reason" }]

#[clippy::safety { SP1, SP2: "shared reason for the two SPs" }]
#[clippy::safety { SP1, SP2: "shared reason for the two SPs"; SP3 }]
#[clippy::safety { SP3; SP1, SP2: "shared reason for the two SPs" }]
```

`#[clippy::safety]` is a [tool attribute] that you attach to an unsafe function (or to an expression
that performs unsafe calls). Take [`ptr::read`]: its safety comment lists three requirements, so we
create three corresponding tags on the function declaration and mark each one off at the call site.

```rust
#[clippy::safety { ValidPtr, Aligned, Initialized }] // defsite
pub unsafe fn read<T>(ptr: *const T) -> T { ... }

#[clippy::safety { ValidPtr, Aligned, Initialized }] // callsite
unsafe { read(ptr) };
```

We can also attach comments for a tag or a group of tags to clarify how safety requirements are met:

```rust
#[clippy::safety {
  ValidPtr, Aligned, Initialized: "addr range p..p+n is properly initialized from aligned memory";
  InBounded, ValidNum: "`n` won't exceed isize::MAX here, so `p.add(n)` is fine";
}]
for _ in 0..n {
    unsafe {
        c ^= p.read();
        p = p.add(1);
    }
}
```

[tool attribute]: https://doc.rust-lang.org/reference/attributes.html#tool-attributes
[`ptr::read`]: https://doc.rust-lang.org/std/ptr/fn.read.html

Every safety tag declared on a function must appear in `#[clippy::safety { ... }]` together with an
optional reason; any omission triggers a warning-by-default diagnostic that lists the missing tags
and explains each one:

```rust
unsafe { ptr::read(ptr) }
```

```rust
warning: `ValidPtr`, `Aligned`, `Initialized` tags are missing. Add them to `#[clippy::safety { }]`.
   --> file.rs:xxx:xxx
    |
LLL | unsafe { ptr::read(ptr) }
    | ^^^^^^^^^^^^^^^^^^^^^^^^^ This unsafe call requires these safety tags.
    |
    = NOTE: See core::ptr::invariants::ValidPtr
    = NOTE: See core::ptr::invariants::Aligned
    = NOTE: See core::ptr::invariants::Initialized
```

The process of verifying whether a tag is present is referred to as tag discharge.

Note that it's allowed to discharge tags of unsafe callees onto the unsafe caller for unsafe
delegation or propogation:

```rust
#[clippy::safety { ValidPtr, Aligned, Initialized }] // ✅
unsafe fn constructor<T>() -> T {
    unsafe { read(...) }
}
```

## Safety Tags as Ordinary Items

Before tagging a function, we must declare them as ordinary items with `#[clippy::safety::tag]` such
as [uninhabited] types or plain functions:

```rust
#[clippy::safety::tag]
enum ValidPtr {}

#[clippy::safety::tag]
fn Aligned() {}
```

Tags live in their own [type namespace] carry item-level [scopes] and obey [visibility] rules,
keeping the system modular and collision-free. Since they are never referenced directly as real
items, we propose importing them uses a dedicated syntax: inner-styled or outer-styled 
`clippy::safety::r#use` tool attribute on modules:

```rust
#![clippy::safety::r#use { UseTree })] // {} signifies a delimiter here, thus () also works
```

[`UseTree`] follows the exact grammar of the `use` declaration. Some examples:

```rust
#[clippy::safety::r#use { core::ptr::invariants::* }]
mod foo;

mod bar {
    #![clippy::safety::r#use { core::ptr::invariants::{ValidPtr, Aligned} }]
}
```

That's to say:
* Tags declared or re-exported in the current module are automatically in scope: no import required.
* Tags from other modules must be brought in with the inner-tool attribute shown above.
* Tags are visible and available to downstream crates whenever their declaration paths are public.
* Attempting to import a tag from a private module is a **hard error**.
* Referencing a tag that has never been declared is also a **hard error**.

[uninhabited]: https://doc.rust-lang.org/reference/glossary.html#uninhabited
[type namespace]: https://doc.rust-lang.org/reference/names/namespaces.html
[scopes]: https://doc.rust-lang.org/reference/names/scopes.html#item-scopes
[visibility]: https://doc.rust-lang.org/reference/visibility-and-privacy.html
[`UseTree`]: https://doc.rust-lang.org/reference/items/use-declarations.html

Tags are treated as items so rustdoc can render their documentation and hyperlink tag references.
And Rust-Analyzer can offer **full IDE support**: completion, go-to-definition/declaration, and
doc-hover.

Tags constitute a public API; therefore, any alteration to their declaration or definition must be
evaluated against [Semantic Versioning][semver].
* Adding a tag declaration or definition is a **minor** change.
* Removing a tag declaration or definition is a **major** change.

To give dependent crates time to migrate, mark obsolete tag items with `#[deprecated]`. Clippy will
surface the deprecation warning whenever the tag is used w.r.t definitions and discharges.

[semver]: https://doc.rust-lang.org/cargo/reference/semver.html

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Unstable Features

Currently, safety tags requires the following unstable features
* `#![feature(proc_macro_hygiene, stmt_expr_attributes)]` for tagging statements or expressions.
* `#![feature(custom_inner_attributes)]` for `#![clippy::safety::r#use(...)]` imports

Since the safety-tag mechanism is implemented primarily in Clippy and Rust-Analyzer, no additional
support is required from rustc.

But we ask the libs team to adopt safety tags for all public `unsafe` APIs in libstd, along with
their call sites. To enable experimentation, a nightly-only library feature
`#![feature(safety_tags)]` should be introduced and remain unstable until the design is finalized.

## Implementation in Clippy

Procedure:

1. Scan the crate for every item marked `#[clippy::safety::tag]`; cache the compiled tag metadata of
   upstream dependencies under `target/` for later queries.
2. Validate every `#![clippy::safety { ... }]` import by a reachability analysis that ensures every
   referenced tag is defined and accessible.
3. Verify that every unsafe call carries the required safety tags:
   * Resolve the callee, collect its declared tags, then walk outward from the call site until the
     function’s own signature confirms these tags are listed in its `#![clippy::safety::r#use(...)]`
     attribute.
   * Tags are only discharged inside or onto an `unsafe fn`; it's an error to tag a safe function.
   * If an unsafe call lacks any required tag, emit a diagnostic whose severity (warning or error)
     is governed by the configured Clippy lint level.

Libraries in `rust-lang/rust` must enforce tag checking as a hard error, guaranteeing that every tag
definition and discharge is strictly valid.

## Implementation in Rust Analyzer

Safety-tag analysis requirements:

* Harvest every item marked `#[clippy::safety::tag]`, including those pulled in from dependencies.
* Offer tag path completion for `#![clippy::safety::r#use(...)]`.
* Offer tag name completion for `#[clippy::safety { ... }]` on unsafe functions, let-statements, or
  expressions.
* Validate all tags inside `#[clippy::safety { ... }]`, and support “go-to-definition” plus inline
  documentation hover.

# Drawbacks
[drawbacks]: #drawbacks

Even though safety tags are machine-readable, their correctness still hinges on human review:
developers can silence Clippy by discharging tags without verifying underlying safety requirements.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Alternatives from IRLO

There are alternative discussion or Pre-RFCs on IRLO:

* 2023-10: [Ability to call unsafe functions without curly brackets](https://internals.rust-lang.org/t/ability-to-call-unsafe-functions-without-curly-brackets/19635/22)
  * This is a discussion about make single unsafe call simpler, so the idea evolved into tczajka's Pre-RFC.
  * But the idea and syntax from Scottmcm's comments are very enlightening to our RFC.
* 2024-10: [Detect and Fix Overscope unsafe Block](https://internals.rust-lang.org/t/detect-and-fix-overscope-unsafe-block/21660/19) 
  * The OP is about safe code scope in big unsafe block, which is not discussed in our RFC.
  * But scottmcm's comments are good inspiration for our RFC.
* 2024-12: [Pre-RFC: Unsafe reasons](https://internals.rust-lang.org/t/pre-rfc-unsafe-reasons/22093) proposed by chrefr
  * This is a good improvement on abstracting safety comments into a single, machine-readable and
    checkable identifier. However, it doesn't specify arguments and lacks more fine-grained string
    interpolation for detailing unsafe reasons.
  * It also requests big changes on language and compiler change, while safety tags in our RFC is lightweight
* 2025-02: [Pre-RFC: Single function call `unsafe`](https://internals.rust-lang.org/t/pre-rfc-single-function-call-unsafe/22343) proposed by tczajka
  * The practice of using a single unsafe call is good, but the postfix `.unsafe` requires more
    compiler support and does not offer suggestions for improving safe comments.
  * Our RFC, however, supports annotating safety tags on any expression, including single calls.
* 2025-05: [Pre-RFC: Granular Unsafe Blocks - A more explicit and auditable approach](https://internals.rust-lang.org/t/pre-rfc-granular-unsafe-blocks-a-more-explicit-and-auditable-approach/23022) proposed by Redlintles
  * The safety categories suggested are overly broad. In contrast, the safety properties outlined in
    our RFC are more granular and semantics-specific.
* 2025-07: [Unsafe assertion invariants](https://internals.rust-lang.org/t/unsafe-assertion-invariants/23206)
  * It’s a good idea to embed safety requirements into doc comments, which aligns with one of the
    goals in our RFC.
* 2025-07: [Pre-RFC: Safety Property System](https://internals.rust-lang.org/t/pre-rfc-safety-property-system/23252) proposed by vague
  * It's a draft of our current proposal, but more focused on custom linter's design. Also see 
    [this thread in opsem channel](https://rust-lang.zulipchat.com/#narrow/channel/136281-t-opsem/topic/Safety.20Property.20System/with/530679491).
  * The critical parts have already been refined from Clippy’s perspective in current proposal.

## Alternatives from Rust for Linux

More importantly, our proposal is a big improvement to these proposals, which Rust for Linux care
more about:
* 2024-09: [Rust Safety Standard: Increasing the Correctness of unsafe Code][Rust Safety Standard]
  proposed by Benno Lossin
  * These slides outline the motivations and objectives of safety-documentation standardization —
    exactly what our proposal aims to deliver.
  * They omit implementation details; nevertheless, Predrag (see next line) and we remain faithful
    to their intent.
* 2024-10: [Automated checking of unsafe code requirements](https://hackmd.io/@predrag/ByVBjIWlyx)
  proposed by Predrag
  * Predrag’s proposal focuses on structured safety comments, entity references, requirement
    discharges, and the careful handling of soundness hazards when safety rules evolve. Most are
    compatible with our proposal.
  * The principal divergence is syntactic: Predrag embeds the rules in doc- and line-comments, which
    remain highly readable for humans, but not so much for tools, because line-comments are 
    discarded early by the compiler. This makes retrieving a rule for a specific expression far 
    harder than with [`stmt_expr_attributes`](https://github.com/rust-lang/rust/issues/15701).

Originally, we only focus on libstd's common safety propeties ([paper]), but noticed the RustWeek
[meeting note] in zulipchat. Thus [tag-std#3](https://github.com/Artisan-Lab/tag-std/issues/3) is
opened to support Rust for Linux on safety standard.

[meeting note]: https://hackmd.io/@qnR1-HVLRx-dekU5dvtvkw/SyUuR6SZgx
[Rust Safety Standard]: https://kangrejos.com/2024/Rust%20Safety%20Standard.pdf
[paper]: https://arxiv.org/abs/2504.21312

# Prior art
[prior-art]: #prior-art

Currently, there are efforts on introducing contracts and formal verification into Rust:
* [contracts](https://rust-lang.github.io/rust-project-goals/2024h2/Contracts-and-invariants.html):
  the lang experiment has been implemented since
  [rust#128044](https://github.com/rust-lang/rust/issues/128044).
* [verify-rust-std] pursues applying formal verification tools to libstd. Also see Rust Foundation
  [announcement][vrs#ann], project goals during [2024h2] and [2025h1].

While safety tags are less formally verified and intended to be a check list on safety requirements.

[verify-rust-std]: https://github.com/model-checking/verify-rust-std
[vrs#ann]: https://foundation.rust-lang.org/news/rust-foundation-collaborates-with-aws-initiative-to-verify-rust-standard-libraries/
[2024h2]: https://rust-lang.github.io/rust-project-goals/2024h2/std-verification.html
[2025h1]: https://rust-lang.github.io/rust-project-goals/2025h1/std-contracts.html

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Should Tags Take Arguments?

If a tag takes arguments, how should the tag declared? An uninhabited enum doesn't express 
correct semantics anymore. The closest item is a function with arguments. But it's brings
many problems, like 
* What argument types should be on such tag declaration functions? Do we need to declare extra 
types specifically for such tag declaration?
* Is there a argument check on tag definitions on unsafe functions? Are we reinventing contracts
system again?
* Or just only keep uninhabited enums as tag declaration, and allow any arguments for all tags,
but don't check the validity arguments. Tag arguments enhance precision of safety operation,
but no checks on them avoid any complicated interaction with type system. We can make a compromise.

When a tag needs parameters, we must decide what its declaration looks like.  An uninhabited enum
can no longer express “this tag carries data”, so the nearest legal item is a function whose
parameters represent the tag’s arguments.  Unfortunately, this immediately raises design questions:

1. **Argument types**  Which types are allowed in the declaration?  Do we have to introduce new,
   purpose-built types just so the compiler can see them at the use-site?

2. **Definition-side checking**  Will `unsafe fn` definitions be obliged to supply arguments that
   type-check against the declaration?  If so, are we quietly reinventing a full contract system?

I'd like to propose a solution or rather compromise here by trading strict precision for simplicity:

We could keep uninhabited enums as the only formal declaration and allow *any* arguments at use
sites, skipping all validation. Tag arguments would still refine the description of an unsafe
operation, but they are never type checked.

Also see [this][tag-args] RFC discussion for our thoughts.

[tag-args]: https://github.com/rust-lang/rfcs/pull/3842#discussion_r2246551643

## Tagging on Unsafe Traits and Impls

We can extend safety definitions to unsafe traits and require discharges in unsafe trait impls.

Crates with heavy unsafe-trait usage likely needs the extension. We’d welcome more minds on this.

## Tagging on Datastructures

We believe safety requirements are almost always imposed by unsafe functions, so tagging a struct,
enum, or union is neither needed nor permitted.

## Tagging on Unsafe Fields

[Unsafe fields] are declared and accessed with the `unsafe` keyword, often accompanied by safety
comments. We could extend safety tags to cover unsafe fields as well, both in their definitions and
at every access point they are discharged.

[Unsafe fields]: https://github.com/rust-lang/rfcs/pull/3458

# Future possibilities
[future-possibilities]: #future-possibilities

## Generate Safety Docs from Tags

We can take structured safety comments one step further by turning the explanatory prose into
explicit tag reasons.

For `ptr::read`, the existing comments are replaced with safety tags:

```rust
/// * `src` must be [valid] for reads.
/// * `src` must be properly aligned. Use [`read_unaligned`] if this is not the case.
/// * `src` must point to a properly initialized value of type `T`.
pub const unsafe fn read<T>(src: *const T) -> T { ... }
```

```rust
#[safety {
    ValidPtr: "`src` must be [valid] for reads";
    Aligned: "`src` must be properly aligned. Use [`read_unaligned`] if this is not the case";
    Initialized: "`src` must point to a properly initialized value of type `T`"
}]
pub const unsafe fn read<T>(src: *const T) -> T { ... }
```

`#[safety]` becomes a procedural macro that expands to both `#[doc]` attributes and the
`#[clippy::safety]` attribute.

```rust
/// # Safety
/// 
/// - ValidPtr: `src` must be [valid] for reads
/// - Aligned: `src` must be properly aligned. Use [`read_unaligned`] if this is not the case
/// - Initialized: `src` must point to a properly initialized value of type `T`
#[clippy::safety { ValidPtr, Aligned, Initialized }]
pub const unsafe fn read<T>(src: *const T) -> T { ... }
```

## `any { Option1, Option2 }` Tags

Sometimes it’s useful to declare a set of safety tags on an unsafe function while discharging only
one of them.

For instance, `ptr::read` could expose the grouped tag `any { DropCheck, CopyType }` and then
discharge either `DropCheck` or `CopyType` at the call site, depending on the concrete type `T`.

Another instance is `<*const T>::as_ref`, whose safety doc states that the caller must guarantee
“the pointer is either null or safely convertible to a reference”. This can be expressed as
`#[clippy::safety { any { Null, ValidPtr2Ref } }]`, allowing the caller to discharge whichever tag
applies.

## Entity References and Code Review Enhancement

To cut boilerplate or link related code locations, we introduce `#[clippy::safety::ref(...)]` which
establishes a two-way reference.

An example of this is [IntoIter::try_fold][vec_deque] of VecDeque, using `#[ref]` for short:

[vec_deque]: https://github.com/rust-lang/rust/blob/ebd8557637b33cc09b6ee8273f3154d5d3af6a15/library/alloc/src/collections/vec_deque/into_iter.rs#L104

```rust
fn try_fold<B, F, R>(&mut self, mut init: B, mut f: F) -> R
    impl<'a, T, A: Allocator> Drop for Guard<'a, T, A> {
        #[ref(try_fold)] // 💡 ptr::read below relies on this drop impl
        fn drop(&mut self) { ... }
    }
    ...

    init = head.iter().map(|elem| {
        guard.consumed += 1;

        #[ref(try_fold)] // 💡
        #[safety {
            ValidPtr, Aligned, Initialized,
            DropCheck: "Because we incremented `guard.consumed`, the deque \
              effectively forgot the element, so we can take ownership."
        }]
        unsafe { ptr::read(elem) }
    })
    .try_fold(init, &mut f)?;

    tail.iter().map(|elem| {
        guard.consumed += 1;

        #[ref(try_fold)] // 💡 No longer to write SAFETY: Same as above.
        unsafe { ptr::read(elem) }
    })
    .try_fold(init, &mut f)
}

fn try_rfold<B, F, R>(&mut self, mut init: B, mut f: F) -> R {
    impl<'a, T, A: Allocator> Drop for Guard<'a, T, A> {
        #[ref(try_fold)] // 💡
        fn drop(&mut self) { ... }
    }
    ...

    init = tail.iter().map(|elem| {
            guard.consumed += 1;

            #[ref(try_fold)] // 💡 No longer to write SAFETY: See `try_fold`'s safety comment.
            unsafe { ptr::read(elem) }
        })
        .try_rfold(init, &mut f)?;

    head.iter().map(|elem| {
            guard.consumed += 1;

            #[ref(try_fold)] // 💡 No longer to write SAFETY: Same as above.
            unsafe { ptr::read(elem) }
        })
        .try_rfold(init, &mut f)
}
```

These `#[ref]` annotations act as cross-references that nudge developers to inspect every linked
site. When either end or the code around it changes, reviewers are instantly aware of all affected
locations that Clippy reports and thus can assess if every referenced safety requirement is still
satisfied.

