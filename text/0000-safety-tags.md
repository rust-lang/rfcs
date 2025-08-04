- Feature Name: safety_tag
- Start Date: 2025-07-29
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC introduces a concise safety-comment convention for unsafe code in standard libraries:
tag every public unsafe function with `#[safety::requires]` and call with `#[safety::checked]`.

Safety tags refine today’s safety-comment habits: a featherweight syntax that condenses every
requirement into a single, check-off reminder.

The following snippet [compiles] today if we enable enough nightly features, but we expect Clippy
and Rust-Analyzer to enforce tag checks and provide first-class IDE support.

[compiles]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2024&gist=6eb0e47c416953da1f2470b11417e69a

```rust
#[safety::requires { // 💡 define safety tags on an unsafe function
    ValidPtr = "src must be valid for reads",
    Aligned = "src must be properly aligned, even if T has size 0",
    Initialized = "src must point to a properly initialized value of type T"
}]
pub unsafe fn read<T>(ptr: *const T) { }


fn main() {
    #[safety::checked { // 💡 discharge safety tags on an unsafe call
        ValidPtr, Aligned, Initialized = "optional reason"
    }]
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

2. **Semantic granularity**. Tags must label a single unsafe call, or an expression contains single 
   unsafe call. No longer constrained by the visual boundaries of `unsafe {}`. This sidesteps the
   precision vs completeness tension of unsafe blocks, and zeros in on real unsafe operations.
   * To enable truly semantic checking, we envision an [entity-reference] system that meticulously
     traces every unsafe related operation that could break an invariant in source code.

3. **Versioned invariants**. Tags are real items; any change to their declaration or definition is a
   *semver-breaking* API change, so safety invariants evolve explicitly.

4. **Lightweight checking**. Clippy only matches tag paths. No heavyweight formal proofs, keeping
   the system easy to adopt and understand.

[entity-reference]: #entity-reference

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

Tags -> Tag (`,` Tag)* `,`?

Tag -> ID (`=` LiteralString)?

ID -> SingleIdent
```

Here are some tag examples:

```rust
#[safety::requires { SP }]
#[safety::requires { SP1 = "description1", SP2 = "description2" }]

#[safety::checked { SP }]
#[safety::checked { SP1 = "description1", SP2 = "description2" }]
```

`#[safety]` is a tool attribute with two forms to operate on safety invariants:
* `safety::requires` is placed on an unsafe function’s signature to state the safety invariants that
  callers must uphold;
* `safety::checked` is placed on an expression that wraps an unsafe call.

Take [`ptr::read`] as an example: its safety comment lists three requirements, so we create three
corresponding tags on the function declaration and mark each one off at the call site.

```rust
#[safety::requires { ValidPtr, Aligned, Initialized }] // defsite or definition
pub unsafe fn read<T>(ptr: *const T) -> T { ... }

#[safety::checked  { ValidPtr, Aligned, Initialized }] // callsite or discharge
unsafe { read(ptr) };
```

We can also attach comments for a tag or a group of tags to clarify how safety requirements are met:

```rust
for _ in 0..n {
    unsafe {
        #[safety::checked { ValidPtr, Aligned, Initialized =
            "addr range p..p+n is properly initialized from aligned memory"
        }]
        c ^= p.read();

        #[safety::checked { InBounded, ValidNum =
            "`n` won't exceed isize::MAX here, so `p.add(n)` is fine"
        }]
        p = p.add(1);
    }
}
```

[tool attribute]: https://doc.rust-lang.org/reference/attributes.html#tool-attributes
[`ptr::read`]: https://doc.rust-lang.org/std/ptr/fn.read.html

## Discharge Tags

When calling an unsafe function, tags defined by `#[safety::requires]` on it must be present in
`#[safety::checked]` together with an optional reason; any omission triggers a warning-by-default
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

The process of verifying whether a tag is checked is referred to as tag discharge.

Now consider forwarding invariants of unsafe callees onto the unsafe caller for unsafe delegation or
propogation:

```rust
#[safety::requires { ValidPtr, Aligned, Initialized }]
unsafe fn propogation<T>(ptr: *const T) -> T {
    #[safety::checked { ValidPtr, Aligned, Initialized }]
    unsafe { read(ptr) }
}
```

Tags defined on an unsafe function must be **fully** discharged at callsites. No partial discharge:

```rust
#[safety::requires { ValidPtr, Initialized }]
unsafe fn delegation<T>(ptr: *const T) -> T {
    #[safety::checked { Aligned }] // 💥 Error: Tags are not fully discharged. 
    unsafe { read(ptr) }
}
```

For such partial unsafe delegations, please fully discharge tags on the callee and define needed
tags on the caller.

```rust
#[safety::requires {
  ValidPtr, Initialized: "ensure the allocation spans at least size_of::<T>() bytes past ptr"
}]
unsafe fn delegation<T>(ptr: *const T) -> T {
    let align = mem::align_of::<T>();
    let addr = ptr as usize;
    let aligned_addr = (addr + align - 1) & !(align - 1);

    #[safety::checked {
      Aligned: "alignment of ptr has be adjusted";
      ValidPtr, Initialized: "delegated to the caller"
    }]
    unsafe { read(ptr) }
}
```

In this delegation case, you're able to declare a new meaningful tag for ValidPtr and Initialized
invariants, and define the new tag on `delegation` function. This practice extends to partial unsafe
delegation of multiple tag discharges:

```rust
#[safety::declare_tag]
enum MyInvaraint {} // Invariants of A and C, but could be a more contextual name.

#[safety::requires { MyInvaraint }]
unsafe fn delegation() {
    unsafe {
        #[safety::checked { A: "delegated to the caller's MyInvaraint"; B }]
        foo();
        #[safety::checked { C: "delegated to the caller's MyInvaraint"; D }]
        bar();
    }
}
```

Note that discharing a tag that is not defined will raise a hard error.

## Safety Tags are Part of an Unsafe Function

Tags are extra information of unsafe functions, so rustdoc can render documentation of tags,
displaying each tag and its optional description below function's doc. Rust-Analyzer can also offer
**full IDE support**: completion, go-to-definition, and doc-hover.

Tags constitute a public API; therefore, any alteration to their definition must be evaluated
against [Semantic Versioning][semver].
* Adding a tag definition is a **minor** change.
* Removing a tag definition is a **major** change. Renaming a tag definition is a two-step
  operation of removal and addition, bringing a major change due to removal. 

[semver]: https://doc.rust-lang.org/cargo/reference/semver.html

To give dependent crates time to migrate an outdated tag definition, use `@deprecated` in tag
description. Clippy will emit a deprecation warning whenever the tag is used in `safety::checked`.

```rust
#[safety::requires {
  NewTag = "description",
  Tag = "@deprecated explain why this tag is discouraged or what tag shoud be used instead",
}]
unsafe fn deprecate_a_tag() {}

// warning: Tag is deprecated, copy description to here.
// error: NewTag is not discharged.
#[safety::requires { Tag }]
unsafe { deprecate_a_tag() }
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Unstable Features

Currently, safety tags requires the following features
* `#![feature(proc_macro_hygiene, stmt_expr_attributes)]` for tagging statements or expressions.
* registering `safety` tool in every crate to create safety namespace.

Since the safety-tag mechanism is implemented primarily in Clippy and Rust-Analyzer, no additional
significant support is required from rustc.

But we ask the libs team to adopt safety tags for all public `unsafe` APIs in libstd, along with
their call sites. To enable experimentation, a nightly-only library feature
`#![feature(safety_tags)]` should be introduced and remain unstable until the design is finalized.

## Implementation in Clippy

Procedure:

1. Validate `#[safety::requires]` only appears on unsafe functions if the attribute exists.
2. Validate `#[safety::checked]` on HIR nodes whose `ExprKind` is one of
   - **direct unsafe nodes**: `Call`, `MethodCall` that invoke an unsafe function/method, or
   - **indirect unsafe nodes**: `Block` (unsafe), `Let`, `Assign`, `AssignOp`.

   Algorithm for every function body:
   1. Walk the HIR; whenever an unsafe `Call`/`MethodCall` is encountered, record the unsafe callee
      and its nearest ancestor that is an *indirect unsafe node*.
   2. If the callee is annotated with safety tags, require that **either** the call itself **or**
      its recorded ancestor carries `#[safety::checked]`.
   3. Any node that carries `#[safety::checked]` must contain **exactly one** unsafe call/method;
      otherwise emit a diagnostic. *(We intentionally stop at this simple rule; splitting complex
      unsafe expressions into separate annotated nodes is considered good style.)*
   4. Make sure tags in `#[safety::checked]` correspond to their definitions.
   5. Diagnostics are emitted at the current Clippy lint level (warning or error).

[HIR ExprKind]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/hir/enum.ExprKind.html

Libraries in `rust-lang/rust` must enforce tag checking as a hard error, guaranteeing that every tag
definition and discharge is strictly valid.

## Implementation in Rust Analyzer

Safety-tag analysis requirements: offer tag name completion, go-to-definition and inline
documentation hover in `#[safety::checked]` as per tag definitions on unsafe calls.

Maybe some logics on safety tags like collecting tag definitions need to be extracted to a shared
crate for both Clippy and Rust-Analyzer to use. 

# Drawbacks
[drawbacks]: #drawbacks

Even though safety tags are machine-readable, their correctness still hinges on human review:
developers can silence Clippy by discharging tags without verifying underlying safety requirements.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Rationale for the Proposed Implementation

We argued in the [guide-level-explanation] why safety tags are necessary; here we justify the *way*
to implement them.

1. It's linter's work. Tag checking is an API-style lint, not a language feature. All the
   `#[safety::*]` information is available in the HIR, so a linter (rather than the compiler) is the
   right place to enforce the rules.

2. Re-use the existing linter. We prototyped the checks in [safety-tool], but a stand-alone tool
   cannot scale: project may pin its own toolchain, so we would need one binary per toolchain. We
   already maintain three separate feature-gated builds just for verify-rust-std, rust-for-linux,
   and Asterinas. Once the proposal is standardised, the only sustainable path is to upstream the
   lint pass into Clippy. Project building on stable or nightly toolchains since then will get the
   checks automatically whenever it runs Clippy. Moreover, if `rust-lang/rust` repo has already
   undergone Clippy CI, no extra tooling is required for tag checking on the standard libraries.

3. IDE integration. The same reasoning applies to the language server. A custom server would again
   be tied to internal APIs and specific toolchains. Extending Rust-Analyzer is therefore the only
   practical way to give users first-class IDE support.

We therefore seek approvals from the following teams:

1. **Library team** – to allow the tagging of unsafe operations and to expose tag items as public
   APIs.
2. **Clippy team** – to integrate tag checking into the linter.  
3. **Rust-Analyzer team** – to add IDE support for tags.  
4. **Compiler team** – to reserve the `safety` namespace and gate the feature via
   `#![feature(safety_tags)]` for the namespace and tag APIs in standard libraries.

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
* 2025-01: [RFC: Add safe blocks](https://github.com/rust-lang/rfcs/pull/3768) by Aversefun
  * This is a continum of discussion of 2024-10, focusing on visual granularity.
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
unsafe { ptr::read(elem) }
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

When a tag needs arguments to refine context in discharging, we must decide what its definition
looks like. Unfortunately, this immediately raises design questions:

1. **Argument types**. Which types are allowed in the definition?
2. **Type checking**. Will tag operations `safety::requires` and `safety::checked` type-check
   against these arguments? If so, are we quietly reinventing a full contract system?

I'd like to propose a solution or rather compromise here by trading strict precision for simplicity:

We could allow *any* arguments in tag usage without validation. Tag arguments would still refine the
description of an unsafe operation, but they are never type checked. An example:

```rust
#[safety::requires {
  ValidPtr = {
    args = [ "p", "T", "len" ],
    desc = "pointer `{p}` must be valid for \
      reading and writing the `sizeof({T})*{n}` memory from it"
  }
}]
unsafe fn foo<T>(ptr: *const T) -> T { ... }

#[safety::checked { ValidPtr(p) }] // p will not be type-checked
unsafe { bar(p) }
```

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

## Discharge One Tag from `any = { Option1, Option2 }`

Sometimes it’s useful to declare a set of safety tags on an unsafe function while discharging only
one of them.

For instance, `ptr::read` could expose the grouped tag `any { DropCheck, CopyType }` and then
discharge either `DropCheck` or `CopyType` at the call site, depending on the concrete type `T`.

Another instance is `<*const T>::as_ref`, whose safety doc states that the caller must guarantee
“the pointer is either null or safely convertible to a reference”. This can be expressed as
`#[safety::requires { any = { Null, ValidPtr2Ref } }]`, allowing the caller to discharge whichever
tag applies.

## Entity References and Code Review Enhancement

<a id="entity-reference"></a>

To cut boilerplate or link related code locations, we introduce `#[safety::ref(...)]` which
establishes a two-way reference.

An example of this is [`IntoIter::try_fold`][vec_deque] of VecDeque, using `#[ref]` for short:

[vec_deque]: https://github.com/rust-lang/rust/blob/ebd8557637b33cc09b6ee8273f3154d5d3af6a15/library/alloc/src/collections/vec_deque/into_iter.rs#L104

```rust
fn try_fold<B, F, R>(&mut self, mut init: B, mut f: F) -> R
    impl<'a, T, A: Allocator> Drop for Guard<'a, T, A> {
        #[ref(try_fold)] // 💡 unsafety of ptr::read below relies on this drop impl
        fn drop(&mut self) { ... }
    }
    ...

    init = head.iter().map(|elem| {
        guard.consumed += 1;

        #[ref(try_fold)] // 💡
        #[safety { ValidPtr, Aligned, Initialized, DropCheck =
            "Because we incremented `guard.consumed`, the deque \
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

These `#[ref]` tags act as cross-references that nudge developers to inspect every linked site. When
either end or the code around it changes, reviewers are instantly aware of all affected locations
and thus can assess if every referenced safety requirement is still satisfied.

Clippy can generate a diff-style report that pinpoints every location where changes to referenced
HIR nodes occur between two commits or crate versions, enabling more focused code reviews. To
improve dev experiences, Rust-Analyzer can retrieve every ref sites from a given ref tag object.
