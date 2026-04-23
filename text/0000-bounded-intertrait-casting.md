- Feature Name: `bounded_intertrait_casting`
- Start Date: 2025-11-26
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Safe, constant-time, minimal-space-overhead casting between trait objects
that share a common root supertrait. A *bounded* trait graph is one rooted
at a single explicitly-declared supertrait; that root names the closure
of traits a cast may target, so the compiler can compute a per-type
metadata table globally and resolve each cast with two loads and a
branch. The user-facing surface is a `cast!(in dyn Root, expr => dyn U)`
macro (plus `try_cast!` and `unchecked_cast!` variants) that works for
references, `&mut`, and owned `Box`/`Rc`/`Arc`. Unlike ecosystem
solutions, casting does not require `'static`, global registries, or
`TypeId`, and remains correct across crate boundaries and generic
instantiations.

```rust
pub trait Root: TraitMetadataTable<dyn Root> {}
pub trait Sub: Root { fn greet(&self); }

let r: &dyn Root = /* … */;
match cast!(in dyn Root, r => dyn Sub) {
    Ok(s)  => s.greet(),                      // r implemented Sub
    Err(_) => { /* r did not implement Sub */ }
}
```

# Motivation
[motivation]: #motivation

Rust's trait objects enable powerful abstraction and dynamic polymorphism, but today the language lacks a safe, principled, and efficient mechanism for converting between related trait objects in non-trivial trait hierarchies. In practice, large Rust codebases routinely define families of interrelated traits where a single concrete type implements multiple traits that conceptually belong to the same behavioral "graph." In these situations, it is natural to want conversions such as:

* converting `&dyn TraitA` to `&dyn TraitB`
* converting up and down within a bounded trait hierarchy
* performing these conversions without `'static` constraints, runtime registries, or bespoke machinery

Today, that is not something Rust can express safely or ergonomically.

Ecosystem solutions exist, but they all share fundamental drawbacks. They rely on global registries, dynamic maps, `TypeId` lookups, or user-maintained metadata. These approaches introduce runtime dependencies, require correct registration discipline, and impose performance and optimization penalties. They are rarely constant-time, often force `'static` lifetimes, interact poorly with generics, and are fragile across crate boundaries.

Meanwhile, the compiler already possesses the global knowledge required to solve this problem correctly. After monomorphization, the compiler effectively knows:

* every type implementing a particular root trait
* every trait reachable from that root
* the layout and identity of the corresponding vtables

However, Rust currently lacks a mechanism to safely expose and leverage this information for inter-trait casting.

This RFC proposes a language-level facility for bounded inter-trait casting, rooted at an explicitly declared "super trait." For all types participating in a given hierarchy, the compiler computes global, per-type metadata describing which traits are implemented and how to reach them. This enables:

* constant-time, optimizer-friendly checked casts between trait objects sharing a root supertrait
* no runtime registries, no global maps, no user-maintained state
* cross-crate correctness and stability, driven by the compiler's global view
* full lifetime correctness, rather than `'static`-only casting
* support for generics, multiple supertraits, and complex trait graphs

Conceptually, this capability fills the same niche as `dynamic_cast` in C++ or interface casting in JVM languages, but is designed for Rust's compilation and trait systems. It enables richer trait hierarchies, more flexible dynamic polymorphism, and more expressive API design, while remaining consistent with Rust's zero-cost abstraction principles.

In short: developers already want inter-trait casting, and today's ecosystem solutions prove demand but are fundamentally constrained. This RFC provides a sound, efficient, and language-supported path to make inter-trait casting a first-class capability in Rust.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Rust lets you declare a trait as the *root* of a bounded trait hierarchy.
Every trait that transitively inherits from that root forms a *trait graph*,
and every type that implements the root is a member of the graph. Within a
graph, the `cast!` macro converts between trait-object references (and
owned trait objects in `Box`, `Rc`, `Arc`) in constant time, returning
`Err` when the target trait is not implemented or the cast would violate
lifetime erasure.

A root supertrait is declared by naming itself in a `TraitMetadataTable`
supertrait bound:

```rust
pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
```

The self-referential shape — the trait's own `dyn` type appearing in
its supertrait bound — is what marks `SuperTrait` as a root and makes
its metadata table visible to casts scoped to `dyn SuperTrait`. This
RFC's prose sometimes writes it as `TraitMetadataTable<dyn Self>` as
shorthand for "`TraitMetadataTable<dyn X>` where `X` is the root being
declared"; `dyn Self` is not valid Rust syntax and does not appear in
the actual declaration.

Any trait that names `SuperTrait` (directly or transitively) as a supertrait
joins the graph rooted at `SuperTrait`. A small running example:

```rust
pub trait Trait1: SuperTrait { }
pub trait Trait2: SuperTrait { }
pub trait Trait3: Trait1 + Trait2 { }

struct S;
impl SuperTrait for S { }
impl Trait1 for S { }
impl Trait2 for S { }
impl Trait3 for S { }

let s: &dyn SuperTrait = &S;

// Downcast to a specific subtrait:
let t1 = cast!(in dyn SuperTrait, s => dyn Trait1).unwrap();

// Chain casts: once you have a &dyn Trait1, you can jump to a sibling or
// descendant without going back through the concrete type.
let t3 = cast!(in dyn SuperTrait, t1 => dyn Trait3).unwrap();

// Missing impls return Err, not panic:
struct Loner;
impl SuperTrait for Loner { }
let l: &dyn SuperTrait = &Loner;
assert!(cast!(in dyn SuperTrait, l => dyn Trait1).is_err());
```

Three properties drive the design:

* **Bounded by the root.** A trait outside the graph (no transitive path to
  `SuperTrait`) cannot appear as a cast target; attempting it is a
  compile-time error.
* **Graph-wide, not pairwise.** `cast!` takes the root as context —
  `cast!(in dyn SuperTrait, …)` — because the metadata table is per-root.
  Any two traits that share a root can cast between each other without
  declaring a direct relationship.
* **Constant time.** The macro lowers to two loads, an integer multiply,
  and a null-check branch. No registries, no `TypeId`, no `'static`
  requirement.

An exhaustive four-type / six-trait matrix exercising these properties is
in *Appendix A: Trait-graph worked examples*.

## Multiple roots

A type may participate in more than one graph by implementing multiple root
supertraits. Every cast is scoped to exactly one root, so casts between
disjoint graphs are a compile-time error. A trait whose supertrait chain
reaches *both* roots can be used as a cast target from either:

```rust
pub trait SuperA: TraitMetadataTable<dyn SuperA> { }
pub trait SuperB: TraitMetadataTable<dyn SuperB> { }

pub trait ATrait: SuperA { }
pub trait BTrait: SuperB { }
pub trait Shared: ATrait + BTrait { }

// COMPILE ERROR: ATrait and BTrait have no common root.
//   cast!(in dyn SuperA, some_a => dyn BTrait)
//
// OK: Shared is reachable from both SuperA and SuperB.
//   cast!(in dyn SuperA, some_a => dyn Shared)
//   cast!(in dyn SuperB, some_b => dyn Shared)
```

A type that implements both roots has two metadata tables — one per root —
and casts consult the one matching the `in` clause. Worked example with
sharing and partial implementations: *Appendix A: Multiple roots*.

## Generic roots

A generic root is monomorphized like any other trait: `dyn SuperTrait<u8>`
and `dyn SuperTrait<u16>` are distinct roots with distinct graphs. A
subtrait fixed over a concrete parameter (`Trait1: SuperTrait<u8>`) joins
only the matching root; a subtrait generic in the same parameter
(`Trait2<T>: SuperTrait<T>`) joins whichever root shares its instantiation.
See *Appendix A: Generic roots*.

## Lifetimes

The core rule is **erased lifetimes stay erased.** When a concrete
`C<'a, ...>` is coerced to `&dyn SuperTrait`, any lifetime parameter
of `C` that does not appear in `SuperTrait`'s signature (methods,
associated types, supertrait bounds) is existentially hidden behind
the trait object. The lifetime still bounds the underlying value, but
the trait-object type has no way to refer to it. A later cast to a
subtrait must not invent a fresh binding for it.

The unsound pattern this rules out — caller picks `'b`, downcasts,
reads a `&'b T` whose real lifetime was `'a`:

```rust
pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
pub trait Trait1<'a>: SuperTrait {
    fn f(&self) -> &'a u8;
}
struct S1<'a> { a: &'a u8 }
impl<'a> SuperTrait for S1<'a> { }
impl<'a> Trait1<'a> for S1<'a> {
    fn f(&self) -> &'a u8 { self.a }
}

fn inner<'a, 'b>(s: &(dyn SuperTrait + 'a)) -> &'b u8 {
    // Rejected: `'a` was erased on the way into `dyn SuperTrait`,
    // so `Trait1<'b>` cannot be reselected with a fresh `'b`.
    cast!(in dyn SuperTrait + 'a, s => dyn Trait1<'b> + 'a).unwrap().f()
}
```

The formal statement — every lifetime of a subtrait must be expressible in
terms of the root's lifetimes, and relationships between lifetimes must be
preserved across erasure — is in *Reference-level explanation: Lifetime
Erasure*. All bound lifetimes participate, including lifetimes that only
appear through associated-type bindings such as `dyn Sub<Assoc = &'a T>`.

### `'static` is special in trait selection

Trait type parameters are invariant, so `SubTrait<'static>` and
`SubTrait<'a>` are genuinely different trait-object types. Casts honor
that distinction:

* A value whose concrete type only implements `SubTrait<'static>` is not
  castable to `SubTrait<'a>` for non-`'static` `'a`, and vice versa.
* An impl written as `impl<'a> SubTrait<'a> for S<'static>` effectively
  satisfies `for<'a> SubTrait<'a>` and casts to any instantiation.
* An impl written as `impl<'a> SubTrait<'static> for S<'a>` casts only
  to `SubTrait<'static>`, regardless of the concrete lifetime of `S`.

The full matrix of these cases is worked out in *Appendix A: Lifetime
selection*.

### Relationships between lifetimes

Impls may carry outlives predicates (`where 'b: 'a`) that turn the impl
into a selection predicate. Casts preserve these: an impl guarded by
`where 'b: 'a` is admissible only when the caller can prove that relation
at the call site. Two structs with identical type signatures but different
impl predicates therefore produce different cast behavior — see
*Appendix A: Multiple lifetimes*.

## Cross-crate boundaries and cdylibs

The *global crate* is the artifact where trait-graph layout is finalized
— typically a binary, staticlib, or cdylib (i.e., the crate that can see
the full monomorphized trait graph). Every such artifact computes its own
layout independently and tags its metadata tables with a unique identity.

In short: casts never cross global-crate boundaries, even when the trait
and struct definitions are literally identical on both sides. A cast
whose source object and call site carry different identities returns
`Err(TraitCastError::ForeignTraitGraph)`.

Why this restriction is load-bearing: two independently built cdylibs `A`
and `B` that depend on a shared library `C` each compute their own layouts
in isolation. The index `A` assigns to `ATrait` may collide with the
index `B` assigns to `BTrait`. A loader that passed a `B`-built object
into an `A`-built cast would, absent the identity check, silently read off
the wrong slot. The identity comparison rejects such casts regardless of
any index coincidence.

The deeper reason a shared schema cannot be precomputed in `C` is that the
trait graph is *lazily monomorphized*: `dyn Trait2<DownstreamType>` does
not exist from `C`'s point of view until a downstream crate instantiates
it. No precomputation in `C` can fix a canonical layout that covers all
future instantiations downstream crates might invent. A dynamic registry
would have to codegen new vtables at runtime — effectively shipping a
subset of the compiler — so this RFC rejects that path.

Consequently, casts are rejected across global-crate boundaries even when:

* the root trait is defined in a shared crate (like `C` above),
* the object layout is the same concrete type compiled into both
  artifacts, and
* the traits on both sides are literally the same definition.

A worked cdylib example reproducing the failure mode end-to-end is in
*Appendix B: Cross-crate cdylib example*.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This section defines the user-facing contract in two sweeps. First the
definitions, core types, and intrinsics that expose the per-root layout;
then the cast semantics themselves — the `TraitCast` trait, the `cast!`
family of macros, the lifetime-erasure rules, the metadata-table
structure observed by a cast, and when cast-site codegen is finalized —
followed by diagnostics. The implementation machinery that realizes this
contract — delayed codegen, the `call_id` chain,
`GenericArgKind::Outlives`, and the global-phase queries that assemble
the metadata tables — lives in *Appendix C: Implementation sketch
(non-normative)*. A conforming implementation may differ in any of those
details as long as it preserves the semantics below.

## Definitions

Supertrait: `trait Subtrait where Self: Supertrait {}` only. Does not include blanket traits over `T: Supertrait`.

Root supertrait: the minimum/top supertrait that a type must implement to be considered a valid instance of a trait graph.
In all the examples in this RFC, `SuperTrait` is the root supertrait.

Outlives class: a unique class per subtrait, which encodes the impl-selection
sensitive region relationships that are non-uniform over all types implementing
the subtrait.

Concretely, two impls of the *same* subtrait can have different region
requirements that decide whether the impl is selectable at a given call
site. Each such requirement — or the absence of any — forms a distinct
outlives class:

```rust
trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
trait Sub<'a, 'b>: SuperTrait { }

// Two outlives classes for `Sub<'a, 'b>`:
//   class C0: no predicate (always admissible)
//   class C1: 'b: 'a     (admissible only when the caller can prove it)
impl<'a, 'b> Sub<'a, 'b> for S0<'a, 'b> { }                     // class C0
impl<'a, 'b> Sub<'a, 'b> for S1<'a, 'b> where 'b: 'a { }        // class C1
```

A cast target `dyn Sub<'a, 'b>` picks a class based on the outlives
relations known at the cast site. Types whose impl satisfies that class
are castable; others fall through to `Err`. The full layout rules are in
*Metadata Table / Table Entries*.

### Global crate

Introduced in *Cross-crate boundaries and cdylibs* above. The crate
that represents the point at which type-system information is
maximal: no downstream or sibling crate can add new traits or new
monomorphizations of upstream traits to the trait graph.

The trait graph is *lazy*: only traits that appear as a cast target
are included.

Exactly one crate in a compilation is designated as the global crate.
By default the designation is driven by crate type:

* **Global by default:** binaries, staticlibs, and cdylibs — artifacts
  that close the trait graph for their consumers.
* **Not global by default:** rlibs, dylibs, proc-macros, and sdylibs —
  artifacts intended to be composed into a later global crate.

The default must be overridable at compile time so that non-standard
artifacts (for example, a dylib loaded via `dlopen` and known to
bootstrap statically) can opt in or out. Multi-artifact build drivers
(Cargo and friends) drive this through existing crate-type selection.

Each global crate is tagged with a unique identifier, in the form of
a unique address, which is used to identify the trait metadata tables
and indices used by that crate. See *Identity tokens* below for the
contract these addresses carry and the backend obligation that keeps
the per-global-crate uniqueness property intact through codegen, LTO,
and linking.

The default policy is deliberately conservative: it guarantees that
the metadata tables and indices are present for linking purposes even
in programs that might *in theory* admit a more permissive
global-crate choice. For example, a Rust codegen crate loaded via
`dlopen` with a large amount of host-process shared code could, in
theory, be compiled in with respect to casting; making that work
requires changes to the compiler that are out of scope here, so this
RFC does not propose changes to the Rust codegen ecosystem and will
not affect compatibility with external codegen crates. Plugin
architectures are in tension with ahead-of-time optimization, and
this RFC prefers the latter.

The rustc-internal surface that exposes global-crate status
(`tcx.is_global_crate()` and the `-Z global_crate=yes|no` override)
is described in *Appendix C §C.0*.

## TraitMetadataTable
[trait-metadata-table]: ##trait-metadata-table

`TraitMetadataTable` is the opt-in marker by which a trait becomes a
root supertrait. A user declares

```rust
pub trait Root: TraitMetadataTable<dyn Root> {}
```

and from that declaration the compiler begins computing a per-root
metadata table for every concrete type that implements `Root`. Users
do not implement `TraitMetadataTable` directly; a blanket impl covers
every `Sized` type, so writing `impl Root for T` is sufficient. The
rustc-internal form of the trait (language-item marker, coinduction
attribute, blanket impl, and cycle-avoidance reasoning) is in
*Appendix C §C.0*.

```rust
/// The table is computed only for the global crate. It is satisfied
/// for every type that implements the root supertrait; `SuperTrait`
/// must be a trait-object type (`dyn Trait`).
pub trait TraitMetadataTable<SuperTrait>: MetaSized
where
    SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>>,
{
    /// The returned slice is a static array of all trait vtables for
    /// this concrete type. Its order is implementation-defined and
    /// unstable, but constant for a given `SuperTrait`. Must not
    /// dereference any part of `self`. (Lowering this to a "virtual
    /// const" rather than a virtual function call is a desired future
    /// optimization; this RFC does not require it.)
    fn derived_metadata_table(&self) -> (&'static u8, NonNull<Option<NonNull<()>>>);
}
```

Four compiler intrinsics expose the per-root layout observed by a
cast. User code reaches them only through the `TraitCast` impl and
the `cast!` macros; they are unstable and not part of the public
surface (see *Stability*). The rustc-internal attributes
(`#[rustc_intrinsic]`, `#[rustc_nounwind]`) are omitted here; see
*Appendix C §C.0*.

```rust
/// Retrieve the index of `Trait`'s vtable in the slice returned via
/// `TraitMetadataTable::derived_metadata_table`. The index includes
/// the outlives-class offset, computed during the global phase from
/// lifetime relationships at the call site. The specific value is
/// implementation-defined and unstable; it is constant for a given
/// `Trait` and `SuperTrait` but not `const fn` because the global
/// computation is required. The `&'static u8` is a per-global-crate
/// identity token, independent of the generic params.
pub unsafe fn trait_metadata_index<SuperTrait, Trait>() -> (&'static u8, usize)
    where SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>> + TraitMetadataTable<SuperTrait>,
          Trait: MetaSized + Pointee<Metadata = DynMetadata<Trait>> + TraitMetadataTable<SuperTrait>;

/// Retrieve the slice returned via
/// `TraitMetadataTable::derived_metadata_table` for the given
/// `SuperTrait`. Calling this intrinsic forces the caller to be
/// delayed until after global monomorphization. The value is
/// constant for a given `ConcreteType` and `SuperTrait` but not
/// `const fn` because the global computation is required.
pub unsafe fn trait_metadata_table<SuperTrait, ConcreteType>() -> (&'static u8, NonNull<Option<NonNull<()>>>)
    where SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>> + TraitMetadataTable<SuperTrait>,
          ConcreteType: Sized + TraitMetadataTable<SuperTrait>;

/// Return the length of the metadata table for the given
/// `SuperTrait`. Separate from the table itself so optimizations can
/// eliminate OoB checks.
pub unsafe fn trait_metadata_table_len<SuperTrait>() -> usize
where SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>> + TraitMetadataTable<SuperTrait>;

/// Return true iff casting to `TargetTrait` (within the graph rooted
/// at `SuperTrait`) is safe with respect to lifetime erasure. Checks
/// that every lifetime in `TargetTrait`'s binder is expressible
/// through `SuperTrait`'s binder and that the concrete outlives
/// relationships at the call site establish equivalence. Resolved
/// during the global phase when generic parameters may transitively
/// contain lifetimes; otherwise resolved earlier. Separated from the
/// table entries to facilitate lifetime binders.
pub unsafe fn trait_cast_is_lifetime_erasure_safe<SuperTrait, TargetTrait>() -> bool
    where SuperTrait: MetaSized + Pointee<Metadata = DynMetadata<SuperTrait>> + TraitMetadataTable<SuperTrait>,
          TargetTrait: MetaSized + Pointee<Metadata = DynMetadata<TargetTrait>> + TraitMetadataTable<SuperTrait>;
```

### Identity tokens

Each trait-cast intrinsic returns a `(&'static u8, …)` tuple whose first
element is an *identity token* for the global crate. Two tokens obtained
within the same global crate compare equal by address; two tokens
obtained from independently compiled global crates must compare unequal.
The value of the dereferenced `u8` is unspecified — only the address is
load-bearing.

Cast-site code uses this to reject foreign-graph casts: it compares the
address returned by `trait_metadata_table` against the address returned
by `trait_metadata_index` and rejects the cast if they differ. This is
the mechanism behind the `ForeignTraitGraph` path described in
*Cross-crate boundaries and cdylibs* in the guide, and it rejects
cross-global-crate casts even when the trait and struct definitions are
literally identical on both sides.

**Backend obligation.** A conforming backend and linker must preserve
token non-equality across every pass that could merge
address-insignificant constants — `unnamed_addr`-style merging in LLVM,
linker ICF, and any analogous cross-compilation-unit deduplication. If
such a pass merges the tokens, the identity check compares equal when
it must not, and a cast that should return `Err(ForeignTraitGraph)` can
instead succeed against the wrong table. Soundness of the
cross-global-crate rejection rests on this obligation.

A build whose backend or linker cannot make this guarantee is not a
supported configuration for this feature. Stabilization on a non-LLVM
backend requires the backend to honor the obligation (or an equivalent
mechanism); see *Unresolved questions / Non-LLVM backend enforcement of
`address_significant`*. The specific mechanism rustc uses today to
satisfy this obligation is non-normative and is described in
*Appendix C §C.6*.

## TraitCast
[trait-cast]: #trait-cast

### Cast contract (declarative)

Whether a given cast returns `Ok` or `Err` is part of the semantic
contract, not an implementation detail. The API and impls that follow
realize this rule; the evolution policy governing it lives under
*Stability / Evolution policy*.

**`checked_cast` returns `Ok` iff all of the following hold:**

1. The source object's trait-metadata-table identity matches the cast
   site's global-crate identity.
2. An impl of the target trait for the concrete type behind the trait
   object is *admissible at the cast site* — the impl's trait-ref
   matches the target binder's instantiation, and every outlives
   predicate on the impl is provable under the caller's outlives
   relationships as recorded by borrow checking.
3. The target trait's binder is erasure-safe under the root
   supertrait's binder (see *Lifetime Erasure or Downcast-Safety*).

Otherwise the cast returns `Err`, with the variant chosen per the
trait definition below. `unchecked_cast` uses the same rule with
clause (3) dropped; the caller shoulders (3) as a safety obligation.

### API

```rust
use core::ptr::{Pointee, DynMetadata};
use core::marker::{MetaSized, PointeeSized};

/// In `core`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum TraitCastError<T> {
  /// This object is from a different global crate than the one
  /// that is performing the cast.
  /// Useful if you'd like to provide a more informative error message.
  /// Note: do not rely on this behavior. It is subject to change.
  ForeignTraitGraph(T),
  /// This object does not implement the specified trait, or the cast does not
  /// satisfy lifetime erasure requirements. 
  UnsatisfiedObligation(T),
}
impl<T> TraitCastError<T> {
  /// Recover the contained, un-casted, value. Does not panic — both variants
  /// carry the original operand so a failed cast can be retried or returned
  /// to the caller unchanged.
  pub fn into_inner(self) -> T {
    match self {
      Self::ForeignTraitGraph(v) | Self::UnsatisfiedObligation(v) => v,
    }
  }
}

/// `I` is the root supertrait.
/// In a future extension, the root supertrait could be implied. Regardless of the specific root supertrait the result of
/// the cast is the same, since the output vtable will be the same after monomorphization
/// (or is essentially user-invisible).
pub trait TraitCast<I: MetaSized, U: MetaSized>: Sized
    where I: Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I>,
          U: Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I>,
{
    type Target;
    /// Attempt to cast `self` to `U`. All trait impl-obligations are enforced,
    /// but lifetime-erasure soundness is not.
    ///
    /// # Safety
    /// The caller must ensure that the cast is lifetime-erasure safe.
    /// Prefer `checked_cast` or `cast` unless you have verified erasure safety
    /// through other means (e.g., lifetime binder implementations).
    ///
    /// Returns Err(TraitCastError::UnsatisfiedObligation) if the cast is not
    /// possible due to unfulfilled generic obligations.
    /// Returns Err(TraitCastError::ForeignTraitGraph) if the cast is not
    /// possible because the object is from a different global crate.
    unsafe fn unchecked_cast(self) -> Result<Self::Target, TraitCastError<Self>>;
    /// Attempt to cast `self` to `U`.
    ///
    /// Returns Err(TraitCastError::ForeignTraitGraph) if the cast is not
    /// possible because the object is from a different global crate.
    /// Returns Err(TraitCastError::UnsatisfiedObligation) if the cast is not
    /// possible due to lifetime erasure requirements or because of unfulfilled
    /// generic obligations.
    fn checked_cast(self) -> Result<Self::Target, TraitCastError<Self>> {
        if !core::intrinsics::trait_cast_is_lifetime_erasure_safe::<I, U>() {
            return Err(TraitCastError::UnsatisfiedObligation(self));
        }
        unsafe { self.unchecked_cast() }
    }
    /// Same as `checked_cast`, but strips TraitCastError::* from the return type.
    fn cast(self) -> Result<Self::Target, Self> {
        self.checked_cast().map_err(TraitCastError::into_inner)
    }
}
impl<'r, T, U, I> TraitCast<I, U> for &'r T
    where I: Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I> + 'r,
          T: MetaSized + TraitMetadataTable<I>,
          U: MetaSized + Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I> + 'r,
{
    type Target = &'r U;
    unsafe fn unchecked_cast(self) -> Result<&'r U, TraitCastError<Self>> {
        unsafe {
            let (obj_graph_id, table) = <T as TraitMetadataTable<I>>::derived_metadata_table(self);
            let (crate_graph_id, idx) = crate::intrinsics::trait_metadata_index::<I, U>();
            if crate_graph_id as *const u8 != obj_graph_id as *const u8 {
                return Err(TraitCastError::ForeignTraitGraph(self));
            }

            let table_len = crate::intrinsics::trait_metadata_table_len::<I>();
            let table: &[Option<NonNull<()>>] =
                &*crate::ptr::from_raw_parts(table.as_ptr(), table_len);

            let (p, _) = (self as *const T).to_raw_parts();
            let Some(Some(vtable)) = table.get(idx) else {
                return Err(TraitCastError::UnsatisfiedObligation(self));
            };
            Ok(&*crate::ptr::from_raw_parts(p, crate::mem::transmute(vtable)))
        }
    }
}

impl<'r, T, U, I> TraitCast<I, U> for &'r mut T
    where I: Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I> + 'r,
          T: MetaSized + TraitMetadataTable<I>,
          U: MetaSized + Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I> + 'r,
{
    type Target = &'r mut U;
    // Body mirrors `&'r T`'s, using `*mut T`, `from_raw_parts_mut`, and a
    // final `&mut *` to rebuild the reference.
    unsafe fn unchecked_cast(self) -> Result<&'r mut U, TraitCastError<Self>> { /* ... */ }
}

/// In `alloc`
impl<'a, T, U, I, A> TraitCast<I, U> for Box<T, A>
    where I: Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I>,
          T: MetaSized + TraitMetadataTable<I> + 'a,
          U: MetaSized + Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I> + 'a,
          A: Allocator,
{
    type Target = Box<U, A>;
    // Body mirrors `&'r T`'s, using `Box::into_raw_with_allocator` and
    // `Box::from_raw_with_allocator` (and re-wrapping on the `Err` paths so
    // the caller gets back the original `Box`).
    unsafe fn unchecked_cast(self) -> Result<Box<U, A>, TraitCastError<Self>> { /* ... */ }
}

/// In `alloc`
impl<'a, T, U, I, A> TraitCast<I, U> for Rc<T, A>
    where I: MetaSized + Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I>,
          T: MetaSized + TraitMetadataTable<I> + 'a,
          U: MetaSized + Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I> + 'a,
          A: Allocator,
{
    type Target = Rc<U, A>;
    // Body mirrors `Box`'s, using `Rc::into_raw_with_allocator` and
    // `Rc::from_raw_in` (and re-wrapping on the `Err` paths so the caller
    // gets back the original `Rc`).
    unsafe fn unchecked_cast(self) -> Result<Rc<U, A>, TraitCastError<Self>> { /* ... */ }
}

/// In `alloc`
impl<'a, T, U, I, A> TraitCast<I, U> for Arc<T, A>
    where I: MetaSized + Pointee<Metadata = DynMetadata<I>> + TraitMetadataTable<I>,
          T: MetaSized + TraitMetadataTable<I> + 'a,
          U: MetaSized + Pointee<Metadata = DynMetadata<U>> + TraitMetadataTable<I> + 'a,
          A: Allocator,
{
    type Target = Arc<U, A>;
    // Body mirrors `Box`'s, using `Arc::into_raw_with_allocator` and
    // `Arc::from_raw_in`.
    unsafe fn unchecked_cast(self) -> Result<Arc<U, A>, TraitCastError<Self>> { /* ... */ }
}
```

The `&'r T` impl above is the canonical body; the other four impls differ
only in the pointer-reconstruction helpers called out in their comments.
These five impls (`&T`, `&mut T`, `Box<T, A>`, `Rc<T, A>`, `Arc<T, A>`) are
the complete set proposed. Impls for `Pin<P>`, raw pointers (`*const T` /
`*mut T`), and `NonNull<T>` are out of scope here; adding them is discussed
in *Future possibilities*.

## Casting macros
[cast-macros]: #cast-macros

```rust
/// In `core`; re-exported in `std`.
/// Attempt to cast `$e` to `$u` in the trait graph of `$i`.
/// Returns Err($e) if the cast is not possible.
#[macro_export]
macro_rules! cast {
    (in $i:ty, $e:expr => $u:ty) => {{
        core::trait_cast::TraitCast::<$i, $u>::cast($e)
    }};
}

/// In `core`; re-exported in `std`.
/// Attempt to cast `$e` to `$u` in the trait graph of `$i`.
///
/// Returns Err(TraitCastError::ForeignTraitGraph) if the cast is not
/// possible because the object is from a different global crate.
/// Returns Err(TraitCastError::UnsatisfiedObligation) if the cast is not
/// possible due to lifetime erasure requirements or because of unfulfilled
/// generic obligations.
#[macro_export]
macro_rules! try_cast {
    (in $i:ty, $e:expr => $u:ty) => {{
        core::trait_cast::TraitCast::<$i, $u>::checked_cast($e)
    }};
}

/// In `core`; re-exported in `std`.
/// Unsafely attempt to cast `$e` to `$u` in the trait graph of `$i`.
///
/// All trait impl-obligations are enforced, but lifetime-erasure soundness is
/// not.
///
/// # Safety
/// The caller must ensure that the cast is lifetime-erasure safe.
///
/// Returns Err(TraitCastError::UnsatisfiedObligation) if the cast is not
/// possible due to unfulfilled generic obligations.
/// Returns Err(TraitCastError::ForeignTraitGraph) if the cast is not
/// possible because the object is from a different global crate.
#[macro_export]
macro_rules! unchecked_cast {
    (in $i:ty, $e:expr => $u:ty) => {{
        core::trait_cast::TraitCast::<$i, $u>::unchecked_cast($e)
    }};
}
```

## Lifetime Erasure or Downcast-Safety

Downcasting via `TraitCast` must not be able to manufacture lifetimes after 
erasure. Informally: after you erase some part of a type's lifetime structure, 
you may not reintroduce a "larger" lifetime when casting down.

The unsound pattern this would permit is:

* Start from a trait object `&dyn SuperTrait` whose vtable was produced from some concrete type `C<'a, ...>`.
* Erase the lifetime parameters of `C` at the supertrait boundary.
* Later, cast that same object to a trait `dyn SubTrait<'b, ...>` and treat it as if the underlying `C<'b, ...>` existed, even when `'b` is not compatible with the original `'a`.

To rule this out, we restrict which trait graphs can participate in `TraitCast` and how erased parameters are tracked:

1. **Region closure of subtraits by the root supertrait**

   For a root supertrait `I` and any subtrait `J` that may appear in `I`'s metadata table,
   every lifetime parameter that can appear in the public interface of `J` (method 
   signatures, associated types, supertrait constraints) must be expressible in terms of
   the lifetime parameters of `I`.

   Concretely, there must exist a mapping from `J`'s region parameters to `I`'s region
   parameters such that, for all legal instantiations, the regions used by `J` do not 
   outlive those used by `I`. Intuitively: the root supertrait's lifetimes form a "closure"
   that bounds all lifetimes flowing through any trait reachable from it, so that erasing
   down to `I` does not lose information necessary to check subtrait lifetime soundness.
   
   This implies, for example, you cannot have a non-generic root:

    ```rust
    pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
    pub trait Trait1<'a>: SuperTrait { ... }
    ```
   participate in a downcast-safe graph, because `SuperTrait` has no region parameters that could bound the `'a` of `Trait1<'a>`.

2. Erased parameters are existentially fixed (no “re-selection” on downcast)

   When a trait object is formed by unsizing a concrete type `C<…>` to a root 
   supertrait `dyn I<…>`, any type/lifetime parameters of `C` that are not 
   present in the public interface of `I` become existentially hidden behind that
   object. After this erasure step, the program must not be able to “choose” new
   instantiations for those hidden parameters by casting down the trait graph.

   Note: this does not modify unsizing.

Together, these restrictions ensure that after unsizing to a root supertrait, any
successful downcast cannot manufacture longer lifetimes than those that existed in
the original concrete value or extend the lifetimes of any references reachable
through that value.

## `trait_cast_is_lifetime_erasure_safe`

The `trait_cast_is_lifetime_erasure_safe` intrinsic is used to check whether
casting to `TargetTrait` (within the graph rooted at `SuperTrait`) is safe
with respect to lifetime erasure. The source trait is irrelevant: it was
already erased to the root during unsizing, so the only question is whether
the root→target binder mapping preserves lifetime identity.
This check is separated from the metadata table entries to facilitate lifetime
binders.

## Outlives evidence after erasure

Generic cast targets (e.g., `dyn SubTrait<'a, T>` where `T` might
transitively contain lifetimes) raise a question the rest of the contract
does not: the outlives-class slot a cast selects and the answer
`trait_cast_is_lifetime_erasure_safe` returns both depend on lifetime
relationships that are only fully known after monomorphization — at which
point lifetimes are normally erased. The contract is:

* MIR regions remain `ReErased`. No path through this feature preserves
  regions in MIR or revives them.
* The cast intrinsics observe the outlives relationships visible *at each
  call site*, not in any global environment. Two call sites with different
  outlives contexts in scope may therefore resolve to different slots or
  produce different erasure-safety results for otherwise-identical casts.
* Soundness does not rest on `ParamEnv`. The outlives evidence that
  selects the slot and answers `trait_cast_is_lifetime_erasure_safe` is
  the call site's own outlives graph, threaded through monomorphization.
* Only functions whose codegen transitively reaches a trait-cast intrinsic
  with outlives-sensitive generic parameters are affected; all other
  codegen proceeds through the existing erased path unchanged.

An implementation sketch — borrowck region summaries, the
`GenericArgKind::Outlives` arg kind, and the call-chain composer that
realize this — is in *Appendix C §C.3*.

## Metadata Table

### Table Entries

Each position in the metadata table corresponds to a pair of 
* the concrete trait instantiation,
* and the outlives relationship graph (determined by the present concrete types
  that query their table and the trait graph).

We need to expand each trait into multiple entries because lifetime
relationships are impl-selection predicates and can be different for different
impls of the trait (ie may be different for each type)

For example:
```rust
trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
trait Trait1<'a, 'b>: SuperTrait { }

struct S1<'a, 'b> {
  // ...
}
impl<'a, 'b> SuperTrait for S1<'a, 'b> { }
impl<'a, 'b> Trait1<'a, 'b> for S1<'a, 'b>
  where 'b: 'a,
{ }
struct S2<'a, 'b> {
  // ...
}
impl<'a, 'b> SuperTrait for S2<'a, 'b> { }
impl<'a, 'b> Trait1<'a, 'b> for S2<'a, 'b>
{ }

// The SuperTrait metadata table layout will need to have three entries:
// 1. The vtable for `SuperTrait`
// 2. The vtable for `Trait1<'a, 'b>`
// 3. The vtable for `Trait1<'a, 'b> where 'b: 'a`

// For a given set of lifetimes, the tables for S1 and S2 would look like this:
//
// [ S1 Table ]                            [ S2 Table ]
// +----------------------------------+    +----------------------------------+
// | 0: vtable for SuperTrait         |    | 0: vtable for SuperTrait         |
// +----------------------------------+    +----------------------------------+
// | 1: None (no base Trait1 impl)    |    | 1: vtable for Trait1<'a, 'b>     |
// +----------------------------------+    +----------------------------------+
// | 2: vtable for Trait1 (if 'b: 'a) |    | 2: vtable for Trait1 (implied)   |
// +----------------------------------+    +----------------------------------+
```

The three-entry layout shown here is the *pre-condensation* view. Layout
applies `impl_universally_admissible` (see the fast-path subsection
below) and condenses outlives classes whose admissible-impl sets are
identical onto shared slots. For traits whose participating impls carry
no per-impl outlives predicates and no Self/trait-param sharing, all
classes collapse into a single slot, so the common case for a real
program is one slot per reachable sub-trait rather than one slot per
`(sub_trait, OutlivesClass)` pair.

This makes the table index encode the trait "ID" as well as an outlives
relationship graph "sub-index".

### Layout

Layout runs only in the global crate and is *implementation-defined* and
unstable; the slot order may be randomly permuted to prevent accidental
dependencies. The contract, expressed as three observable steps:

* **Pruning.** Only casts that actually appear in the program drive
  layout. Sub-traits reachable from the root but not targeted by any
  cast request receive no slot; there is no reserved sentinel index. A
  cast target that the layout has pruned is rejected at compile time
  (see *Diagnostics*).
* **Condensation.** For each reachable sub-trait, outlives classes whose
  admissible-impl sets are identical share a slot. When every
  participating impl passes `impl_universally_admissible` (see below),
  all classes collapse onto one slot. Where-clause-derived outlives
  classes implied by a sub-trait's own `where 'a: 'b`-style predicates
  are folded in so that casts carrying valid outlives evidence through
  generic library code find the right slot.
* **Population.** Per `(root, concrete)` pair, each slot either carries
  a vtable or is `None`. The tables are uniform across all concretes
  feeding a given root, so `None` entries are unavoidable whenever a
  slot is satisfied by at least one concrete type in the graph but not
  by another. At runtime, trait satisfaction is a single branch on
  null.

Traits present in the layout that would violate lifetime-erasure
constraints remain present; `trait_cast_is_lifetime_erasure_safe` guards
against unsafe casts into them, with an unsafe escape hatch for
lifetime-binder implementations.

#### `impl_universally_admissible` fast path

`impl_universally_admissible(impl_def_id: DefId) -> bool` decides whether an
impl's selection is independent of the caller's outlives context — i.e.,
whether the impl is admissible under *every* outlives class for every dyn
binder structure. When every participating impl for a given sub-trait passes
this check, layout skips full per-class admissibility analysis and
collapses all outlives classes for that sub-trait onto a single slot.

The criteria are:

* **(a)** no concrete lifetimes (e.g. `'static`) in the impl's trait ref;
  all trait-ref regions must be `ReEarlyParam` or `ReBound`.
* **(b)** every trait-lifetime position maps to a distinct free impl param
  (no duplicate early-bound regions across trait-ref positions).
* **(c)** no `RegionOutlives` where-clauses whose `longer` or `shorter`
  side is one of the trait-position params collected in (a) + (b).
* **(d)** no trait-position lifetime param also appears in `Self`. A
  Self-anchored param — one that appears *both* in the `SelfTy` and in the
  trait's generic args — is pinned by `Self`-unsizing to the concrete
  value's erased lifetime, but the impl's generic-arg position for it may
  depend on the caller's outlives context; the two can only agree
  universally if the impl forces the param to `'static`. The check takes
  the strict route and rejects any impl that shares a param between
  `Self` and the trait ref.

An inherent impl (no trait ref) is vacuously admissible.

Consequences for code size: when admissibility holds for every participating
impl of every sub-trait in a root's graph, the layout collapses to one slot
per reachable sub-trait, and `trait_metadata_index` call sites all resolve
to that same slot regardless of outlives class — so user functions are not
duplicated by outlives class. Programs whose cast-target impls are free of
per-impl outlives predicates and Self/trait-param sharing are admissible by
construction, which is the common case.

## Delayed codegen and the global crate

Cast-site code cannot be fully codegen'd until the metadata-table layout
is known, and the layout is known only in the global crate. User-visible
consequences:

* An upstream crate that contains cast sites compiles successfully but
  does not emit final code for the cast intrinsics themselves. It
  records each cast site as a delayed request in its rmeta; actual
  codegen happens later.
* The global crate (a binary, staticlib, or cdylib — see *Definitions*)
  consumes every upstream crate's delayed requests, finalizes the
  per-root layout and table population, and emits the final code for
  every cast site in the compilation. This is the single point at which
  the graph is closed.
* A compilation that contains cast sites but no global crate fails at
  link time (see *Diagnostics*). Libraries intended for later linking
  simply produce the delayed-request records and stop.
* Every global crate computes its layout independently. Two
  independently built global-crate artifacts therefore have incompatible
  tables even when the source trait/struct definitions are identical —
  this is the `ForeignTraitGraph` rejection path (see *Cross-crate
  boundaries and cdylibs* in the guide).

## Stability
[stability]: #stability

Stability governs three surfaces independently: the API, the
declarative cast contract, and the implementation underneath.

### API surface (stable)

On stabilization the following become part of the stable surface:

* The `TraitCast` trait and its five impls (`&T`, `&mut T`,
  `Box<T, A>`, `Rc<T, A>`, `Arc<T, A>`).
* The `cast!`, `try_cast!`, and `unchecked_cast!` macros (final paths
  subject to *Unresolved questions*).
* `TraitCastError<T>`, marked `#[non_exhaustive]`. The existing
  `ForeignTraitGraph` and `UnsatisfiedObligation` variants are stable;
  `#[non_exhaustive]` reserves the right to split
  `UnsatisfiedObligation` into finer variants later without breaking
  users who already match exhaustively.

### Cast contract (declarative; stable)

The declarative `Ok`-iff rule that governs every cast is stated under
*TraitCast / Cast contract*. It is part of the stable semantic contract
of this feature; the policy below governs how that rule may evolve.

### Evolution policy

* **`Err → Ok` reversals are permitted.** A cast that returned `Err`
  under rustc `N` may return `Ok` under rustc `N+1`. These arise from
  the same class of changes that turn previously-rejected programs
  into accepted ones — more precise admissibility reasoning,
  NLL/Polonius relaxations, improvements in the outlives solver — and
  flow into cast behavior through clause (2). Crates that rely on a
  cast *failing* as a control-flow signal take on the same exposure
  they already accept for any behavior contingent on the borrow
  checker's precision.
* **`Ok → Err` reversals are breaking** and only permitted as part of
  a soundness fix, on the same footing as any other
  soundness-motivated language change. When such a fix is necessary,
  it follows the standard unsound-feature process (future-incompat
  lint, edition migration, or direct breakage as severity requires).

This mirrors how the trait solver and borrow checker are governed
today: declarative rules are stable, the decision procedure is free
to improve monotonically, and the only permitted breakage is for
soundness.

### Implementation surface (unstable)

The following are implementation-defined and may change at any time,
including within a single stable release series:

* Slot order and index values assigned by `trait_metadata_index`.
* The layout's outlives-class condensation — including which outlives
  classes share a slot and which are collapsed entirely. These
  choices affect code size but not cast success or failure.
* Layout, ordering, and contents of per-`(root, concrete)` metadata
  tables.
* Mangling of augmented `Instance`s and every other internal of the
  delayed-codegen pipeline.

The `core::intrinsics::trait_metadata_*` intrinsics remain unstable
indefinitely; user code reaches this feature exclusively through the
stable `TraitCast` / `cast!` surface.

### Compile-time rejection: pruning

A cast whose target trait is not reached by any cast in the program
is rejected at *compile time* (see *Diagnostics*), not at runtime.
This is part of the stable contract: the rejection is deterministic
given the full program text. The `unused_cast_target` lint handles
the related case of a cast target whose slot exists but is
unreachable (no concrete type implements it).

### Out-of-contract reliance

The declarative cast contract above is the **only** stable contract
this feature provides. Specific index values, address relationships
between `&'static u8` identity tokens, slot adjacency, and every
other observable property of the metadata table are out-of-contract
and may change under any release. Programs must not rely on them.

## Diagnostics

All compile-time diagnostics below are emitted during typeck or trait solving
unless stated otherwise.

### Target trait is not reachable from root supertrait

Emitted when the target trait in a `cast!` expression does not have the root
supertrait as a (transitive) supertrait.

```
error[E0XXX]: `Trait2` is not in the trait graph rooted at `SuperTrait`
 --> src/main.rs:10:5
  |
10|     cast!(in dyn SuperTrait, &s => dyn Trait2)
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: `Trait2` does not have `SuperTrait` as a (transitive) supertrait
  = help: add `SuperTrait` as a supertrait bound on `Trait2`
```

Similarly, emitted when the source trait object type is not in the root's graph.

### Missing `TraitMetadataTable` bound on root supertrait

Emitted when a trait is used as the root supertrait in a `cast!` expression but
does not have `TraitMetadataTable<dyn Self>` as a supertrait bound.

```
error[E0XXX]: `Root` cannot be used as a cast root: missing `TraitMetadataTable` bound
 --> src/main.rs:5:1
  |
5 | pub trait Root {}
  | -------------- `TraitMetadataTable<dyn Root>` is not a supertrait of `Root`
  |
  = help: add a supertrait bound: `trait Root: TraitMetadataTable<dyn Root> {}`
```

### `TraitMetadataTable` type argument must be a trait object

Emitted when a trait declaration names `TraitMetadataTable<T>` as a
supertrait and `T` is not a `dyn Trait` type. The `TraitMetadataTable`
machinery is defined only over trait objects (its blanket impl requires
`T: Pointee<Metadata = DynMetadata<T>>`), so non-`dyn` arguments render
the bound uninhabitable and are never what the author intended.

```
error[E0XXX]: `TraitMetadataTable` type argument must be a trait object
 --> src/main.rs:5:23
  |
5 | pub trait ChildTrait: TraitMetadataTable<u32> {}
  |                       ^^^^^^^^^^^^^^^^^^^^^^^
  |                       |
  |                       `u32` is not a `dyn Trait` type
  |
  = note: `TraitMetadataTable<T>` requires
          `T: Pointee<Metadata = DynMetadata<T>>`, which holds only for
          trait objects
  = help: use `dyn Self` to declare `ChildTrait` as a cast root, or
          `dyn R` for a cast-root supertrait `R` of `ChildTrait`
```

### Mismatched `TraitMetadataTable` type argument

Emitted when a trait declaration names `TraitMetadataTable<dyn X>` as a
supertrait and `dyn X` is neither `dyn Self` (which would declare this
trait as a cast root) nor `dyn R` for a transitive supertrait `R` that
is itself a cast root. Such a bound is satisfiable by the blanket impl
but places the trait in no reachable cast graph, so it is almost always
a user mistake.

```
error[E0XXX]: `TraitMetadataTable` type argument does not match a cast root
 --> src/main.rs:7:25
  |
5 | pub trait Root: TraitMetadataTable<dyn Root> {}
  |     ---- cast root
6 | pub trait Unrelated: TraitMetadataTable<dyn Unrelated> {}
  |     --------- unrelated cast root
7 | pub trait ChildTrait: Root + TraitMetadataTable<dyn Unrelated> {}
  |                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |                              `dyn Unrelated` is not a (transitive)
  |                              supertrait of `ChildTrait`
  |
  = note: on a trait `Tr`, a `TraitMetadataTable<dyn X>` supertrait
          bound requires `X = Self` (declaring `Tr` as a cast root) or
          `X = R` for some transitive supertrait `R` of `Tr` that is
          itself a cast root
  = help: subtraits inherit `TraitMetadataTable<dyn Root>` from their
          root — the explicit bound is usually unnecessary
  = help: if you meant to place `ChildTrait` in `Root`'s graph, write
          `TraitMetadataTable<dyn Root>`; if you meant `ChildTrait` to
          be its own root, write `TraitMetadataTable<dyn ChildTrait>`
```

Both diagnostics are emitted at trait-definition time regardless of
whether any `cast!` expression mentions the trait.

### Lifetime erasure violation (downcast-unsafe trait graph)

Emitted when a subtrait introduces lifetime parameters that are not expressible
in terms of the root supertrait's lifetime parameters.

```
error[E0XXX]: trait graph rooted at `SuperTrait` is not downcast-safe
 --> src/main.rs:8:1
  |
4 | pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> {}
  |           ---------- root supertrait has no lifetime parameters
...
8 | pub trait Sub<'a>: SuperTrait { fn f(&self) -> &'a u8; }
  |               ^^ lifetime `'a` is not bounded by any lifetime on `SuperTrait`
  |
  = note: downcasting to `dyn Sub<'a>` could manufacture lifetimes
          that were erased when unsizing to `dyn SuperTrait`
  = help: add a lifetime parameter to the root: `trait SuperTrait<'a>: ...`
```

This diagnostic is emitted eagerly (at trait definition time) when the root
supertrait is known, rather than only at cast sites, so that library authors
receive the error even if no cast is written in the current crate.

### Non-object-safe trait used as cast target

Emitted when the target trait in a cast expression is not object-safe
(dyn-compatible).

```
error[E0XXX]: `NotObjectSafe` cannot be made into a trait object
 --> src/main.rs:12:5
  |
12|     cast!(in dyn SuperTrait, &s => dyn NotObjectSafe)
  |                     ^^^^^^^^^^^^^ `NotObjectSafe` is not dyn-compatible
```

This reuses the existing object-safety diagnostics.

### Global-phase diagnostics

The following are surfaced during the global codegen phase (after
monomorphization), not during typeck:

**No global crate.** A compilation that contains delayed cast-intrinsic
requests but no global crate is ill-formed. The implementation should
raise a clear diagnostic in the cases it can detect directly — for
example, a final artifact compiled with `-Z global_crate=no`. Cases the
driver cannot distinguish from "library intended for later linking"
(notably standalone dylibs) degrade to an ordinary link-time failure
against the unresolved cast-intrinsic symbols; the naming of those
symbols should be chosen so the linker's message is self-explanatory.

**Unused cast-target trait pruned (lint, off by default).** When a trait
appears as a cast target in a `trait_metadata_index` instantiation but no
concrete type in the final binary satisfies it, the trait's index is set to
unreachable. An optional lint (`unused_cast_target`) can warn about this:

```
warning: cast target `dyn Trait4` is unreachable in the trait graph of `dyn SuperTrait`
 --> src/main.rs:15:5
  |
15|     cast!(in dyn SuperTrait, &s => dyn Trait4)
  |                     ^^^^^^
  |
  = note: no type implementing `SuperTrait` also implements `Trait4`
  = note: this cast will always return `Err` at runtime
  = note: `#[warn(unused_cast_target)]` on by default
```

# Drawbacks
[drawbacks]: #drawbacks

Accepting this RFC commits the language and compiler to a collection
of new surfaces and obligations. This section aggregates them so
reviewers can weigh cost against motivation. Each is a drawback
*relative to the status quo*, not relative to the existing ecosystem
crates the feature replaces.

## Implementation complexity

The design bridges lifetime erasure, monomorphization, and
cross-crate linking. Concretely it adds:

* a new language-item trait (`TraitMetadataTable`) and four compiler
  intrinsics;
* a fourth `GenericArgKind` variant (`Outlives`) with corresponding
  interning, mangling, and type-foldable handling;
* a `call_id` chain threaded through `Call` / `TailCall` terminators
  and preserved across the MIR inliner;
* a global phase that runs after monomorphization in the global
  crate, with its own queries and arena caches;
* a `codegen_mir` query that can be fed a patched MIR body per
  augmented `Instance`;
* new borrowck-side queries (`borrowck_region_summary`,
  `vid_provenance`) whose outputs cross the crate boundary via rmeta;
* a backend-observable `address_significant` flag on allocations.

None of these are individually exotic, but the combination
substantially enlarges the surface area of the compiler's
guarantees. The global-phase machinery in particular is load-bearing
for soundness (see *Identity tokens*), and regressions in it would
manifest as `Ok` / `Err` flips at cast sites rather than as typeck
errors.

## New conceptual surface for users

A user who wants to adopt casting in their own trait hierarchy must
learn, at minimum:

* the root-supertrait opt-in
  (`trait Root: TraitMetadataTable<dyn Root>`);
* the notion of a *bounded* trait graph and why casts are scoped to
  a root;
* the region-closure rule that governs which subtrait shapes are
  admissible under a given root;
* the existential-erasure rule that drives the lifetime-selection
  behavior in *Appendix A.4*;
* the existence and meaning of `ForeignTraitGraph` failures across
  cdylibs.

This is a meaningful teachability cost. Some of the rules
(especially lifetime erasure) are subtle enough that users will
learn them by bouncing off diagnostics. Documentation, diagnostic
quality, and worked examples become part of the stabilization
effort, not follow-on work.

## Diagnostics burden

Lifetime errors are already a leading source of user confusion.
This feature adds three new failure modes that surface as lifetime
errors or their moral equivalents:

* a trait definition may be rejected because its lifetime parameters
  aren't expressible through the root's (region-closure violation);
* a cast may return `Err(UnsatisfiedObligation)` at runtime because
  `trait_cast_is_lifetime_erasure_safe` returned `false` for the
  specific call-site outlives context — a dynamic failure with no
  compile-time counterpart at the cast site;
* a cast may compile and then fail at runtime because the chosen
  outlives class does not match the impl's predicates.

The `unused_cast_target` lint and the global-phase diagnostics help
with the static cases, but the runtime-visible erasure-safety
failure is novel: it is a case where borrow-checker precision
elsewhere in the program affects whether a cast that "looks right"
returns `Ok`. Producing useful diagnostics for that path will take
sustained work.

## Compilation cost

Several axes of extra work land on the compiler:

* **Global phase is serial.** Layout, table population, and final
  codegen for every cast site run in one crate (the global crate)
  after every upstream crate is otherwise done. This is a pipeline
  stall on the critical path of large binaries.
* **Incremental compilation.** Any change to a participating trait,
  impl, or cast target in any crate invalidates the global crate's
  layout and forces the global phase to re-run. Downstream crates
  that previously cached cleanly may recompile because their
  rmeta-recorded `DelayedInstance`s are inputs to the global phase.
* **Cross-compilation caching.** Tools like `sccache` key on
  crate-level work; the global phase straddles crates, which may
  interact poorly with existing cache assumptions. Build systems
  relying on deterministic rmeta hashes for dependency reuse will
  need to extend those hashes to cover the global-phase inputs.
* **Outlives-class code duplication.** When a cast target's impl
  selection depends on lifetime predicates (e.g.
  `impl<'a, 'b> Trait for S<'a, 'b> where 'b: 'a`), user functions
  that reach the cast can be duplicated per outlives class. In
  practice, layout condenses outlives-equivalent classes and
  `impl_universally_admissible` collapses classes entirely in the
  common case — duplication is constrained to CFGs in which
  lifetimes flow into target-trait lifetimes, and vanishes when
  trait casting is not used — but the worst case is real and
  load-bearing to bound.

## Code and data size

* **Casting code size.** Each cast lowers to two loads, an integer
  multiply, and one branch — effectively free. The other branch is
  optimized away in the common case.
* **Additional vtables.** Monomorphization restricts vtables to
  concrete types and traits that actually participate in
  downcasting; unreferenced blanket generic impls are not included.
* **Metadata tables.** One `[Option<NonNull<()>>; N]` per
  `(root, concrete)` pair in the program, with `N` the reachable
  slot count for that root. Pruning keeps `N` tight, but `None`
  entries are unavoidable whenever a slot is satisfied by at least
  one concrete but not by another. A follow-on could shrink entries
  to `Option<NonMaxU32>` with vtables offset from a base, halving
  the table size.
* **Identity tokens.** One extra `unnamed_addr`-suppressed byte
  allocation per global crate. Negligible.

## Ecosystem pressure toward large root supertraits

Because casts only work within a single graph, there is a real
design incentive to place traits under a shared root supertrait even
when they are otherwise unrelated. Authors who prefer narrower
hierarchies can still define multiple independent roots, at the cost
of not being able to cast between them; a trait that needs to be
targetable from two roots must declare both as supertraits (see
*Appendix A.2*). Over time this pressure could calcify into a
preference for a few "god roots" in widely used libraries, in the
same way as trait-object-safe `Error` has become a de facto root for
error types. That may or may not be desirable, but it is a shape
the language does not push toward today.

## Backend and toolchain portability

The identity-token contract (*Identity tokens*) depends on the
codegen backend honoring the per-allocation `address_significant`
flag. LLVM satisfies this via `UnnamedAddr::No`; Cranelift and GCC
have no active address-merging pass today, so the flag is recorded
but not acted on. A future pass on either backend that introduces
ICF-style merging must honor the flag, or an equivalent mechanism.
Non-LLVM backends without such a mechanism cannot soundly host the
stabilized feature (see *Unresolved questions / Non-LLVM backend
enforcement*).

Similarly, the v0 symbol mangler is specified to encode
`GenericArgKind::Outlives`; the legacy mangler has no encoding. Any
fallback to the legacy mangler for augmented `Instance`s is
unsupported (see *Unresolved questions / Legacy symbol mangler*).

## Dependency on other unstable features

The proposed API signatures reference `MetaSized` and
`Pointee<Metadata = DynMetadata<…>>`, both of which are still
evolving. Stabilization of this feature presupposes those are in a
shape compatible with the supertrait bounds as written. A change to
either (for example, `MetaSized` splitting differently, or
`DynMetadata` growing parameters) would force a rework of the trait
signatures here.

## Plugin architectures are second-class

By design, trait casts never cross global-crate boundaries
(*Cross-crate boundaries and cdylibs*). This rules out dynamic
plugin architectures that want to share a single trait graph across
dlopened artifacts — a pattern that works today via ecosystem crates
like `intercast` at the cost of performance and `'static`. Users
with that use case are left without a first-class solution; this
RFC trades their expressive power for constant-time casting and
strict soundness. The trade is deliberate, but it is a trade.

## Interaction with future language directions

* **Dyn upcasting** is already stable via embedded supertrait-vtable
  pointers in each vtable. The metadata-table machinery here could
  in principle subsume it (*Future possibilities / Dyn upcasting*)
  but that direction is speculative and constrained by the stability
  of the existing path.
* **`dyn Trait` composition and negative reasoning.** Future
  features that restrict what impls may exist (e.g. `impl !Trait`,
  specialization) would need to interact with this feature's
  admissibility rules. No blockers are known, but the interaction
  is unspecified.
* **Async trait objects / AFIT / RPITIT.** Casting between async
  trait objects follows the same rules in principle; no specific
  accommodation is made here, and real deployment may surface
  unexpected friction.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Existing solutions to this problem are:
- `intercast` crate: `dyn Trait` to `dyn Trait` casting. Uses a global hashmap to store the trait vtables. Casting is not constant-time and requires virtual dispatch.
- `traitcast` crate: requires AoT knowledge of the trait graph and a runtime type/trait registry. Casting is not constant-time and requires virtual dispatch.

Under the hood, all these crates use `std::any::Any`/`TypeId`: to cast a trait object to another trait object, a two-step process is followed:
- First, the trait object is cast to a raw pointer of the concrete type.
- Then, the raw pointer is cast to the desired trait object type. Rustc attaches the vtable of the desired trait object type to the raw pointer.

However, this approach has a few drawbacks:
* it is not a constant time,
* pessimizes the optimizer due to global lookups and virtual dispatch,
* forces `'static` lifetimes due to `std::any::Any`, and 
* it doesn't work w/ generic traits/types, without also manually monomorphizing the traits/types.

Another approach is possible but does not appear to be implemented in any
published crate: use `rustc_public` to expose the trait implementations and
types. That approach does not allow delayed codegen on its own. It would
require multiple complete compilations of the crates: first to extract the
trait vtables, then a second compilation that could use the built vtable
tables. It would not work cross-crate without additional workarounds.

## `cast!` surface syntax

The `cast!(in $root:ty, $e:expr => $u:ty)` shape is constrained by
`macro_rules!` follow-set rules, not aesthetics. After an `$e:expr` fragment
the grammar only admits `=>`, `,`, or `;` as the next token — so the natural
`$e as dyn U` / `$e as dyn U in dyn Root` forms are not expressible as a
declarative macro. `=>` is the only separator in the admissible set that
reads as a cast arrow, and the leading `in $root,` clause places the root
where it can precede the `$e:expr` (whose follow-set is the binding
constraint) rather than after it.

Alternatives considered:

* **Method form, `e.cast::<Root, U>()`.** Works, but hides the root in a
  turbofish and reads as a method on the pointer type rather than a
  language-level cast. Also loses the visual parallel with `as`-casts.
* **`$e as dyn U` with the root inferred.** Blocked by macro follow-sets as
  above; would require a proc-macro or a built-in construct. A future
  language-level cast could revisit this and infer the root from the
  source's trait-object type, at the cost of an in-compiler surface rather
  than a library macro.
* **Sigil forms (`$e :> dyn U`, etc.).** Same follow-set problem, and
  introduces a new operator-like token without broader justification.

The `in` keyword is reused purely as a macro-internal marker token; it is
not a new contextual keyword and does not appear in the grammar outside the
macro's matcher. A future migration to a built-in cast construct would be
free to drop it.

## Dynamically loaded trait graphs

As stated in the guide, this proposal does not support dynamic trait graphs.

## Lifetime Erasure Avoidance by Casting Directly from `SubTrait1` to `SubTrait2`

Lifetime Erasure rules are defined only for the `SuperTrait` to 
`SubTrait1`/`SubTrait2` path, essentially making all casts downcasts. We have to
do this since table entry obligations are not checkable per-type, only
per-trait-object (i.e., once, i.e., w.r.t. the root supertrait).

The alternative would be to add an expensive check per cast: each cast would 
need to compare a compiler-generated, encoded, lifetime relationship graph of
the lifetimes of the source trait and target trait. The latter of which would 
have to live in the metadata table entries. At minimum, this would require an
extra memcmp, and in full generality, it is equivalent to the rooted graph
isomorphism problem.

## Lifetime Erasure Avoidance by Augmenting the Unsize Site

A symmetric alternative to the rule in *Lifetime Erasure or
Downcast-Safety* ("region closure of subtraits by the root supertrait")
would push the closure obligation onto the *unsize site* rather than
the root. At a coercion
`C<'a, ...> -> &dyn SuperTrait`, the compiler knows the concrete type's
lifetime parameters and whatever outlives relations hold at that program
point. Those relations could be captured on the unsizing as augmented
`Instance` data — the same machinery used for impl-selection outlives
classes on subtrait impls (see *Metadata Table / Table Entries* and
*Appendix C §C.3*) — and fold into vtable/table selection. Downcasts
then succeed only against table variants the unsize site certified.

The user-visible effect is that the current restriction disallowing
non-generic roots with region-generic subtraits (e.g. the
`Trait1<'a>: SuperTrait` example under *Lifetime Erasure or
Downcast-Safety*) could be partially lifted: such a graph would be
admissible as long as every unsize site carried enough outlives
evidence to pin the subtrait's regions.

This RFC does not take that route. Three concerns drive the choice:

1. **Augmentation at every participating unsize site, not just every
   cast site.** Only unsize coercions whose target `dyn` trait
   inherits from `TraitMetadataTable` are affected, so the surface is
   narrower than "all unsizing" — but it is still strictly broader
   than the cast-site-only surface this RFC relies on. Cast sites are
   syntactically distinguished (`cast!(in dyn Root, ...)`); the
   qualifying unsize sites are not, and every `&C<'a,...> -> &dyn Root`
   coercion in a participating graph would need augmented-`Instance`
   handling. This broadens the region-sensitive monomorphization
   surface described in *Generic cast targets and lifetime-sensitive
   monomorphization* below.

2. **Vtable identity diverges from concrete identity.** Under this
   alternative the table key becomes
   `(root, concrete, outlives-evidence-at-unsize)` rather than
   `(root, concrete)`. Two `&dyn SuperTrait` values of the same
   concrete type but produced at different unsize sites carry
   different vtables and different cast behavior. That is observable
   in ways the current model never is.

3. **Action at a distance.** Whether a cast succeeds depends on what
   the unsize site proved, not on anything at the cast site. The
   locality property — the outlives evidence available at the cast
   itself selects the table entry — is what keeps diagnostics
   tractable; losing it produces errors of the form "this cast
   would have succeeded had the value been unsized under a stronger
   outlives bound in some other module," which is hard to surface
   usefully.

Soundness is preservable: the existing invariant ("erased lifetimes
stay erased") becomes the special case where no unsize-site
augmentation occurs, and table selection refuses any outlives class
the unsize site did not certify. But the cost structure — doubling
the set of augmented sites, weakening cast-site locality, and
introducing per-unsize-site vtable divergence for the same concrete
type — does not justify the marginal gain in admissible trait
graphs. Programs that need a region-generic subtrait can declare a
region-generic root, which this RFC already accommodates with
predictable cost.

# Prior art
[prior-art]: #prior-art

- `dynamic_cast` in C++

Key differences:
- There is no need to patch up data pointers to handle diamond inheritance.
- Dynamically loaded trait implementations are intentionally disregarded, so no runtime graph traversal is needed.

Conceptually, C++ could implement casting similarly to this proposal if those two features weren't required.

- Java and C#: interfaces

These are roughly the same ideas. Java's array casting is also out of scope
here, as Rust doesn't have `dyn [Trait]`, at least until fat pointers are
generalized.

Java assigns each concrete class a vtable for ordinary virtual dispatch and an
independent per-interface dispatch structure ("itable") for every interface that
the class implements. An itable is conceptually a dense, per-interface method
table that the JVM installs into the object's header via an indirection stored
in the class metadata, allowing constant-time resolution of interface calls 
without requiring graph traversal or RTTI lookups. During class loading, the JVM
computes these itables globally: it walks the full interface inheritance graph, 
flattens inherited interface methods into a canonical ordering, and records, for
each concrete class, the implementing method entry corresponding to each 
interface slot. Failed interface casts are handled by consulting this same global
metadata; the checked-cast operation performs a membership test against the 
precomputed interface implementation sets rather than performing structural 
probing at runtime. The net effect is that Java achieves stable, constant-time 
interface dispatch and constant-time checked interface casting at the cost of 
global computation and additional per-class metadata, which is broadly analogous
in spirit to this proposal's globally computed trait-metadata tables and indices.

- Go: interface type assertions

Go's `v, ok := x.(I)` is the closest surface analogue: an interface value `x`
is checked at runtime against another interface type `I`, yielding a new
interface value if the concrete dynamic type of `x` satisfies `I`. The
mechanics differ in several ways that are instructive for comparison. Go's
interface satisfaction is structural and name-based — a concrete type
satisfies `I` iff its method set covers `I`'s methods by name and signature,
with no declaration site — so the runtime derives the "does T implement I"
answer by walking method sets rather than reading a compiler-emitted table.
The result is cached in a global, lock-protected `itab` hash table keyed by
`(concrete type, interface type)`, so repeated assertions are cheap but the
cold-path first assertion costs a method-set walk. Because the Go runtime
owns all type metadata and builds itabs lazily, assertions compose cleanly
across plugin / shared-library boundaries — roughly the scenario this
RFC's `ForeignTraitGraph` path rejects. The price is a mandatory runtime,
mutable global state on the fast path of a previously-unseen assertion,
no compile-time bound on the set of interfaces that may be targeted, and
no mechanism for expressing lifetime relationships of the kind this
proposal has to preserve.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

## Macro naming

The proposed macro names `cast!`, `try_cast!`, and `unchecked_cast!` (exported
from `core` and re-exported in `std`) are short and may collide with user-code
identifiers. Stabilization should revisit whether the macros should carry a
`trait_` prefix (e.g. `trait_cast!`, `try_trait_cast!`,
`unchecked_trait_cast!`) or live under a dedicated path such as
`core::cast::cast!`. This RFC does not pre-commit a final name.

## `Display` and `Error` impls for `TraitCastError<T>`

`TraitCastError<T>` derives only `Debug, Clone, Copy`. Stabilization needs to
decide whether it should implement `core::fmt::Display` and
`core::error::Error`, and if so, what formatter output is appropriate for each
variant (in particular `ForeignTraitGraph` vs. `UnsatisfiedObligation`).

## Pin, raw pointer, and `NonNull<T>` impls of `TraitCast`

This RFC proposes `TraitCast` for exactly `&T`, `&mut T`, `Box<T, A>`,
`Rc<T, A>`, and `Arc<T, A>`. No impls are proposed for `Pin<P>`, `*const T`,
`*mut T`, or `NonNull<T>`. `Pin<&T>` in particular is a natural candidate.
Raw-pointer impls would need a crisp safety contract around the
`obj_graph_id` comparison (since the pointer may not be dereferenceable).
Stabilization should decide the final set.

## Non-LLVM backend enforcement of `address_significant`

The global-crate-id allocation relies on the codegen backend to suppress
`unnamed_addr`-style merging, or the per-global-crate uniqueness contract can
be broken by LTO or linker ICF. LLVM honors the address-significance flag
directly via `set_unnamed_address(UnnamedAddr::No)`. For Cranelift and GCC
the RFC does not prescribe a mechanism, so a binary built through those
backends has no functional guard against merging. Stabilization must resolve
this — either by requiring each backend to suppress merging for
address-significant allocations, or by introducing a shared upstream helper
emitting a marker the backends all honor.

## Legacy symbol mangler and `GenericArgKind::Outlives`

Only the v0 mangler is specified to encode `GenericArgKind::Outlives` (see
*Appendix C §C.3.3*). If a compilation falls back to the legacy mangler
for an augmented `Instance`, the resulting symbol encoding is unspecified.
Resolve either by explicitly rejecting legacy mangling of augmented
`Instance`s (and asserting v0 on those paths), or by extending the legacy
mangler to encode `Outlives` args.

## `VidProvenance::BoundedByUniversal` semantics

`BoundedByUniversal` covers the case where the NLL constraint graph records
only a forward edge (`'universal: vid`) on an unsizing coercion because
`dyn` types are covariant, but the effective concrete lifetime through the
coercion is the universal itself. Its interaction with nested unsizings,
higher-ranked subtyping, and re-borrow patterns is under-specified relative
to the other variants. Stabilization requires a test matrix that exercises
the variant on realistic user code and documentation describing the
invariant it preserves.

## Global crate identification in build systems

The RFC allows multiple global crates to coexist at runtime (see the cdylib
discussion in the guide) and exposes `-Z global_crate=yes|no` to override the
default derivation from crate-type. Stabilization needs to decide how Cargo
and other build systems should surface the global-crate role: continue to
derive it purely from crate-type, introduce a manifest key, or surface a
diagnostic when the heuristic is ambiguous.

## Interaction with native `dyn` upcasting

Rust already supports native `dyn` upcasting via embedded supertrait-vtable
pointers per trait object. This RFC's trait-cast machinery is additive; the
two mechanisms coexist. The long-term question is whether they should be
unified so that `&dyn Sub as &dyn Super` goes through the metadata table
(eliminating the embedded supertrait-vtable pointers in each vtable at the
cost of a small runtime lookup). See
*Future possibilities > Dyn upcasting* for the speculative sketch.

# Future possibilities
[future-possibilities]: #future-possibilities

## Dyn upcasting

Native `dyn` upcasting is already stable. It is implemented by embedding, in
each vtable, a pointer to the vtable of every supertrait reachable along the
trait hierarchy; an upcast from `&dyn Sub` to `&dyn Super` is a constant-time
load of that embedded pointer.

The per-root metadata-table machinery introduced by this RFC could, in
principle, subsume upcasting: an upcast is structurally identical to a
downcast where the target happens to lie above the source in the graph.
Routing upcasts through the per-supertrait metadata table would let us
drop the embedded supertrait-vtable pointers from every vtable, trading
a small per-upcast runtime lookup (essentially the same two loads as a
downcast) for a reduction in vtable size that scales with the depth and
fan-in of the trait graph.

This is a speculative future direction — it would require care around
backwards compatibility of the existing stable upcasting path, and the tradeoff
between vtable size and per-upcast cost is workload-dependent. No commitment
is made here.

## Downcasting to concrete types

As is, this proposal requires Pointee's with specific Metadata types, which preclude concrete types.

However, the proposed lifetime erasure rules could allow a path to safely downcast to a concrete type.

## Can we generalize the global visits?

Generally, we are performing global visits of two things:

- The trait graph rooted at a trait.
- The concrete types implementing the trait (or a trait).

And then we generate additional code and data as a result of those visits. The core capability is to delay until after
global monomorphization, while still allowing typeck/etc to work locally.

The mechanisms this RFC introduces to do that — delayed codegen, global-phase
queries that run once per final artifact, and cross-crate `DelayedInstance`
exchange — are not specific to trait casting. Plausibly they could be factored
into a general "global phase" capability that other features would consume:

- Whole-program vtable deduplication: coalescing vtables for identical concrete
  `(Ty, TraitRef)` pairs that would otherwise be emitted independently in each
  CGU.
- Global RTTI / linker-level reflection: emitting a single, canonical table of
  type descriptors for the final artifact, without requiring every dependency
  to agree on the format at build time.
- Global allocator selection and similar whole-program decisions that today
  live as ad-hoc lang items and late-resolved symbols.
- Whole-program const-eval of cross-crate tables where the input set is only
  known after the full dependency graph is visible.

Offering this as a general facility would mean stabilizing the contract that
the global phase operates on — in particular, which queries are allowed in the
global phase, how augmented `Instance`s are exchanged, and how backend
enforcement (the `address_significant` story) composes across features. This
is speculative; the current RFC does not propose such a generalization, only
notes that the building blocks it adds are a plausible starting point.

# Appendix A: Trait-graph worked examples
[appendix-a]: #appendix-a

The examples below are conformance oracles for the guide-level explanation.
They are not required reading for understanding the proposal; each
illustrates one property in isolation.

## A.1 Exhaustive four-type / six-trait matrix

```rust
pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }

// These types and traits can be spread out over multiple crates.
struct S0;
struct S1;
struct S2;
struct S3;
pub trait Trait1: SuperTrait { }
pub trait Trait2: SuperTrait { }
pub trait Trait3: Trait1 + Trait2 { }
pub trait Trait4: SuperTrait { }
pub trait Trait5: Trait4 { }
pub trait Trait6: Trait3 + Trait5 { }

/// A trait that is not part of the trait graph.
/// It can't be cast from or to any trait in the graph.
pub trait IrrelevantTrait { }

impl SuperTrait for S0 { }
impl Trait1 for S0 { }

impl SuperTrait for S1 { }
impl Trait2 for S1 { }

impl SuperTrait for S2 { }
impl Trait1 for S2 { }
impl Trait2 for S2 { }
impl Trait3 for S2 { }

impl SuperTrait for S3 { }
impl Trait1 for S3 { }
impl Trait2 for S3 { }
impl Trait3 for S3 { }
impl Trait4 for S3 { }
impl Trait5 for S3 { }
impl Trait6 for S3 { }

#[test]
fn s0() {
    let s = S0;
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait1).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait1)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait2).map(|r| r as *const _).ok(),
        None
    );
}
#[test]
fn s1() {
    let s = S1;
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait1).map(|r| r as *const _).ok(),
        None
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait2).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait2)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait3).map(|r| r as *const _).ok(),
        None
    );
}
#[test]
fn s2() {
    let s = S2;
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait1).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait1)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait2).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait2)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait3).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    let s1 = cast!(in dyn SuperTrait, &s => dyn Trait1).unwrap();
    let s2 = cast!(in dyn SuperTrait, &s => dyn Trait2).unwrap();
    assert_eq!(
        cast!(in dyn SuperTrait, s1 => dyn Trait3).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, s2 => dyn Trait3).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
}
#[test]
fn s3() {
    let s = S3;
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait1).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait1)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait2).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait2)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait3).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait4).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait4)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait5).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait5)
    );
    assert_eq!(
        cast!(in dyn SuperTrait, &s => dyn Trait6).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait6)
    );

    let s3 = cast!(in dyn SuperTrait, &s => dyn Trait3).unwrap();
    assert_eq!(
        cast!(in dyn SuperTrait, s3 => dyn Trait4).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait4)
    );
}
```

## A.2 Multiple roots

```rust
pub trait SuperTrait1: TraitMetadataTable<dyn SuperTrait1> { }
pub trait SuperTrait2: TraitMetadataTable<dyn SuperTrait2> { }

pub trait Trait1: SuperTrait1 { }
pub trait Trait2: SuperTrait2 { }
pub trait Trait3: Trait1 + Trait2 { }

pub struct S1;
pub struct S2;
pub struct S3;

impl SuperTrait1 for S1 { }
impl SuperTrait2 for S2 { }
impl SuperTrait1 for S3 { }
impl SuperTrait2 for S3 { }
impl Trait1 for S1 { }
impl Trait2 for S2 { }
impl Trait1 for S3 { }
impl Trait2 for S3 { }
impl Trait3 for S3 { }

// S3 will have *two* trait vtable tables: one for SuperTrait1 and one for SuperTrait2.
// S1 and S2 will have only one trait vtable table.

#[test]
fn s3_multiple_supertraits() {
    let s = S3;
    assert_eq!(
        cast!(in dyn SuperTrait1, &s => dyn Trait1).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait1)
    );
    assert_eq!(
        cast!(in dyn SuperTrait2, &s => dyn Trait2).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait2)
    );
    assert_eq!(
        cast!(in dyn SuperTrait1, &s => dyn Trait3).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    assert_eq!(
        cast!(in dyn SuperTrait2, &s => dyn Trait3).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );

    // So far, so obvious. But what about this?
    let s1 = cast!(in dyn SuperTrait1, &s => dyn Trait1).unwrap();
    let s2 = cast!(in dyn SuperTrait2, &s => dyn Trait2).unwrap();
    // COMPILE ERROR: Trait1 and Trait2 do not share a common supertrait, so
    // the following have unsatisfiable constraints:
    //   cast!(in dyn SuperTrait1, s1 => dyn Trait2)
    //   cast!(in dyn SuperTrait2, s2 => dyn Trait1)

    // But Trait3 has a shared supertrait with both Trait1 and Trait2, so:
    assert_eq!(
        cast!(in dyn SuperTrait1, s1 => dyn Trait3).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
    assert_eq!(
        cast!(in dyn SuperTrait2, s2 => dyn Trait3).map(|r| r as *const _).ok(),
        Some(&s as *const dyn Trait3)
    );
}
```

## A.3 Generic roots

```rust
pub trait SuperTrait<T>: TraitMetadataTable<dyn SuperTrait<T>> { }

pub trait Trait1: SuperTrait<u8> { }
pub trait Trait2<T>: SuperTrait<T> { }
pub trait Trait3: Trait1 + Trait2<u16> { }

// Same as the multiple-supertrait example, but with a generic supertrait.
// Trait3 has two supertraits: SuperTrait<u8> and SuperTrait<u16>.

/// This will have one super trait, after monomorphization.
pub trait Trait4: Trait1 + Trait2<u8> { }
```

## A.4 Lifetime selection

```rust
trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
trait SubTrait<'a>: SuperTrait { }

struct S0<'a>(PhantomData<fn(&'a ()) -> &'a()>);
impl<'a> SuperTrait for S0<'a> { }
impl<'a> SubTrait<'a> for S0<'a> { }

struct S1<'a>(PhantomData<fn(&'a ()) -> &'a()>);
impl<'a> SuperTrait for S1<'a> { }
impl<'a> SubTrait<'a> for S1<'static> { }
// Technically, `S1<'static>` implements `for<'a> SubTrait<'a>`, i.e.
// for all lifetimes.

struct S2<'a>(PhantomData<fn(&'a ()) -> &'a()>);
impl<'a> SuperTrait for S2<'a> { }
impl<'a> SubTrait<'static> for S2<'a> { }
// Note: `S1<'_>` does not implement `for<'a> SubTrait<'a>` (!= `SubTrait<'static>`).
// Trait generics are invariant, so `'static` can't be "relaxed" to any lifetime
// like, e.g., `&'static u8` can.

macro_rules! cast_helper {
  ($b:lifetime, $e:expr) => (
    cast!(in dyn SuperTrait, $e as &(dyn SuperTrait + $b) => dyn SubTrait<$b>).ok()
  )
}

#[test]
fn static_s0() {
  const S: S0<'static> = S0(/*...*/);
  assert!(cast_helper!('static, &S).is_some());
}
#[test]
fn non_static_s0() {
  let s = S0(/*...*/);
  fn inner<'a>(s: &'a S0<'a>) {
    assert!(cast_helper!('a, s).is_some());
    assert!(cast_helper!('static, s).is_none());
  }
  inner(&s);
}
#[test]
fn static_s1() {
  const S: S1<'static> = S1(/*...*/);
  fn inner<'a>(s: &'static S1<'static>, _: &'a ()) {
    assert!(cast_helper!('a, s).is_some());
    assert!(cast_helper!('static, s).is_some());
  }
  inner(&S, &());
  assert!(cast!(in dyn SuperTrait, &S => dyn for<'out> SubTrait<'out>).is_ok());
}
#[test]
fn non_static_s1() {
  let s = S1(/*...*/);
  fn inner<'a>(s: &'a S1<'a>) {
    // `S1<'a>` does not implement `SubTrait<'_>` for any lifetime other
    // than `'static`.
    assert!(cast_helper!('a, s).is_none());
    assert!(cast_helper!('static, s).is_none());
  }
  inner(&s);
}
#[test]
fn non_static_s2() {
  let s = S2(/*...*/);
  fn inner<'a>(s: &'a S2<'_>) {
    assert!(cast_helper!('a, s).is_none());
    // `S2<'a>` implements `SubTrait<'static>` for any lifetime `'a`.
    assert!(cast_helper!('static, s).is_some()); // !
  }
  inner(&s);
}
```

All bound lifetimes participate in the check, not only those syntactically
present in the trait definition:

```rust
trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }
trait SubTrait: SuperTrait {
  type Assoc;
}
/// Note: all lifetimes are considered, including those reached through
/// associated-type bindings:
type T3<'a> = dyn SubTrait<Assoc = &'a u8>;
```

## A.5 Multiple lifetimes

With multiple lifetimes, casts must preserve relationships (`'b: 'a`, etc.)
independent of erasure:

```rust
trait SuperTrait<'a, 'b>: TraitMetadataTable<dyn SuperTrait<'a, 'b>> { }
trait SubTrait<'a, 'b>: SuperTrait<'a, 'b> { }

#[derive(Default)]
struct S0<'a, 'b> {
  _m0: PhantomData<&'a ()>,
  _m1: PhantomData<&'b ()>,
}
#[derive(Default)]
struct S1<'a, 'b> {
  _m0: PhantomData<&'a ()>,
  _m1: PhantomData<&'b ()>,
}
impl<'a, 'b> SuperTrait<'a, 'b> for S0<'a, 'b> { }
impl<'a, 'b> SuperTrait<'a, 'b> for S1<'a, 'b> { }
impl<'a, 'b> SubTrait<'a, 'b> for S0<'a, 'b> { }
impl<'a, 'b> SubTrait<'a, 'b> for S1<'a, 'b>
where 'b: 'a,
{ }

macro_rules! cast_helper {
  ($a:lifetime, $b:lifetime, $e:expr) => (
    cast!(in dyn SuperTrait<'_, '_>, $e as &dyn SuperTrait<'_, '_> => dyn SubTrait<$a, $b>).ok()
  )
}

#[test]
fn unrelated_lifetimes() {
  fn inner<'a, 'b>(_: &'a (), _: &'b ()) {
    let s = S0::<'a, 'b>::default();
    assert!(cast_helper!('a, 'b, &s).is_some());
    let s = S1::<'a, 'b>::default();
    assert!(cast_helper!('a, 'b, &s).is_none());
  }
  inner(&(), &());
}
#[test]
fn related_lifetimes() {
  fn inner<'a, 'b>(_: &'a (), _: &'b ())
    where 'b: 'a,
  {
    let s0 = S0::<'a, 'b>::default();
    assert!(cast_helper!('a, 'b, &s0).is_some());
    assert!(cast_helper!('a, 'a, &s0).is_some()); // via variance of S0
    let s1 = S1::<'a, 'b>::default();
    assert!(cast_helper!('a, 'b, &s1).is_some()); // S1's `'b: 'a` impl predicate is now satisfied.
    assert!(cast_helper!('a, 'a, &s1).is_some()); // via variance of S1
  }
  inner(&(), &());
}
```

# Appendix B: Cross-crate cdylib example
[appendix-b]: #appendix-b

The topology is `A` cdylib + `B` cdylib + `C` shared dylib. `A` and `B`
act as interfaces; `C` is a shared library both depend on. The core
problem stems from separately computed `(SuperTrait, Struct, Trait)`
indices in different global crates — longer dependency chains behave the
same way, so this is the minimal shape.

```rust
#![crate_type = "dylib"]
// C.rs
pub trait SuperTrait: TraitMetadataTable<dyn SuperTrait> { }

#[repr(C)]
pub struct FfiObject(Box<dyn SuperTrait>);
impl FfiObject {
  pub fn new(inner: impl SuperTrait) -> Self { Self(Box::new(inner)) }
}
impl core::ops::Deref for FfiObject {
  type Target = dyn SuperTrait;
  fn deref(&self) -> &Self::Target { &self.0 }
}
impl core::ops::DerefMut for FfiObject {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
```
```rust
// B.rs
#![crate_type = "cdylib"]
extern crate C;
use C::*;

trait BTrait: SuperTrait {
  fn thing_done(&self) -> bool;
  fn do_b_thing(&mut self) -> Result<(), Box<str>>;
}

struct InternalB { thing_done: bool }
impl SuperTrait for InternalB { }
impl BTrait for InternalB {
  fn thing_done(&self) -> bool { self.thing_done }
  fn do_b_thing(&mut self) -> Result<(), Box<str>> {
    self.thing_done = true;
    Ok(())
  }
}

#[no_mangle]
unsafe extern "C" fn init_obj(obj: *mut MaybeUninit<FfiObject>) {
  unsafe {
    obj.as_mut_unchecked().write(FfiObject::new(InternalB { thing_done: false }));
  }
}
#[no_mangle]
unsafe extern "C" fn uninit_obj(obj: *mut FfiObject) {
  let Some(obj) = (unsafe { obj.as_mut() }) else { return; };
  unsafe { core::ptr::drop_in_place(obj); }
}
#[no_mangle]
unsafe extern "C" fn do_thing(obj: *mut FfiObject) -> core::ffi::c_int {
  let Some(obj) = (unsafe { obj.as_mut() }) else { return 0; };
  let Ok(obj) = cast!(in dyn SuperTrait, &mut **obj => dyn BTrait) else { return 0; };
  obj.do_b_thing().is_ok() as _
}
#[no_mangle]
unsafe extern "C" fn thing_done(obj: *mut FfiObject) -> core::ffi::c_int {
  let Some(obj) = (unsafe { obj.as_mut() }) else { return 0; };
  let Ok(obj) = cast!(in dyn SuperTrait, &mut **obj => dyn BTrait) else { return 0; };
  obj.thing_done() as _
}
```
```rust
// A.rs — symmetrically the same as B.rs, with `BTrait`/`InternalB`/`do_b_thing`
// replaced by `ATrait`/`InternalA`/`do_a_thing`.
```

A loader binary (Rust for exposition, but could equally be C++) dlopens
both cdylibs. The salient observation is the final block:

```rust
// user.rs (dlopen/ffi scaffolding elided)
fn main() {
    let a = dlopen_load("libA.so");
    let b = dlopen_load("libB.so");

    let mut a_obj = a.new_obj();  // libA-built: trait graph is A's.
    let mut b_obj = b.new_obj();  // libB-built: trait graph is B's.

    // Both return 0: the cast inside do_thing returns
    // Err(TraitCastError::ForeignTraitGraph) because the global-crate
    // identities do not match — regardless of any index coincidence
    // between A's ATrait and B's BTrait.
    assert_eq!(unsafe { (a.do_thing)(&mut b_obj) }, 0);
    assert_eq!(unsafe { (b.do_thing)(&mut a_obj) }, 0);
}
```

Forcing `C` to be the global crate is not workable in general, even if
all traits are defined in `C`. The trait graph is over lazily
monomorphized trait-object nodes such as `dyn SuperTrait<u8>`,
`dyn Trait2<u16>`, and `dyn Trait2<Downstream>`; castability depends on
concrete instantiations, and `dyn Trait2<Downstream>` does not exist from
`C`'s point of view until `Downstream` is monomorphized in `A`. Any
scheme that tries to have `C` pre-assign indices for all future
instantiations is unbounded and unknowable at `C`'s compile time.
Dynamic registries are also out: the trait graph is lazy (only traits
appearing as cast targets are included), so a registry would need to
codegen vtables for foreign types at runtime — effectively shipping a
subset of the Rust compiler. Absent a major shift in compiler
infrastructure, a solution without these drawbacks is out of reach.

# Appendix C: Implementation sketch (non-normative)
[appendix-c]: #appendix-c

This appendix sketches how a compiler could realize the contract
defined in the reference section. It is not normative: a conforming
implementation may differ in any of the mechanisms described here, as
long as it preserves the semantics specified above. The specific types,
queries, and algorithmic choices below are drawn from the prototype and
are included to demonstrate tractability.

## C.0 rustc-internal hooks

### C.0.1 Global-crate detection

The declarative contract in *Definitions / Global crate* names a
compile-time boolean and an override mechanism without specifying
them. rustc realizes them as:

* `tcx.is_global_crate() -> bool`, a compile-time boolean exposed on
  `TyCtxt` (defined in `rustc_middle::ty::context`). The default
  provider keys on `CrateType`: `Executable`, `StaticLib`, and
  `Cdylib` return `true`; `Dylib`, `Rlib`, `ProcMacro`, and `Sdylib`
  return `false`.
* `-Z global_crate=yes|no`, an unstable session option backed by
  `unstable_opts.global_crate: Option<bool>` in
  `rustc_session::options`. When set, the explicit value is returned
  by `tcx.is_global_crate()` unconditionally, bypassing the
  `CrateType`-based default.

A conforming implementation may satisfy the global-crate contract by
any equivalent mechanism.

### C.0.2 `TraitMetadataTable` internals

The user-facing form in *TraitMetadataTable* is a bare trait
declaration with four intrinsic free functions. rustc realizes them
with language-item and attribute markers and a blanket impl:

* `#[lang_item = "trait_metadata_table"]` on the trait so the
  compiler can resolve it by name.
* `#[rustc_coinductive]` to allow coinductive resolution of cycles
  arising from root supertraits that inherit from
  `TraitMetadataTable<dyn Self>` (e.g.,
  `trait Foo: TraitMetadataTable<dyn Foo>`).
* `#[rustc_nounwind] #[rustc_intrinsic]` on each of the four
  `trait_metadata_*` / `trait_cast_is_lifetime_erasure_safe`
  intrinsic declarations.

A blanket impl covers all `Sized` types, making the trait effectively
`#[rustc_deny_explicit_impl]` — users do not write
`impl TraitMetadataTable` directly; the supertrait bound
`trait Root: TraitMetadataTable<dyn Root>` is satisfied by the
blanket for any concrete `T` the user writes `impl Root for T` on:

```rust
impl<SuperTrait, T: Sized> TraitMetadataTable<SuperTrait> for T
where
    SuperTrait: MetaSized
        + Pointee<Metadata = DynMetadata<SuperTrait>>
        + TraitMetadataTable<SuperTrait>,
{
    fn derived_metadata_table(&self) -> (&'static u8, NonNull<Option<NonNull<()>>>) {
        // SAFETY: intrinsic requires `unsafe` but is not actually unsafe to call here.
        unsafe { core::intrinsics::trait_metadata_table::<SuperTrait, T>() }
    }
}
```

The actual constraint that `T` implements the root supertrait is
enforced by the supertrait relationship itself (the user must write
`impl Root for T`), not by this impl's where-clauses.

The impl intentionally omits an `Unsize<SuperTrait>` bound to avoid a
cycle in the trait solver: proving `T: Unsize<dyn Root>` would
require `T: Root`, which requires the supertrait
`T: TraitMetadataTable<dyn Root>`, which would cycle back through
`Unsize`. The `SuperTrait: TraitMetadataTable<SuperTrait>` bound on
the intrinsic is satisfied via the object candidate (vtable
dispatch) when `SuperTrait = dyn Root`, since
`TraitMetadataTable<dyn Root>` is a supertrait of `Root`.

## C.1 Metadata-table layout: query-level detail

Computation runs only in the global crate as three cached queries that
feed one another. All three are keyed by `Ty<'tcx>` so that invalidation
tracks the reachable trait set at trait-object granularity; the graph
and layout are arena-cached:

* `trait_cast_graph(root: Ty<'tcx>) -> &'tcx TraitGraph<'tcx>`
* `trait_cast_layout(root: Ty<'tcx>) -> &'tcx TableLayout<'tcx>`
* `trait_cast_table((root, concrete)) -> &'tcx [Option<AllocId>]`

**`trait_cast_graph(root)`** partitions the gathered delayed-codegen
requests into sub-trait → outlives-class mappings and a set of concrete
types that requested metadata tables for this root. Only requests whose
`super_trait` matches `root` are considered; each index request is
reduced to its outlives class and inserted into the per-sub-trait info
for the target. After the request scan, the graph is augmented with any
where-clause-derived outlives classes implied by the sub-trait's own
`where 'a: 'b`-style predicates, so that casts carrying valid outlives
evidence through generic library code find the right slot. The query
reruns whenever the root's reachable delayed-codegen requests change.

**`trait_cast_layout(root)`** assigns a table slot index to every
`(sub_trait, OutlivesClass)` pair in the graph. For each sub-trait, it
first resolves the participating impls over each concrete type in the
graph. If every resolved impl passes the `impl_universally_admissible`
fast path, all outlives classes for that sub-trait collapse onto a
single slot (see the `impl_universally_admissible fast path` subsection
in the reference). Otherwise, the layout builds a `BitMatrix<u32, u32>`
whose rows are outlives classes and columns are participating concrete
types, with bit `(c, t)` set iff the impl for `t` is admissible under
class `c`. Classes with identical rows share a single slot. The output
`TableLayout` stores the flat `(sub_trait, OutlivesClass) -> slot` map
plus per-slot metadata (sub-trait, representative class, binder-variable
count). `OutlivesClass` borrows an interned, sorted-and-deduped subslice
of a monomorphized `Instance`'s outlives entries:

```rust
pub struct OutlivesClass<'tcx> {
    /// `instance.outlives_entries()[1..]` — the semantic pairs,
    /// skipping the sentinel at position 0.
    pub entries: &'tcx [GenericArg<'tcx>],
}
```

**`trait_cast_table((root, concrete))`** populates one table per
concrete struct. Iterating the layout's sub-traits, it resolves the
concrete type's impl once and then, for each slot belonging to that
sub-trait, checks admissibility against the slot's representative class
using the cached outlives-reachability matrix. Slots where the impl is
admissible receive the corresponding vtable's `AllocId`; slots the
concrete type does not satisfy remain `None`. A sibling query emits the
resulting array as an immutable `.rodata` allocation.

Pruning is implicit: sub-traits with no requesting outlives class are
skipped and never receive a slot, so unreachable cast targets do not
appear in the layout at all (there is no reserved sentinel index).

## C.2 Delayed codegen requests

A direct call to `trait_metadata_index` / `trait_metadata_table` /
`trait_metadata_table_len` / `trait_cast_is_lifetime_erasure_safe`
resolves to an `Instance` whose final codegen must be deferred until
the global crate. The concrete slot index, vtable table, and length are
products of globally-computed trait-cast layout and population
(`trait_cast_layout`, `trait_cast_table`, `trait_cast_table_alloc`),
none of which are available to an upstream crate in isolation. Emitting
these intrinsics eagerly upstream would bake in stale indices.

Instead, the collector records each such caller plus its intrinsic
callees as a `DelayedInstance<'tcx>`:

```rust
pub struct DelayedInstance<'tcx> {
    pub instance: Instance<'tcx>,
    pub callee_substitutions: &'tcx [(
        &'tcx List<(DefId, u32, GenericArgsRef<'tcx>)>,
        Instance<'tcx>,
    )],
    pub intrinsic_callees: &'tcx [Instance<'tcx>],
}
```

`instance` is the caller whose codegen is being deferred.
`callee_substitutions` pairs `call_id` chains (§C.3.1) with the
augmented callee `Instance` that the global phase must splice into the
body at that call site. `intrinsic_callees` is the separate list of
augmented intrinsic leaves used by the global-phase condensation
pipeline.

Upstream crates record these `DelayedInstance`s in their crate metadata
and **do not** codegen the intrinsics themselves. The global crate
consumes every upstream crate's `delayed_codegen_requests(CrateNum)` to
drive its global-phase work (layout, table population, MIR patching,
and final codegen). Upstream rmeta carries a per-crate
`LazyArray<DelayedInstance>` that the global crate decodes on demand.

## C.3 Monomorphization

* Mono collection is made outlives-sensitive for functions that
  transitively contain trait-cast intrinsics whose generic parameters
  may carry lifetimes. MIR regions continue to be `ReErased` as usual
  — no per-function deviation from the existing erasure pipeline.
  Outlives information is instead threaded through two new
  borrowck-side queries:
  * `borrowck_result(LocalDefId) -> &'tcx mir::BorrowckResult<'tcx>` —
    the shared core computation; `mir_borrowck` and
    `borrowck_region_summary` both project from it.
  * `borrowck_region_summary(DefId) -> &'tcx mir::BorrowckRegionSummary`
    — the cross-crate surface the mono collector consumes
    (`separate_provide_extern`).
* `BorrowckRegionSummary` carries:
  * `call_site_mappings: UnordMap<u32, CallSiteRegionMapping>` keyed by
    the per-body `call_id` counter from the
    `(DefId, u32, GenericArgsRef<'tcx>)` chain (§C.3.1);
  * `outlives_graph: ProjectedOutlivesGraph` — the projected SCC graph
    over regions involved in call-site mappings;
  * `vid_provenance: UnordMap<u32, VidProvenance>` and
    `vid_to_param_pos: Vec<(u32, u32)>` /
    `vid_to_resolved_param: Vec<(u32, u32)>` giving the
    universal-region / param-position correspondence, with
    `STATIC_PARAM_POS = u32::MAX` marking `ReStatic`.

  Mutually outliving regions (`'a: 'b` and `'b: 'a`) are encoded via
  Hamiltonian-cycle pairs over the condensed SCC, because
  `ty::Instance` is regions-erased and equivalence classes cannot be
  collapsed to a single representative without losing mangling and
  query-cache identity.
* In each MIR body, collect contained normalized but not erased unique
  (`SuperTrait`, `Trait`) pairs from the `trait_metadata_index`
  intrinsic, and similar unique (`SuperTrait`, `Struct`) pairs from the
  `trait_metadata_table` intrinsic.
* Any direct call to `trait_metadata_index` / `trait_metadata_table` /
  `trait_metadata_table_len` / `trait_cast_is_lifetime_erasure_safe` is
  treated as a monomorphization request and is added to the crate's
  list of delayed codegen requests. Upstream crates never codegen these
  intrinsics; they only record them as requirements in metadata.
* Ensure the linkage and visibility of direct references from ^ is
  linkable downstream.

### C.3.1 Call Site Identity: the `call_id` Chain

After lifetime erasure, two `Call` terminators inside the same MIR body
that resolve to the same callee `DefId` with the same erased
`GenericArgsRef` are indistinguishable by any property of the
terminator's `func` operand alone. Each call site may nevertheless sit
under a different outlives context in the caller, and the
per-call-site outlives computation (§C.3.5) must produce a distinct
augmented callee `Instance` for each. Call sites therefore need stable,
erasure-independent identity that survives all MIR passes and is
preserved through inlining. That marker is the `call_id` chain on every
`Call` / `TailCall` terminator.

Both `TerminatorKind::Call` and `TerminatorKind::TailCall` gain an
interned chain field:

```rust
TerminatorKind::Call {
    // ...existing fields...
    #[type_foldable(identity)]
    #[type_visitable(ignore)]
    call_id: &'tcx List<(DefId, u32, ty::GenericArgsRef<'tcx>)>,
}
```

Each tuple entry records one link in the inlining path. The `DefId`
names the function body in which the call was originally constructed
during MIR building; the `u32` is a body-local counter unique among
`Call` / `TailCall` terminators within that body; and the
`GenericArgsRef<'tcx>` stores the callee's edge-local generic-arg
template expressed in that source body's own generic-parameter space.
The `#[type_foldable(identity)]` / `#[type_visitable(ignore)]`
attributes are load-bearing: the chain is a structural identifier, not
a type, and generic substitution on a body must not touch it — the
embedded `DefId` and template `GenericArgsRef` are resolved stepwise
against outer-caller args downstream.

The `u32` counter is allocated at MIR build time; a body carries a
`next_call_id` cursor so that synthetic calls added later (drop
elaboration, shims, etc.) can allocate fresh non-colliding ids. Chains
are interned on `TyCtxt`, and pointer equality on the interned
`&'tcx List<…>` is the primary identity used downstream, so passes
preserve chain sharing and only re-intern when they must rewrite.

When the MIR inliner splices a callee body into a caller, it captures
the caller terminator's chain and, while walking the inlined callee,
**prepends** the caller's chain to each inlined terminator's chain,
re-interning the result:

```rust
chain.extend(self.caller_call_chain.iter());
chain.extend(call_id.iter());
*call_id = self.tcx.mk_call_chain(&chain);
```

The chain thus grows monotonically from outermost caller to innermost
call site; `call_id[0]` always identifies the outermost source body.

Monomorphization-time patching locates a specific `Call` / `TailCall`
in a cloned body by **pointer equality** on the pre-patch interned list
and rewrites the terminator's `func` operand to reference the augmented
callee's `FnDef`. Pointer equality suffices because chain sharing is
preserved by the inliner and the interner, so a given
`(call_site → callee)` substitution in a delayed codegen request
uniquely picks out one terminator.

Two downstream consumers rely on the chain:

- **Per-call-site outlives derivation** iterates call terminators to
  trait-cast intrinsics and reads `&call_id[0]` to key each site's
  `CallSiteRegionMapping` lookup in the originating body's
  `borrowck_region_summary` on the `u32`. Two call terminators with
  different `call_id`s in the same body thus yield distinct augmented
  callee instances even when every visible generic arg matches.
- **Intrinsic collection** walks call terminators to discover
  trait-cast intrinsic sites and projects off the resulting intrinsic
  `Instance`'s args. The `call_id` is what lets the collector key this
  work per call site rather than collapsing erased-identical sites.

The `call_id` does not itself participate in v0 symbol mangling. What
lands in the augmented callee's mangled name is the resulting
`GenericArgKind::Outlives` args carried on the substituted `Instance`,
not the chain used to compute them.

### C.3.2 Add `GenericArgKind::Outlives`

After lifetime erasure, two call sites to `trait_metadata_index` with
different outlives contexts produce identical `ty::Instance` values
(same `DefId`, same erased `GenericArgsRef`). Since `Instance` is used
as a unique key for symbol names, query caching, and mono-item
deduplication, these must be distinguished.

We add a fourth variant to `GenericArgKind` (in `rustc_type_ir`) that
wraps an interner handle to outlives-predicate data:

```rust
pub struct OutlivesArgData {
    pub longer: usize,
    pub shorter: usize,
}

pub enum GenericArgKind<I: Interner> {
    Lifetime(I::Region),
    Type(I::Ty),
    Const(I::Const),
    Outlives(I::OutlivesArg),
}
```

Each `Outlives` arg encodes a single outlives predicate using the
canonical bound-variable (`BoundVar`) indices of the `dyn` type's
existential binder. Index `usize::MAX` denotes `'static`. For example,
given `dyn SubTrait<'^0, '^1>` where `'^1: '^0`, the arg's
`OutlivesArgData` is `{ longer: 1, shorter: 0 }`. For
`dyn SubTrait<'^0, '^1>` where `'^0: 'static`, it is
`{ longer: 0, shorter: usize::MAX }`. For
`dyn SubTrait<Target = &'^0 ()>` where `'^0: 'static`, it is
`{ longer: 0, shorter: usize::MAX }` — the index refers to the binder
variable, not to a position in the generic args list. Note: all
lifetimes are actually `ReErased` here; `dyn SubTrait<'static, _>` is
impossible.

These indices are stable across erasure because they refer to positions
in the `dyn` type's existential binder, whose canonical variable
ordering is deterministic and independent of lifetime erasure.
`Outlives` args must be sorted in `(longer, shorter)` order.

`Outlives` args are appended after the function's declared generic
parameters and after closure generic parameters.

**Interning.** `I::OutlivesArg` is interned via a new `outlives_arg`
field on `CtxtInterners` and constructed through
`tcx.mk_outlives_arg(longer, shorter)`. Interning lets each
`OutlivesArg` be represented by a single pointer (rather than two
`usize`s), preserves hashing/equality at pointer granularity, and keeps
`GenericArg` at its existing size.

**Pointer tagging.** `GenericArg` uses the low 2 bits of an interned
pointer as a tag discriminant. The existing tags are `0b00` (Type),
`0b01` (Region), `0b10` (Const) — `0b11` was unused and is now claimed
by `Outlives`.

This gives us:

* Distinct `symbol_name` results.
* Distinct query cache entries for `symbol_name`, `items_of_instance`,
  `size_estimate`, etc. This is correct: different outlives contexts
  require different codegen (different index constants).
* Correct `MonoItem` deduplication — different outlives contexts are
  different mono items.

### C.3.3 Instance Augmentation: Base vs. Augmented

**Motivation.** Two intrinsic call sites that resolve to the same
`DefId` with the same post-erasure `GenericArgsRef` still differ in
their outlives context (the relationships between the `dyn` type's
binder variables at that site). The global phase must codegen them as
structurally distinct mono items so that symbol mangling, query
caching, and mono-item deduplication treat them separately. We achieve
this by threading outlives information through `Instance::args` itself:
an **augmented** `Instance` appends `Outlives` generic args to its base
`Instance`'s args, producing a new `Instance` whose `args` pointer is
distinct under structural equality and hashing.

**`OUTLIVES_SENTINEL`.** Appending outlives entries alone is not
sufficient, because a site with zero outlives relationships would be
indistinguishable from its base. Augmentation therefore always prepends
a sentinel before any caller-supplied outlives pairs:

```rust
pub const OUTLIVES_SENTINEL: (usize, usize) = (usize::MAX, usize::MAX);
```

The constructor `Instance::with_outlives(self, tcx, outlives)` is the
only supported path: it `debug_assert!`s that `outlives` does not
contain the sentinel, then builds a fresh `GenericArgs` by chaining
`self.args`, one interned sentinel `Outlives` arg, and one interned
`Outlives` arg per caller-supplied pair. A base `Instance` has **zero**
`Outlives` args — not even the sentinel. Any `Instance` whose tail
begins with `OUTLIVES_SENTINEL` has been augmented, even if it carries
no real outlives relationships.

**Helpers on `Instance`.** The following helpers are defined alongside
`with_outlives`:

* `outlives_entries(self) -> &'tcx [GenericArg<'tcx>]` — the tail slice
  of `Outlives` entries (including the sentinel). Returns `&[]` for a
  base instance.
* `outlives_indices_iter(self) -> impl Iterator<Item = (usize, usize)>`
  — yields the semantic `(longer, shorter)` pairs, skipping the
  sentinel; `bug!()`s if a non-`Outlives` entry appears in the tail.
* `has_outlives_entries(self) -> bool` — `true` iff the `Instance` has
  been augmented (carries at least the sentinel).
* `strip_outlives(self, tcx) -> Instance<'tcx>` — reconstructs the
  base `Instance` by truncating `args` at the first `Outlives` entry.

**Two coordinate systems.** The `(longer, shorter)` indices carried by
`Outlives` args do not all live in one space: for MIR-backed
user-wrapper callees they name walk-order positions in the callee's
own `GenericArgs`, while for MIR-less intrinsic leaves they index into
the `dyn` type's existential binder. A full specification of the
spaces and the transport rules between them appears in §C.3.4.

**v0 symbol mangling.** v0 gains a new `<generic-arg>` production for
the `Outlives` kind: tag bytes `Oo`, followed by the `longer` index, a
`_` separator, the `shorter` index, and a trailing `E`. Impl-path
printing must explicitly check for `Outlives` entries when deciding
whether to emit a generic-arg list, because `Outlives` args do not set
`TypeFlags`'s "has non-region param" bit. The legacy (pre-v0) mangler
has no dedicated `Outlives` handling; augmented `Instance`s are
expected to reach only v0-mangled call paths, and sites that end up on
the legacy path with augmented args have no stable encoding.

**Phase-2 cleanup.** A base `Instance` can enter mono collection
during a Phase-1 traversal (for example, `check_a` is first recorded
non-augmented, then `main` later augments it). After Phase-2
augmentation runs, the collector removes superseded base mono items so
only the augmented variant survives for codegen: for each replaced
base, the collector transfers the base's usage entries onto the
augmented replacement and filters the base out of the `delayed_codegen`
set so per-crate `delayed_codegen_requests` consumers only see
augmented replacements.

**Reaching codegen.** Augmented `Instance`s flow through `codegen_mir`
exactly like any other `Instance`; for those with patched bodies, the
global phase of partitioning feeds the patched body via
`tcx.feed_codegen_mir(instance, body)` before inserting the instance
into the final mono-item set.

### C.3.4 Outlives Index Spaces

Every augmented `Instance` carries a tail of `Outlives(longer, shorter)`
generic args, each a pair of `usize` indices. The *shape* of those pairs
is uniform, but the *meaning* of the indices depends on which kind of
callee the args are attached to. There are three distinct index spaces,
and the `usize::MAX` sentinel for `'static` is shared across all three
(the same convention used by `STATIC_PARAM_POS: u32 = u32::MAX` on
`BorrowckRegionSummary::vid_to_param_pos`).

**Space 1 — user-wrapper walk order.** For augmented `Instance`s of
MIR-backed user functions that transitively call a trait-cast
intrinsic, the indices name walk-order positions in the callee's own
`GenericArgs`. "Walk order" is the numbering produced by a type-visitor
DFS walk that advances a counter on *every* region encountered,
regardless of region kind — `ReVar`, `ReBound`, `ReErased`,
`ReEarlyParam`, `ReLateParam`, `ReStatic` all consume one position, so
the numbering stays stable across different region representations of
the same type structure. The `walk_pos → RegionVid` mapping lives on
each `CallSiteRegionMapping` recorded by borrowck.

**Space 2 — intrinsic dyn-binder.** For the MIR-less leaf intrinsic
`trait_metadata_index::<SuperTrait, TargetTrait>`, the native consumer
space is the target (sub-trait) `dyn` type's existential
binder-variable space. A transport-to-native rewrite maps each walk
position to its binder variable before the intrinsic consumes it. The
other two table-dependent intrinsics, `trait_metadata_table` and
`trait_metadata_table_len`, pass through augmentation but consult only
the concrete type arguments — they do not read the `Outlives` tail —
so Space 2 is effectively exercised only by `trait_metadata_index`.

**Space 3 — combined root+target+`'static`.** For
`trait_cast_is_lifetime_erasure_safe::<SuperTrait, TargetTrait>`, the
transport/origin space arranges slots in two concatenated blocks:

```text
// transport / origin walk-position space for the erasure-safe intrinsic:
[0 .. n_root)                   // root supertrait's walk positions
[n_root .. n_root + n_target)   // target trait's walk positions (offset by n_root)
 usize::MAX                     // sentinel for 'static (shared across spaces)
```

with `n_root` and `n_target` the region-slot counts of the super- and
target-trait types respectively. A target predicate walk-position
`t_wp` is translated to transport position `n_root + t_wp`; a root
predicate walk-position `r_wp` is kept as `r_wp`. The post-remap
("native") space packs the two segments' binder variables contiguously
(root bvs, then target bvs, with `'static` still at `usize::MAX`). A
single combined space is required because the erasure-safety check
compares pairs that name both binders at once (e.g. a target lifetime
mutually outlived by a root lifetime), so neither binder alone
suffices.

**Transport between spaces.** Two resolvers carry indices from Space 1
to Space 2 or Space 3:

* The `trait_metadata_index` resolver calls
  `augmented_outlives_for_call` plus `compose_all_through_chain` to
  transport entries into origin walk-position space, then remaps them
  into Space 2.
* The `trait_cast_is_lifetime_erasure_safe` resolver calls the same
  two helpers, then passes the transported entries (kept in
  origin/transport coordinates shaped like Space 3) to
  `tcx.is_lifetime_erasure_safe`.

Between user wrappers on a call-chain edge the transport stays in
walk-order space throughout. The batched composer is
`compose_all_through_chain(tcx, caller, call_id, n_positions) ->
Vec<Option<usize>>`, wrapped by the query `augmented_outlives_for_call`.
The `call_id` chain entries are `(DefId, u32, GenericArgsRef<'tcx>)`
triples — the third field is the edge-local template — and the
composer instantiates those templates stepwise from the outer caller
`Instance`.

The chain-composition machinery has three supporting types:

* `InputSlot { arg_ordinal: u32, offset_within_arg: u32 }` — decomposes
  a walk-order position into which argument carried the lifetime and
  where within that argument (so argument-template composition works
  precisely for projected lifetimes). A DFS builder over the body's
  parameter signature assigns one `InputSlot` per walk position.
* `VidProvenance` — a four-variant enum: `Static`, `Input(InputSlot)`,
  `BoundedByUniversal(InputSlot)`, `LocalOnly`. Each borrowck region
  vid carries a `VidProvenance` on the `BorrowckRegionSummary`,
  recording where it came from in the caller's input-space. The
  `BoundedByUniversal` case records unsizing-edge "lifetime GCD"
  bounds that are covariance-only in the NLL constraint graph.
* `compose_all_through_chain(tcx, caller, call_id) -> Vec<Option<usize>>`
  — the batched composer. Walks the `call_id` chain, concretizing each
  link's edge-local `GenericArgsRef<'tcx>` template against the outer
  caller `Instance`, and returns transported walk-order positions for
  every binder variable in the origin callee's space (or `None` for
  binder variables with no preserved input provenance).

MIR-less intrinsic consumers must remap transported entries into their
own native binder-variable space before consuming them, per the
`OutlivesClass` contract. The `GenericArgKind::Outlives` computation
that actually *produces* the entries from a caller's outlives
environment is specified in §C.3.5.

### C.3.5 `GenericArgKind::Outlives` Computation

The query that actually produces the `Outlives` tail for a given
`(caller, call_id, callee)` triple is `augmented_outlives_for_call`.
It returns the sentinel-stripped `&'tcx [GenericArg<'tcx>]` ready to
append via `Instance::with_outlives`; the sentinel itself is prepended
later by `with_outlives`, not by this query. All three pieces of
information feed into the same four-step pipeline — get the callee's
sensitivity, compose walk positions through the `call_id` chain, build
the caller's outlives oracle, then run `augment_callee` — but the
MIR-backed and MIR-less intrinsic branches enter that pipeline with
different inputs.

**Step 1 — look up the callee's sensitivity.** The first action is
`tcx.cast_relevant_lifetimes(callee)` (§C.4.3). A MIR-backed callee
that transitively calls a trait-cast intrinsic returns a
`CastRelevantLifetimes` value: one `LifetimeBVToParamMapping` per
`dyn` type the callee is sensitive to, each mapping listing
`(bv_idx, Option<callee_walk_pos>)` entries — one per binder variable
of that `dyn` type, with `None` standing for a binder variable pinned
to `'static`. All positions are expressed in *callee walk-order
space*. MIR-less intrinsic leaves (`trait_metadata_index`,
`trait_cast_is_lifetime_erasure_safe`, `trait_metadata_table`,
`trait_metadata_table_len`) have no body and are therefore absent from
the sensitivity map; they take the fallback branch covered below.

**Step 2 — compose through the `call_id` chain.** For MIR-backed
callees, the query calls `compose_all_through_chain(tcx, caller,
call_id, max_walk_pos)` to translate every callee walk-order position
that the sensitivity mentions into an *origin* walk-order position in
the outermost source body's own input space. The composer iterates the
chain from innermost link to outermost, concretizing each link's
edge-local `GenericArgsRef<'tcx>` template via
`instantiate_mir_and_normalize_erasing_regions` and resolving each
still-live position through
`borrowck_region_summary(body_def_id).call_site_mappings[local_id]`
and `VidProvenance`. The `Input` / `BoundedByUniversal` provenances
map to outer-caller `InputSlot`s via `build_template_input_slot_map`;
`Static` and `LocalOnly` provenances drop the position (writing
`None`). Entries may survive to the outermost link (yielding
`Some(origin_walk_pos)`) or be extinguished anywhere along the way
(`None`). When a link has no `call_site_mapping` but
post-monomorphization regions still exist — e.g. the edge is
`U = dyn Trait<'lt>` with `U` a caller type parameter — the composer
falls back to threading positions through the template itself.

**Step 3 — build the caller's outlives environment.** This step
produces a `CallerOutlivesEnv` — an oracle that answers "does region
`a` outlive region `b`?" for the caller's outlives relationships. A
`CallerOutlivesEnv` wraps a precomputed Floyd–Warshall reachability
`BitMatrix` (returned by the shared `outlives_reachability((entries,
dim))` query) together with an optional
`key_to_idx: FxHashMap<usize, usize>` that remaps caller-space keys to
matrix indices.

**`'static` convention.** Matrix index `dim - 1` is reserved for
`'static` throughout `CallerOutlivesEnv`. The user-visible
`usize::MAX` sentinel from §C.3.4 is folded onto `dim - 1` by
`CallerOutlivesEnv::resolve`, and any reachability edge whose
successor is `dim - 1` emits a `(bv, usize::MAX)` pair in Step 4, so
`'static` passes through unchanged.

Two constructors cover the two caller regimes:

- **Augmented caller.** When `caller.has_outlives_entries()` is true,
  the caller already carries its outlives evidence on its own
  `Instance` tail. `CallerOutlivesEnv::from_outlives_entries` reads
  `caller.outlives_indices_iter()`, sizes `dim` to `max_idx + 2` (one
  extra slot for `'static`), and feeds the pairs directly to
  `outlives_reachability`. `key_to_idx` is `None` because caller-space
  keys *are* matrix indices under this regime.
- **Ground-level caller.** Otherwise the caller is the outermost
  source body in its own input space. `caller_env_for_call_id` looks
  up the call-site `CallSiteRegionMapping` on
  `borrowck_region_summary(origin_def_id)`. If that mapping is missing
  (the origin is generic and its intrinsic args are type params whose
  regions only materialize after monomorphization) it returns an empty
  1-dimensional env. Otherwise it constructs
  `CallerOutlivesEnv::from_region_summary_walk_pos`: one matrix slot
  per SCC in `summary.outlives_graph.scc_successors` plus one for
  `'static`, seeded with the condensed SCC edges and cached through
  `outlives_reachability`, with `key_to_idx` translating each
  call-site walk position to its region-vid's SCC index.

**Step 4 — execute `augment_callee`.** `augment_callee` consumes the
callee sensitivity, the caller env, and the composed mapping, and
produces the final augmented `Instance` via
`callee_instance.with_outlives(tcx, &outlives_pairs)`. It runs in
three stages:

1. **Build nodes.** For every `LifetimeBVToParamMapping` in the callee
   sensitivity, for every `(bv_idx, Some(callee_walk_pos))` entry,
   look up `composed_mapping[callee_walk_pos]`; if it is
   `Some(caller_key)`, record `(bv_idx, caller_key)`. When
   `composed_mapping` is `None` (the fallback-identity path described
   below) the callee walk position is used directly as the caller key.
   Sort `nodes` by `bv_idx` and dedup — each binder variable
   contributes at most one caller-space key.
2. **Resolve and invert.** Resolve every node's caller-space key to a
   matrix index via `CallerOutlivesEnv::resolve`. Nodes whose keys
   fail to resolve (a walk position not present in the call-site
   mapping, for example) are dropped silently. Build
   `idx_to_bvs: matrix_idx → SmallVec<[bv_idx; 4]>`.
3. **Emit pairs.** For each resolved node `(bv_i, idx_i)`, iterate
   over every matrix index that `idx_i` outlives according to the
   caller env's reachability matrix. If the successor is the
   `'static` slot, emit `(bv_i, usize::MAX)`. For every other
   successor `idx_j`, emit `(bv_i, bv_j)` for every `bv_j` in
   `idx_to_bvs[idx_j]` with `bv_i != bv_j`. The row walk preserves
   reflexive hits, so two binder variables that alias onto the same
   caller key correctly produce a mutually-outliving Hamiltonian pair.
   Finally, `sort` and `dedup`.

This is O(N · dim) rather than the O(N²) naive pairwise probe,
load-bearing because dim is typically ≤ 10 but N can be large when a
callee is sensitive to many binder variables across many `dyn` types.

**MIR-less intrinsic fallback.** When Step 1's lookup returns `None`,
the query classifies the callee by intrinsic symbol and runs
`augment_callee` with a *synthetic* sensitivity standing in for the
absent MIR body, because the outlives information still has to come
from somewhere:

- **Augmented caller** (`caller.outlives_entries().len() > 1`). Strip
  the caller back to its base, re-run `items_of_instance` on the base
  to recover its direct sensitivity, wrap it with
  `CastRelevantLifetimes::from_direct_mappings`, and build the caller
  env from the augmented caller's own `Outlives` entries
  (`from_outlives_entries`). No composition is needed — the direct
  sensitivity is already in the caller's own space — so
  `composed_mapping` is `None` and `augment_callee` runs in identity
  mode.
- **Ground-level caller** (`caller.outlives_entries().len() <= 1`:
  base instance, or sentinel-only augmentation with no real pairs).
  Pull `borrowck_region_summary(origin_def_id).call_site_mappings[origin_local_id]`
  and synthesize `input_identity_sensitivity_for_call_site`: one
  `LifetimeBVToParamMapping` that maps each call-site walk position
  whose vid has `Input`, `BoundedByUniversal`, or `LocalOnly`
  provenance to itself, and drops `Static`-provenance positions.
  Build the caller env via `from_region_summary_walk_pos` against the
  same mapping and run `augment_callee` in identity mode. Any other
  (non-intrinsic) callee that falls through Step 1 — which should be
  unreachable given the sensitivity-map invariants — returns `&[]`.

Entries emitted on the MIR-less fallback path are in the origin call
site's walk-position or SCC space — *Space 1* per §C.3.4, not yet the
intrinsic's native binder-variable space. Remapping to Space 2 (or
Space 3 for the erasure-safety intrinsic) is done by the intrinsic
resolvers (`resolve_table_callee`, `resolve_erasure_safe_callee`)
before the intrinsic body is consulted.

**Sentinel handling.** `with_outlives` always prepends
`OUTLIVES_SENTINEL` to distinguish a zero-pair augmentation from a
base `Instance`. The query returns `&all[1..]` from
`augmented.outlives_entries()` when the tail is non-empty past the
sentinel, or `&[]` when only the sentinel is present. Callers that
want the full tail (including the sentinel) read
`augmented.outlives_entries()` directly off the returned `Instance`;
the query's return value is the shape consumed by the Phase-2 patcher,
which threads it back into `Instance::with_outlives` at each augmented
call site.

## C.4 New queries

### C.4.1 `codegen_mir`

The codegen-facing "get MIR body" query; the monomorphization collector
uses it to hand codegen patched bodies for outlives-sensitive instances.

```rust
/// Returns the MIR body to use for codegen of the given Instance.
/// Defaults to `instance_mir`, but may be overridden by the
/// monomorphization collector for outlives-sensitive instances
/// that need patched MIR with augmented callee references.
query codegen_mir(key: ty::Instance<'tcx>) -> &'tcx mir::Body<'tcx> {
    desc { "getting codegen MIR for `{}`", key }
    feedable
}
```

The default provider falls through to `tcx.instance_mir(instance.def)`.
For outlives-sensitive instances the global phase of partitioning feeds
a patched body via `tcx.feed_codegen_mir(instance, body)` just before
the instance is inserted into the final mono-item set, which takes
precedence over the fall-through provider. No `separate_provide_extern`
is needed — `Instance` is always resolved locally.

Codegen backends call `tcx.codegen_mir(instance)` in place of
`tcx.instance_mir(instance.def)`.

### C.4.2 `delayed_codegen_requests`

The per-crate list of delayed codegen requests described under §C.2.

```rust
/// Tracks which MIR bodies contain calls to trait casting intrinsics,
/// signaling that their codegen must be delayed until the global crate.
/// For the local crate, proxies into `collect_and_partition_mono_items`.
/// For upstream crates, decoded from metadata.
query delayed_codegen_requests(key: CrateNum) -> &'tcx [mir::mono::DelayedInstance<'tcx>] {
    separate_provide_extern
    desc { "tracking MIR bodies for delayed codegen" }
}
```

The value type is `&'tcx [DelayedInstance<'tcx>]`, not
`&'tcx [Instance<'tcx>]`: each entry carries the augmented-callee
substitution map and intrinsic-callee list required by the global
phase (see §C.2 above for the struct layout). The query is **not**
`feedable`. The local provider projects out of
`collect_local_mono_items(())`:

```rust
providers.queries.delayed_codegen_requests = |tcx, _key: LocalCrate| {
    tcx.collect_local_mono_items(()).delayed_codegen
};
```

The choice of `collect_local_mono_items` rather than
`collect_and_partition_mono_items` is load-bearing: it breaks a cycle
(`collect_and_partition_mono_items → gather_trait_cast_requests →
delayed_codegen_requests → collect_and_partition_mono_items`).

The extern provider decodes a per-crate
`LazyArray<DelayedInstance<'static>>` from rmeta into a fresh arena
slice.

### C.4.3 Global-phase queries

Beyond `codegen_mir` and `delayed_codegen_requests`, this RFC
introduces the following global-phase queries. Each is declared once
per compilation session and drives one step of the "gather → classify
→ layout → populate → emit" pipeline described under §C.3.

* `gather_trait_cast_requests(()) -> &'tcx TraitCastRequests<'tcx>`
  (arena-cached) — aggregates every crate's `delayed_codegen_requests`
  into classified buckets.
* `trait_cast_graph(root: Ty<'tcx>) -> &'tcx TraitGraph<'tcx>`
  (arena-cached) — the per-root-supertrait reachable trait graph over
  monomorphized nodes.
* `trait_cast_layout(root: Ty<'tcx>) -> &'tcx TableLayout<'tcx>`
  (arena-cached) — outlives-class condensation and per-slot assignment
  for the metadata tables rooted at `root`.
* `trait_cast_table(key: (Ty<'tcx>, Ty<'tcx>)) -> &'tcx [Option<AllocId>]`
  — populates the per-`(root, concrete)` slot vector with vtable
  `AllocId`s for admissible slots and `None` elsewhere.
* `trait_cast_table_alloc(key: (Ty<'tcx>, Ty<'tcx>)) -> AllocId` —
  emits the immutable per-`(root, concrete)` static that backs the
  metadata table.
* `global_crate_id_alloc(()) -> AllocId` — emits the single-byte
  static whose address serves as the global-crate identifier (see
  *Identity tokens* and *Appendix C §C.6*).
* `impl_universally_admissible(impl_def_id: DefId) -> bool` —
  fast-path admissibility check consumed by layout condensation.
* `outlives_reachability(key: (&'tcx [GenericArg<'tcx>], usize)) -> &'tcx BitMatrix<usize, usize>`
  (arena-cached) — Floyd–Warshall reflexive-transitive closure over a
  `dim`-dimensional index space, shared across layout, population, and
  erasure-safety checks.
* `is_lifetime_erasure_safe(key: (Ty<'tcx>, Ty<'tcx>, &'tcx [Option<usize>], &'tcx [GenericArg<'tcx>])) -> bool`
  — per-site erasure-safety result for a
  `(super_trait, target_trait, origin_positions, call_site_outlives)`
  tuple in walk-position space.
* `augmented_outlives_for_call((Instance<'tcx>, &'tcx List<(DefId, u32, GenericArgsRef<'tcx>)>, Instance<'tcx>)) -> &'tcx [GenericArg<'tcx>]`
  — per-call-site outlives-entry derivation that composes the
  `call_id` chain through the caller's outlives environment and
  returns the sentinel-stripped `Outlives` tail ready for
  `Instance::with_outlives`.
* `cast_relevant_lifetimes(Instance<'tcx>) -> Option<&'tcx CastRelevantLifetimes<'tcx>>`
  — per-Instance thin lookup into the crate-level map; returns `None`
  for non-sensitive Instances.
* `crate_cast_relevant_lifetimes(CrateNum) -> &'tcx UnordMap<Instance<'tcx>, CastRelevantLifetimes<'tcx>>`
  (`separate_provide_extern`) — the crate-level SCC-batch sensitivity
  map; the per-Instance query projects out of this one.

See also the two borrowck-side queries introduced alongside these and
described under §C.3:

* `borrowck_result(LocalDefId) -> &'tcx mir::BorrowckResult<'tcx>`.
* `borrowck_region_summary(DefId) -> &'tcx mir::BorrowckRegionSummary`
  (`separate_provide_extern`).

## C.5 Codegen

Codegen crate changes are minimal: use the new `codegen_mir` query
instead of `instance_mir`. `ty::Instance` uniqueness and hashing is
preserved.

## C.6 Identity token lowering

The contract described under *Identity tokens* requires each global
crate to emit a uniquely-addressed `&'static u8` and to keep that
address distinct across passes that could merge address-insignificant
constants. rustc satisfies this obligation via an `address_significant`
flag on allocations and a backend-specific lowering; a conforming
implementation is free to satisfy the contract by other means (e.g. a
per-crate sentinel symbol in a non-mergeable section) as long as no
backend or linker pass can defeat the uniqueness guarantee.

Concretely, the token is an `AllocId` created once per global crate by
the `global_crate_id_alloc` query (see §C.4.3). The allocation is a
1-byte immutable value (contents unspecified). All four trait-cast
intrinsics return this same `AllocId` as their first tuple element,
promoted to `&'static u8`.

Every allocation carries an `address_significant: bool` field,
defaulting to `false` and set to `true` only for the global-crate-id
allocation. The flag participates in allocation interning so that two
otherwise identical allocations are distinguished when one is
address-significant. Codegen backends observe the flag when lowering
`GlobalAlloc::Memory`:

* **LLVM:** emits the static with `UnnamedAddr::No`, suppressing
  `unnamed_addr`-based merging by `GlobalOpt`, LTO, and linker ICF.
  Without this, LLVM's default behavior on a private zero-byte constant
  would allow `GlobalOpt`, LTO, and downstream linker ICF to merge
  duplicate zero-byte globals across compilation units, violating the
  contract.
* **Cranelift and GCC:** no active address-merging pass exists at this
  layer today, so the flag is recorded but not currently acted on. A
  backend that later adds ICF-style merging must honor the flag (or
  implement an equivalent mechanism satisfying the backend obligation).
