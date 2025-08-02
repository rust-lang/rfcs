- Feature Name: safety_tag
- Start Date: 2025-07-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC introduces a concise safety-comment convention for unsafe code in libstd-adjacent crates:
tag every public unsafe function with `#[safety::requires]` and call with `#[safety::checked]`.

Safety tags refine today’s safety-comment habits: a featherweight syntax that condenses every
requirement into a single, check-off reminder.

The following snippet [compiles] today if we enable enough nightly features, but we expect Clippy
and Rust-Analyzer to enforce tag checks and provide first-class IDE support.

[compiles]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2024&gist=8b22aebccf910428008c4423c436d81e

```rust
#![safety::import(invariant::*)] // 💡

pub mod invariant {
    #[safety::declare_tag] // 💡
    pub enum ValidPtr() {}
}

#[safety::requires { ValidPtr }] // 💡
pub unsafe fn read<T>(ptr: *const T) {}

fn main() {
    #[safety::checked { ValidPtr }] // 💡
    unsafe { read(&()) };
}
```

# Motivation
[motivation]: #motivation

## Safety Invariants: Forgotten by Authors, Hidden from Reviewers

To avoid the misuse of unsafe code, Rust developers are encouraged to provide clear safety comments
for unsafe APIs. Safety comments are often repetitive and may be perceived as less important
than the code itself, which makes them error-prone and increases the risk that reviewers may
overlook inaccuracies or missing safety requirements.

Recent Rust issues [#134496],[#135805], and [#135009] illustrate the problem: several libstd APIs
silently rely on the global allocator, yet their safety comments never state the crucial invariant
that any raw pointer passed to them must refer to memory allocated by that same global allocator.
The pattern is always the same.

```rust
// Ptr is possibly from another allocation.
pub unsafe fn from_raw(ptr: *const T) -> Self {
    // SmartPoiner can be Box, Arc, Rc, and Weak.
    unsafe { SmartPoiner::from_raw_in(ptr, Global) }
}
```

[#134496]: https://github.com/rust-lang/rust/pull/134496
[#135805]: https://github.com/rust-lang/rust/pull/135805
[#135009]: https://github.com/rust-lang/rust/pull/135009

Even if the safety documentation is complete, two problems remain:

- When *writing* the call, the author may forget the inline safety comment that proves every
  invariant has been identified and upheld.  
- When *reviewing* the call, the absence of such a comment forces the auditor to reconstruct the
  required invariants from scratch, with no assurance that the author considered them at all.

## Granular Unsafe: How Small Is Too Small?

The unsafe block faces a built-in tension:
- **Precision** demands the smallest possible scope, hence proposals for prefix or postfix `unsafe`
  operators that wrap a single unsafe call (see “[Alternatives from IRLO]” for such proposals).  
- **Completeness** demands the opposite: unsafe code often depends on surrounding safe (or other
  unsafe) code to satisfy its safety invariants, so the scope that must be considered “safe” 
  balloons outward.

[Alternatives from IRLO]: #IRLO

## Safety Invariants Have No Semver

A severe problem may arise if the safety requirements of an API change over time: downstream users
may be unaware of such changes and thus be exposed to security risks. 

## Formal Contracts, Casual Burden

[Contracts][contracts] excel at enforcing safety invariants rigorously, but they demand the
precision as well as overhead of formal verification, making them too heavy for everyday projects.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

We propose **checkable safety comments** via Clippy’s new **safety-tag** system, addressing today’s
ad-hoc practice with four concrete gains:

1. **Shared clarity**. Authors attach a short tag above every unsafe operation; reviewers instantly
   see which invariant must hold and where it is satisfied.

2. **Semantic granularity**. Tags can label a single unsafe call, a loop body, or the entire
   caller - no longer constrained by the visual boundaries of `unsafe {}`.

3. **Versioned invariants**. Tags are real items; any change to their declaration or definition is a
   **semver-breaking** API change, so safety invariants evolve explicitly.

4. **Lightweight checking**. Clippy only matches tag paths. No heavyweight formal proofs, keeping
   the system easy to adopt and understand.

## `#[safety]` Tool Attribute and Namespace

Safety tags have no effect on compilation; they merely document safety invariants. Therefore, we
propose that `#[safety]` be implicitly registered for every crate.

```rust
#![feature(register_tool)]
#![register_tool(safety)]
```

## Syntax of Safety Tags

Syntax of a safety tag is defined as follows:

```text
SafetyTag -> `#` `[` `safety::` Operation Object `]`

Operation -> requires | checked

Object -> `[` Tags `]` | `(` Tags `)` | `{` Tags `}`

Tags -> Tag (`;` Tag)*

Tag -> ID (`,` ID)* (`:` LiteralString)?

ID -> SingleIdent | SimplePath
```

Here are some tag examples:

```rust
#[safety::requires { SP }]
#[safety::requires { SP1, SP2 }]

#[safety::checked { SP1: "reason" }]
#[safety::checked { SP1: "reason"; SP2: "reason" }]

#[safety::checked { SP1, SP2: "shared reason for the two SPs" }]
#[safety::checked { SP1, SP2: "shared reason for the two SPs"; SP3 }]
#[safety::checked { SP3; SP1, SP2: "shared reason for the two SPs" }]
```

`#[safety]` is a tool attribute with two forms to operate on safety invariants:
* `safety::requires` is placed on an unsafe function’s signature to state the safety invariants that
callers must uphold;
* `safety::checked` is placed on an expression or let-statement that wraps an unsafe call.

Take [`ptr::read`] as an example: its safety comment lists three requirements, so we create three
corresponding tags on the function declaration and mark each one off at the call site.

```rust
#[safety::requires { ValidPtr, Aligned, Initialized }] // defsite
pub unsafe fn read<T>(ptr: *const T) -> T { ... }

#[safety::checked { ValidPtr, Aligned, Initialized }] // callsite
unsafe { read(ptr) };
```

We can also attach comments for a tag or a group of tags to clarify how safety requirements are met:

```rust
#[safety::checked {
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

When calling an unsafe function, tags defined on it must be present in `#[safety::checked]` or
`#[safety::requires]` together with an optional reason; any omission triggers a warning-by-default
diagnostic that lists the missing tags and explains each one:

```rust
unsafe { ptr::read(ptr) }
```

```rust
warning: `ValidPtr`, `Aligned`, `Initialized` tags are missing. Add them to `#[safety::checked]` or
         `#[safety::requires]` if you're sure these invariants are satisfied.
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
#[safety::requires { ValidPtr, Aligned, Initialized }] // ✅
unsafe fn delegation<T>(ptr: *const T) -> T {
    unsafe { read(ptr) }
}
```

Partial discharges are also allowed:

```rust
#[safety::requires {
  ValidPtr, Initialized: "ensure the allocation spans at least size_of::<T>() bytes past ptr"
}]
unsafe fn propogation<T>(ptr: *const T) -> T {
    let align = mem::align_of::<T>();
    let addr = ptr as usize;
    let aligned_addr = (addr + align - 1) & !(align - 1);

    #[safety::checked { Aligned: "alignment of ptr has be adjusted" }]
    unsafe { read(ptr) }
}
```

## Safety Tags as Ordinary Items

Before tagging a function, we must declare them using `#[safety::declare_tag]` as an [uninhabited]
enum whose value is never constructed:

```rust
#[safety::declare_tag]
enum ValidPtr {}
```

Tags live in their own [type namespace] carry item-level [scopes] and obey [visibility] rules,
keeping the system modular and collision-free.

However, tag items are only used in safety tool attribute and never really used in user own code, we
propose importing them uses a dedicated syntax: inner-styled or outer-styled `safety::import` tool
attribute on modules and takes [`UseTree`] whose grammar is shared with that in `use` declaration.
Some examples:

```rust
// outer-styled import: import tag Bar to foo module
#[safety::import(crate::invariants::Bar)]
mod foo;

// inner-styled import: equivalent to the above,
// but must enable #![feature(custom_inner_attributes)]
mod foo { #![safety::import(crate::invariants::Bar)] }

// Below are examples to import multiple tags into scope.

#[safety::import { core::ptr::invariants::* }]
mod foo;

mod bar {
    #![safety::import { core::ptr::invariants::{ValidPtr, Aligned} }]
}
```

That's to say:
* Tags declared or re-exported in the current module are automatically in scope: no import required.
* To use tags defined in other modules or crates, attach the `safety::import` attribute to current
  module.
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

Currently, safety tags requires the following features
* `#![feature(proc_macro_hygiene, stmt_expr_attributes)]` for tagging statements or expressions.
* `#![feature(custom_inner_attributes)]` for `#![safety::import(...)]`.
* registering `safety` tool in every crate to create safety namespace.

Since the safety-tag mechanism is implemented primarily in Clippy and Rust-Analyzer, no additional
significant support is required from rustc.

But we ask the libs team to adopt safety tags for all public `unsafe` APIs in libstd, along with
their call sites. To enable experimentation, a nightly-only library feature
`#![feature(safety_tags)]` should be introduced and remain unstable until the design is finalized.

## Implementation in Clippy

Procedure:

1. Scan the crate for every item marked `#[safety::declare_tag]`; cache the compiled tag metadata of
   upstream dependencies under `target/` for later queries.
2. Validate every tags in `#![safety::import]` through a reachability analysis to ensure the paths
   are accessible.
3. Verify that every unsafe call carries the required safety tags:
   * Resolve the callee, collect its declared tags, then walk outward from the call site until the
     function’s own signature confirms these tags are listed in `#[safety::requires]`.
   * Tags are only discharged inside or onto an `unsafe fn`; it's an error to tag a safe function.
   * If an unsafe call lacks any required tag, emit a diagnostic whose severity (warning or error)
     is governed by the configured Clippy lint level.

Libraries in `rust-lang/rust` must enforce tag checking as a hard error, guaranteeing that every tag
definition and discharge is strictly valid.

## Implementation in Rust Analyzer

Safety-tag analysis requirements:

* Harvest every item marked `#[safety::declare_tag]`, including those pulled in from dependencies.
* Offer tag path completion for `#![safety::import]`.
* Offer tag name and path completion for `#[safety::requires]` on unsafe functions, and
  `#[safety::checked]` on let-statements, or expressions.
* Validate all tags inside `#[safety::{requires,checked}]`, and support “go-to-definition” plus
  inline documentation hover.

# Drawbacks
[drawbacks]: #drawbacks

Even though safety tags are machine-readable, their correctness still hinges on human review:
developers can silence Clippy by discharging tags without verifying underlying safety requirements.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale for the Proposed Implementation

We argued in the [guide-level-explanation] why safety tags are necessary; here we justify the *way*
they are proposed to implemente.

1. It's linter's work. Tag checking is an API-style lint, not a language feature. All the
   `#[safety::*]` information is available in the HIR, so a linter (rather than the compiler) is the
   right place to enforce the rules.

2. Re-use the existing linter. We prototyped the checks in [safety-tool], but a stand-alone tool
   cannot scale: project may pin its own toolchain, so we would need one binary per toolchain. We
   already maintain three separate feature-gated builds just for verify-rust-std, rust-for-linux,
   and Asterinas. Once the proposal is standardised, the only sustainable path is to upstream the
   lint pass into Clippy. Project building on stable or nightly toolchains since then will get the
   checks automatically whenever it runs Clippy. Moreover, if `rust-lang/rust` repo has already
   undergone Clippy CI, so no extra tooling is required for tag checking on the standard libraries.

3. IDE integration. The same reasoning applies to the language server. A custom server would again
   be tied to internal APIs and specific toolchains. Extending rust-analyzer is therefore the only
   practical way to give users first-class IDE support.

We therefore seek approvals from the following teams:

1. **Library team** – to allow the tagging of unsafe operations and to expose tag items as public
   APIs.
2. **Clippy team** – to integrate tag checking into the linter.  
3. **Rust-Analyzer team** – to add IDE support for tags.  
4. **Compiler team** – to reserve the `safety` namespace and gate the feature via
   `#![feature(safety_tags)]`.

[safety-tool]: https://github.com/Artisan-Lab/tag-std/blob/main/safety-tool

## Alternatives from IRLO

<a id="IRLO"></a>

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

## Safety Standard Proposal from Rust for Linux

More importantly, our proposal is a big improvement to these proposals, which Rust for Linux care
more about:
* 2024-09: [Rust Safety Standard: Increasing the Correctness of unsafe Code][Rust Safety Standard]
  proposed by Benno Lossin
  * These slides outline the motivations and objectives of safety-documentation standardization —
    exactly what our proposal aims to deliver.
  * They omit implementation details; nevertheless, Predrag (see next section) and we remain
    faithful to their intent.

[meeting note]: https://hackmd.io/@qnR1-HVLRx-dekU5dvtvkw/SyUuR6SZgx
[Rust Safety Standard]: https://kangrejos.com/2024/Rust%20Safety%20Standard.pdf

## Why Not Structured Safety Comments?

* 2024-10: [Automated checking of unsafe code requirements](https://hackmd.io/@predrag/ByVBjIWlyx)
  proposed by Predrag
  * Predrag’s proposal focuses on structured safety comments, entity references, requirement
    discharges, and the careful handling of soundness hazards when safety rules evolve. Most are
    compatible with our proposal.
  * The principal divergence is syntactic: Predrag embeds the rules in doc- and line-comments, which
    remain highly readable for humans, but not so much for tools, because line-comments are 
    discarded early by the compiler. This makes retrieving a rule for a specific expression far 
    harder than with [`stmt_expr_attributes`](https://github.com/rust-lang/rust/issues/15701).
* 2025-07: [Reddit Post: Safety Property System](https://internals.rust-lang.org/t/pre-rfc-safety-property-system/23252/22)
  discussed by Matthieum 

Our proposed syntax looks closer to structured comments:

```rust
#[safety {
  ValidPtr, Align, Initialized: "`self.head_tail()` returns two slices to live elements.";
  NotOwned: "because we incremented...";
}]
unsafe { ptr::read(elem) }
```

```rust
//  SAFETY
//  - ValidPtr, Aligned, Initialized: `self.head_tail()` returns two slices to live elements.
//  - NotOwned: because we incremented...
```

# Prior art
[prior-art]: #prior-art

Currently, there are efforts on introducing contracts and formal verification into Rust:
* [contracts]: the lang experiment has been implemented since [rust#128044].
* [verify-rust-std] pursues applying formal verification tools to libstd. Also see Rust Foundation
  [announcement][vrs#ann], project goals during [2024h2] and [2025h1].

While safety tags are less formally verified and intended to be a check list on safety requirements.

[contracts]: https://rust-lang.github.io/rust-project-goals/2024h2/Contracts-and-invariants.html
[rust#128044]: https://github.com/rust-lang/rust/issues/128044
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

## Alternatives of Syntax for `requires` and `checked`

@kennytm suggested [named arguments] in such tags:

```rust
// Alternative1: change our `:` to `=` before reasons.
#[safety::checked(Tag1 = "reason1", Tag2 = "reason2", Tag3)]
// This would be ugly for tag arguments (if we allow them) and grouped tags.
#[safety::checked(Tag1(arg1) = "reason1", Tag2(arg2) = "reason2", Tag3(arg3))]
// Ambiguous here: reason for Tag2 or (Tag1, Tag2)?
#[safety::checked(Tag1, Tag2 = "reason for what???", Tag3(arg3))] 
```

[Lint reasons] inspires the following improved form that groups tags within single attribute and
uses the `reason` field to explain why invariants are satisfied.

```rust
// Alternative2: each groupe of tags is in single attribute
#[safety::checked(Tag1, reason = "reason1")]
#[safety::checked(Tag1, Tag2, reason = "reason for Tag1 and Tag2")]
#[safety::checked(Tag1(arg1), reason = "reason for Tag1 and Tag2")]
```

Downside of alternative2 is discharge of single tag results in verbose syntax and lines of code:

```rust
// Must discharge separate tags in separate attributes:
#[safety::checked(Tag1, reason = "reason1")]
#[safety::checked(Tag2, reason = "reason2")]
#[safety::checked(Tag3, reason = "reason3")]
```

By comparison, our proposed syntax is

```rust
#[safety::checked { Tag1: "reason1", Tag2: "reason2", Tag3: "reason3" }]
```

[named arguments]: https://github.com/rust-lang/rfcs/pull/3842#discussion_r2247342603
[Lint reasons]: https://doc.rust-lang.org/reference/attributes/diagnostics.html#lint-reasons

## Encapsulate Tag Item Declaration with `define_safety_tag!`

@clarfonthey [suggested][tool macro] a `define_safety_tag!` tool macro which will unlikely happen.

But I think it'd be necessary and handy to hide tag declarations in some cases.

[tool macro]: https://github.com/rust-lang/rfcs/pull/3842#discussion_r2245923920

# Future possibilities
[future-possibilities]: #future-possibilities

## Better Rustdoc Rendering

Because tags are surfaced as real API items, rustdoc can give `#[safety::declare_tag]`–annotated,
uninhabited enums (the tag items) special treatment: it renders compact pages for them and
establishes bidirectional links between tag items and unsafe functions requiring the tags.

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
`#[safety::requires]` attribute.

```rust
/// # Safety
/// 
/// - ValidPtr: `src` must be [valid] for reads
/// - Aligned: `src` must be properly aligned. Use [`read_unaligned`] if this is not the case
/// - Initialized: `src` must point to a properly initialized value of type `T`
#[safety::requires { ValidPtr, Aligned, Initialized }]
pub const unsafe fn read<T>(src: *const T) -> T { ... }
```

With support for tag arguments, safety documentation can be made more precise and contextual by
dynamically injecting the argument values into the reason strings.

## `any { Option1, Option2 }` Tags

Sometimes it’s useful to declare a set of safety tags on an unsafe function while discharging only
one of them.

For instance, `ptr::read` could expose the grouped tag `any { DropCheck, CopyType }` and then
discharge either `DropCheck` or `CopyType` at the call site, depending on the concrete type `T`.

Another instance is `<*const T>::as_ref`, whose safety doc states that the caller must guarantee
“the pointer is either null or safely convertible to a reference”. This can be expressed as
`#[safety::requires { any { Null, ValidPtr2Ref } }]`, allowing the caller to discharge whichever tag
applies.

## Entity References and Code Review Enhancement

To cut boilerplate or link related code locations, we introduce `#[safety::ref(...)]` which
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

