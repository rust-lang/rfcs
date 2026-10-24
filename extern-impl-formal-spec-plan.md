# Extern Impl Formal Specification Plan

This document outlines the plan for improving the formal specification of the extern impl RFC (0000-extern-impl.md).

## Todo List

- [x] Research current state of formal verification tools and tooling
- [ ] Define terminology and graph-theoretic concepts
- [ ] Enumerate and formalize all coherence invariants
- [ ] Analyze generic type parameters and associated types
- [ ] Analyze supertraits and their interaction with impl deps
- [ ] Analyze intrinsic methods (impl blocks without traits)
- [ ] Add performance and complexity analysis section
- [ ] Review and revise other spec restrictions
- [ ] Design and implement new formal specification
- [ ] Write separate Cargo RFC for impl dependency representation

## Proposed Plan

### Phase 1: Foundations (can be done in parallel)

**1a. Terminology audit**
- Review current terms: "defining crate", "implementing crate", "impl deps", "adoption", "view"
- Consider alternatives: "providing crate" vs "defining", "extending crate" vs "implementing"
- Graph theory: "least common dominator" vs "lowest common ancestor" vs "convergence point"
- Consider borrowing from module systems literature (ML, Scala implicits)

**1b. Tool research** ✓ COMPLETED
- Survey formal methods tools with specific criteria:
  - Good at modeling graph structures with constraints
  - Can express reachability/dominance properties
  - Has counter-example generation
  - Reasonably mature tooling
- Create comparison matrix: Alloy vs TLA+ vs Lean vs Dafny vs others

### Phase 2: Formalization (sequential, builds on Phase 1)

**2a. Complete invariant catalog**
Core invariants to formalize:
1. **Coherence**: `∀(trait, type). |implementations| ≤ 1`
2. **Acyclic deps**: `∀c. c ∉ transitive_deps(c)`
3. **Visibility**: `impl(A, trait, type) ⟹ trait ∈ visible(A) ∧ type ∈ visible(A)`
4. **Orphan extension**: Define precisely when an impl is "local enough"
5. **Check responsibility**: Every impl-pair has exactly one checker
6. **Check completeness**: Checker can see all relevant impls
7. **Binary root**: Each binary is the coherence root; multiple independent binaries need not be coherent with each other

**2b. Graph-theoretic formalization**
- Define "checker responsibility" precisely
- Is it the immediate dominator? The least common ancestor in the dep tree?
- Formalize: "crate C must check impls I₁, I₂ iff C is the closest crate where both are visible through distinct direct deps"

### Phase 3: Generics, Associated Types, and Edge Cases (separate deep dive)

This deserves its own analysis because of several complex interactions.

**3a. Generics and Associated Types**

These add complexity because:
- Blanket impls create universal quantification
- Associated types add type-level functions
- Specialization (if/when stabilized) adds partial ordering

**Key questions:**
1. Does "defining crate" for `impl<T> Trait for Vec<T>` mean the crate defining `Trait`, `Vec`, or the impl itself?
2. Do overlap rules change? (I suspect not, but needs proof)
3. How do associated type constraints interact with visibility?

**3b. Supertraits**

Supertraits create implicit dependencies between trait implementations.

**Key questions:**
1. If `trait A: B` and a type T has `impl A` in crate X and `impl B` in crate Y, does this create coherence issues?
2. Does the existing orphan rule already handle this? (Likely yes, but needs verification)
3. With impl deps: can crate X implement A for T if it has an impl dep on the crate defining A, but T's impl of B is elsewhere?
4. Are there any new coherence concerns when supertrait impls are spread across crates?
5. Does the checker responsibility algorithm need to account for supertrait relationships?

**Hypothesis**: Supertraits are probably already covered by the existing overlap checking rules, since the type system requires `impl A for T` to prove `T: B` exists. But this needs formal verification.

**3c. Intrinsic Methods (impl blocks without traits)**

The current proposal focuses on trait implementations. A natural extension is intrinsic methods.

**Current Rust**: Multiple `impl Type` blocks are allowed in the same crate, spreading methods across different modules/files.

**Proposed extension**: Allow `impl Type` blocks across multiple impl crates.

**Key questions:**
1. How much additional complexity does this add to the proposal?
2. Coherence is simpler (no overlap checking needed between intrinsic methods)
3. But visibility becomes more complex: which methods are visible where?
4. Do we need a "defining crate" concept for intrinsic impls?
5. How does this interact with privacy/visibility rules?
6. Does allowing intrinsic impls in impl-dep crates create security/encapsulation concerns?
7. Should this be in the initial proposal or deferred to future work?

**Consideration**: The RFC mentions this as a future possibility. It may significantly complicate the Cargo story (how to declare which crate provides which methods?) and the coherence checking story (though simpler than traits).

**Approach**: Start with simplified model (no generics, no supertraits, no intrinsic methods), then add each dimension incrementally

### Phase 4: Implementation Concerns (can be parallel with Phase 3)

**4a. Performance analysis**
- Best case: O(n) linear scan when no impl deps
- Worst case: Could be O(n²) if every crate must check every pair?
- Space: Need to track impl deps in metadata
- Incremental compilation: Which caches invalidate?

**4b. Restrictions review**
Your specific question about re-exports:
- Current: "You may only implement for types/traits actually defined in my_definitions itself. Re-exports don't count."
- Why this restriction? Probably to avoid ambiguity about which crate is "defining"
- Could lift it if we track def-site precisely in metadata
- Risk: re-export chains could make reasoning harder

### Phase 5: New Formal Spec (depends on 1b, 2a, 2b)

**Recommended approach:**
1. **Start with Forge or Alloy 6** for structural properties and counter-example finding
2. **Use Lean 4 or Dafny** for proving the invariants hold (future work)

**Structure:**
- Module 1: Basic definitions (crates, deps, traits, types, impls)
- Module 2: Visibility and reachability
- Module 3: Original orphan rule (baseline)
- Module 4: Extended orphan rule with impl deps
- Module 5: Checking responsibility assignment
- Module 6: Proof that assigned checks ensure coherence

### Phase 6: Cargo RFC (separate RFC, depends on Phase 2 and Phase 5)

This will require a separate RFC submission to define how Cargo represents impl dependencies. **This is likely quite complex and deserves significant design work.**

**Fundamental architectural challenge: Package vs Crate dependencies**

Currently, Cargo's dependency model is based on **packages**, while impl deps are fundamentally **crate-level** relationships. Key issues:

- **One library per package**: Cargo currently allows only one library crate per package (plus build scripts, tests, binaries)
- **Multiple libraries per package**: We likely want to extend Cargo to allow multiple library crates per package with "friend" relationships (impl deps between them)
- **Package-level deps → crate-level deps**: Need to reconcile Cargo's package dependency resolution with rustc's crate-level impl dependency requirements

**Key challenges:**

1. **Multiple library crates per package**
   - How to declare multiple library crates in `Cargo.toml`?
   - Naming scheme for library crates within a package?
   - Which library is the "main" one (for backwards compatibility)?
   - How do external packages depend on specific library crates within a package?

2. **Friend/impl relationships within a package**
   - Syntax for declaring impl deps between sibling library crates in the same package
   - Do these need special visibility rules?
   - How does this interact with the module system?

3. **Cross-package impl dependencies**
   - How to specify which library crate within a package is the impl dependency?
   - Current proposal: `impl = true` on package dependency, but which crate in the target package?
   - Need syntax for package-level dep → specific crate-level impl dep

4. **Package resolution implications**
   - Do impl deps affect resolution differently than regular deps?
   - Version compatibility: how do semver rules apply to impl deps?
   - Diamond dependencies: what if two packages have impl deps on different versions?

5. **Publishing and ecosystem concerns**
   - Current RFC prohibits publishing crates with impl deps to crates.io
   - Workspace restrictions: impl deps only allowed within workspaces (path dependencies)
   - How to prevent accidental publishing violations?
   - Future: if we allow third-party impls, what are the ecosystem implications?

6. **Metadata and build system integration**
   - What information needs to be in package metadata?
   - How do other build tools (Buck/Bazel) represent this?
   - Passing impl dep information to rustc via `--extern impl:`

7. **Interaction with existing features**
   - Optional dependencies
   - Cargo features and feature flags
   - Dev/build dependencies
   - Target-specific dependencies

8. **Error messages and diagnostics**
   - When impl dependency constraints are violated
   - When someone tries to publish with impl deps
   - When cross-workspace impl deps are attempted

**Research needed:**
- Review other Cargo RFCs that impact dependency resolution
- Study how other build systems handle sub-package dependencies
- Analyze ecosystem impact of multiple libraries per package
- Consider backwards compatibility path

## Grouping Strategy

**Can group together:**
- 1a + 1b (terminology and tools inform each other)
- 2a + 2b (invariants and graph theory are intertwined)
- 4a + 4b (both implementation concerns)

**Must separate:**
- Phase 2 before Phase 5 (need clear invariants before formalizing)
- Phase 3 separate (generics are complex enough to deserve isolated treatment)
- Tool selection (1b) before new spec (Phase 5)

**Critical path:** 1b → 2a/2b → Phase 5

## Specific Recommendations

**Graph theory term:** The correct term is likely **"immediate dominator"** or **"lowest common dominator"** - the first node in the dependency tree that dominates (can reach) all the implementing crates. In tree terms, it's the "lowest common ancestor" but in a DAG it's the dominator.

**Re-exports restriction:** Propose relaxing this to "must be defined-or-reexported in the impl-dependency crate" with clear def-site tracking. The restriction seems overly conservative.

## Formal Verification Tools Research Summary (2024)

### Current State of Tooling

**Good news**: Most formal verification tools now have decent VSCode integration, addressing concerns about Alloy's integrated-only environment.

### Tool Comparison Matrix

| Tool | IDE Support | Strengths for This Use Case | Weaknesses | 2024 Status |
|------|-------------|------------------------------|------------|-------------|
| **Alloy 6** | VSCode extensions available (multiple options) | ✓ Perfect for relational/graph properties<br>✓ Excellent counter-example finding<br>✓ Concise transitive closure<br>✓ Uses Kodkod SAT solver | - Not for formal proofs<br>- Bounded model checking only | Active (6.2.0 released Jan 2025) |
| **Forge** | Web-based + local tooling, Sterling visualizer | ✓ Based on Alloy with improvements<br>✓ Better visualization<br>✓ More user-friendly | - Teaching-focused<br>- Smaller community | New (OOPSLA 2024) |
| **TLA+** | Official VSCode extension, very active | ✓ Great for protocols/invariants<br>✓ Strong community | - Less natural for pure graph properties<br>- Steeper learning curve | Very active, "Nightly" builds |
| **Dafny** | Excellent VSCode integration, auto-verification | ✓ Can prove implementations correct<br>✓ Great IDE experience<br>✓ Verification-aware language | - Requires writing actual code<br>- Overkill for structural properties | Very active, LSP-based |
| **Lean 4** | VSCode + LSP, best-in-class | ✓ Most powerful prover<br>✓ Excellent tooling<br>✓ Strong metaprogramming | - Steep learning curve<br>- Overkill for this need | Extremely active (DeepMind using it) |
| **Coq** | Two VSCode options (VsCoq official, coq-lsp alternative) | ✓ Mature ecosystem<br>✓ Good for proofs | - Complex for graph properties<br>- Not ideal for counter-examples | Active (renamed to Rocq) |
| **Isabelle** | VSCode extension, but jEdit primary | ✓ Very mature<br>✓ Strong automation | - Tooling not as polished as Lean<br>- Learning curve | Active but VSCode is secondary |

### Recommendation for Specific Needs

Based on requirements (structural graph properties, counter-examples, fixed dependency graphs):

**Primary tool: Forge or Alloy 6**
- **Forge** if you want better visualization and modern UX (published 2024)
- **Alloy 6** if you want the established tool with more resources
- Both now have VSCode support
- Both use Kodkod's SAT solver underneath (same engine)
- Both excel at relational constraints on graphs

**For formal proofs later: Lean 4 or Dafny**
- **Lean 4** has the best tooling ecosystem in 2024, but significant learning investment
- **Dafny** is more pragmatic, easier to learn, good IDE integration

### Specific Findings

**Alloy improvements:**
- Alloy 6.2.0 (Jan 2025) added mutable state, temporal logic, improved visualizer
- Multiple VSCode extensions now available
- Still uses the integrated Alloy Analyzer, but VSCode extensions bundle it

**Forge (new discovery):**
- Brown University's modern take on Alloy (2024)
- Better visualization with Sterling
- Language levels for gradual complexity
- Open source: https://github.com/tnelson/Forge

**P language:**
- Good for state machines and protocols
- Less suited for pure structural properties than Alloy
- Overkill for static graph problem

**Amazon's formal methods stack (from https://queue.acm.org/detail.cfm?id=3712057):**
- TLA+ for early system design and protocol verification
- P for modeling distributed systems (more programmer-friendly)
- Cedar for authorization policy language
- Dafny for implementation verification
- Kani for Rust code verification

### Final Recommendation

1. **Try Forge first** - it's Alloy with 10+ years of UX improvements, published 2024
2. **Fall back to Alloy 6** if Forge is too teaching-focused or immature
3. **Use VSCode extension for either** to avoid the integrated environment issue
4. **Keep Lean 4 or Dafny in mind** for future formal proofs, but don't start there

The original instinct about Alloy being right for the problem was correct, and the tooling situation has improved significantly.

## What the Alloy/Forge Spec Should Look Like

The problem is purely structural: given a fixed DAG with properties on nodes (definitions, implementations) and edges (deps, impl-deps), can you prove local constraints imply global invariants?

### Basic Structure

```alloy
sig Crate {
    deps: set Crate,       // regular dependencies
    impl_deps: set Crate,  // impl dependencies (subset of deps)
    impls: Trait -> Type,  // implementations in this crate
}

sig Trait { def_crate: one Crate }
sig Type { def_crate: one Crate }

// impl_deps must be subset of deps
fact { all c: Crate | c.impl_deps in c.deps }

// Acyclic (build system enforces)
fact { all c: Crate | c not in c.^deps }

// One binary root
one sig Binary extends Crate {}
fact { all c: Crate - Binary | c in Binary.^deps }
```

Then define:
- **Local orphan constraint**: what each crate can implement given its deps/impl_deps
- **Checker responsibility**: which crate must check which impl pairs
- **Prove**: local checks at each crate ⟹ global coherence for Binary

### Key Insight

Since each binary has independent coherence:
- Alloy finds counter-examples for a single binary's dep graph
- The property is: "for any dep graph rooted at a Binary, local rules ensure coherence"
- Multiple binaries = multiple separate Alloy checks (they don't interact)

## Current Alloy Spec Problems

The existing spec in the RFC has conceptual issues, not tool-choice issues:
1. Doesn't model impl-deps as distinct from regular deps
2. Doesn't formalize "checker responsibility"
3. Has a tautology in `dep_coherent_impl_crates` check
4. Doesn't capture "which crate is responsible for checking coherence"
5. Incomplete - has TODO for the most important constraint
