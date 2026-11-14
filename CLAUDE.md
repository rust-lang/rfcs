# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

This is the Rust RFC (Request for Comments) repository. RFCs are design proposals for substantial changes to Rust, Cargo, Crates.io, or the RFC process itself.

## Current Work Context

The current focus is on **RFC 0000-extern-impl** (`text/0000-extern-impl.md`), which proposes relaxing the orphan rule while maintaining coherence through "impl dependencies."

### Key RFC Concept

The extern impl RFC addresses a fundamental constraint in Rust: the orphan rule requires that trait implementations be in the same crate as either the trait or type. This RFC proposes allowing crates to have "impl dependencies" where they can implement traits/types from dependencies as if they were local, while maintaining coherence through distributed checking.

### Active Work

**Formal Specification Plan**: See `extern-impl-formal-spec-plan.md` for the comprehensive plan to improve the RFC's formal specification. This includes:

1. **Tool selection**: Research concluded that **Forge** (modern Alloy variant) or **Alloy 6** are the best tools for modeling the structural graph properties
2. **Outstanding work phases**:
   - Define precise terminology (especially graph-theoretic terms like "immediate dominator")
   - Enumerate all coherence invariants (coherence, acyclicity, visibility, checker responsibility, etc.)
   - Analyze how generics and associated types interact with the system
   - Add performance/complexity analysis
   - Review restrictions (e.g., the prohibition on re-exported definitions may be too conservative)
   - Build a complete formal specification in Alloy/Forge

3. **Key invariant**: Each binary is an independent coherence root. Multiple binaries need not be coherent with each other.

### Current Alloy Spec Issues

The existing Alloy specification in the RFC (Appendix) is incomplete and has several problems:
- Doesn't model impl-deps as distinct from regular deps
- Doesn't formalize "checker responsibility" 
- Contains a tautology in the `dep_coherent_impl_crates` check
- Has a TODO for the most critical constraint

## RFC File Structure

- `text/` - Contains all accepted and proposed RFCs
- `text/0000-extern-impl.md` - The extern impl RFC being worked on
- `0000-template.md` - Template for new RFCs
- `README.md` - RFC process documentation

## Working with RFCs

RFCs follow a specific structure defined in `0000-template.md`:
- Summary
- Motivation  
- Guide-level explanation
- Reference-level explanation
- Drawbacks
- Rationale and alternatives
- Prior art
- Unresolved questions
- Future possibilities

## Git Workflow

Current branch: `extern-impl`

This is a working branch for the extern impl RFC. The RFC is still in draft/revision phase.

## Next Steps

Based on the formal specification plan, the immediate next steps are:

1. Define precise terminology and graph-theoretic concepts
2. Enumerate and formalize all coherence invariants 
3. Decide between Forge (modern, better UX) vs Alloy 6 (more established) for the formal spec
4. Build out the formal specification incrementally

## Key Concepts to Understand

- **Orphan rule**: Current requirement that trait impls must be in the same crate as the trait or type
- **Coherence**: The property that there's at most one impl of a trait for a given type
- **Impl dependency**: The proposed new dependency type where a crate can implement traits/types from dependencies
- **Checker responsibility**: Which crate is responsible for verifying coherence between multiple implementing crates
- **Binary root**: Each binary crate is the root of its own coherence domain
