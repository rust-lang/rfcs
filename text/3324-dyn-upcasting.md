- Feature Name: `trait_upcasting`
- Start Date: 2022-12-10
- RFC PR: [rust-lang/rfcs#3324](https://github.com/rust-lang/rfcs/pull/3324)
- Rust Issue: [rust-lang/rust#65991](https://github.com/rust-lang/rust/issues/65991)
- Design repository: [rust-lang/dyn-upcasting-coercion-initiative](https://github.com/rust-lang/dyn-upcasting-coercion-initiative)

# Summary
[summary]: #summary

Enable upcasts from `dyn Trait1` to `dyn Trait2` if `Trait1` is a subtrait of `Trait2`. 

This RFC does not enable `dyn (Trait1 + Trait2)` for arbitrary traits. If `Trait1` has multiple supertraits, you can upcast to any one of them, but not to all of them.

This RFC has already been implemented in the nightly compiled with the feature gate `trait_upcasting`.

# Motivation
[motivation]: #motivation

If you define a trait with a supertrait

```rust
trait Writer: Reader { }

trait Reader { }
```

you can currently use `impl Writer` anywhere that `impl Reader` is expected:

```rust
fn writes(w: &mut impl Writer) {
    reads(w);
}

fn reads(r: &mut impl Reader) {
    
}
```

but you cannot do the same with `dyn`

```rust
fn writes(w: &mut dyn Writer) {
    reads(w); // <-- Fails to compile today
}

fn reads(r: &mut dyn Reader) {
    
}
```

The only upcasting coercion we permit for dyn today is to remove auto-traits; e.g., to coerce from `dyn Writer + Send` to `dyn Writer`.

## Sample use case

One example use case comes from the [salsa](https://github.com/salsa-rs/salsa) crate. Salsa programs have a central database but they can be broken into many modules. Each module has a trait that defines its view on the final database. So for example a parser module might define a `ParserDb` trait that contains the methods the parser needs to be present. All code in the parser module then takes a `db: &mut dyn ParserDb` parameter; `dyn` traits are used to avoid monomorphization costs.

When one module uses another in Salsa, that is expressed via supertrait relationships. So if the type checker module wishes to invoke a parser, it might define its `trait TypeCheckerDb: ParserDb` to have the `ParserDb` as a supertrait. The methods in the type checker then take a `db: &mut dyn TypeCheckerDb` parameter. If they wish to invoke the `ParserDb` methods, they would ideally be able to pass this `db` parameter to the parser methods and have it automatically upcast. This does not work with today's design, requiring elaborate workarounds.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When a trait is declared, it may include various supertraits. Implementing the trait also requires implementing each of its supertraits. For example, the `Sandwich` trait has both `Eat` and `Grab` as supertraits:

```rust
trait Eat { fn eat(&mut self); }
trait Grab { fn grab(&mut self); }
trait Sandwich: Food + Grab { }
```

Therefore, any type that implements `Sandwich` must also implement `Eat` and `Grab`.

`dyn Trait` values may be coerced from subtraits into supertraits. A `&mut dyn Sandwich`, for example, can be coerced to a `&mut dyn Eat` or a `&mut dyn Grab`. This can be done explicitly with the `as` operator (`sandwich as &mut dyn Grab`) or implicitly at any of the standard coercion locations in Rust:

```rust
let s: &mut dyn Sandwich = ...;
let f: &mut dyn Food = s; // coercion
takes_grab(s); // coercion

fn takes_grab(g: &mut dyn Grab) { }
```

These coercions work for any kind of "pointer-to-dyn", such as `&dyn Sandwich`, `&mut dyn Sandwich`, `Box<dyn Sandwich>`, or `Rc<dyn Sandwich>`.

Note that you cannot, currently, upcast to *multiple* supertraits. That is, an `&mut dyn Sandwich` can be coerced to a `&mut dyn Food` or a `&mut dyn Grab`, but `&mut (dyn Food + Grab)` is not yet a legal type (you cannot combine two arbitrary traits) and this coercion is not possible.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Changes to coercion rules

The `Unsize` trait is the (unstable) way that Rust controls coercions into unsized values. We currently permit `dyn Trait1: Unsize<dyn Trait2>` precisely for the case where there is the same "principal trait" (i.e., non-auto-trait) and the set of auto-traits differ. This RFC extends that coercion to permit `dyn Trait1` to be unsized to `dyn Trait2` if `Trait2` is a (transitive) supertrait of `Trait1`.

The *supertraits* of a trait `X` are defined as any trait `Y` such that `X` has a where-clause `where Self: Y` (note that `trait X: Y` is short for `trait X where Self: Y`). This definition already exists in the compiler, and we already prohibit the supertrait relationship from being cyclic.

Note that this is a *coercion* and not a *subtyping* rule. That is observable because it means, for example, that `Vec<Box<dyn Trait>>` cannot be upcast to `Vec<Box<dyn Supertrait>>`. Coercion is required because vtable cocercion, in general, requires changes to the vtable, as described in the vtable layout section that comes next.

## Expected vtable layout

**This RFC does not specify the vtable layout for Rust dyn structs.** Nonetheless, it is worth discussing how this proposal can be practically implemented. Therefore, we are describing the current implementation strategy, though it may be changed in the future in arbitrary ways.

Given Rust's flexible subtrait rules, coercing from a `&dyn Trait1` to `&dyn Trait2` may require adjusting the vtable, as we cannot always guarantee that the vtable layout for `Trait2` will be a prefix of `Trait1`.

This currently implemented design was proposed by `Mario Carneiro` based on previous proposals on [Zulip discussion](https://zulip-archive.rust-lang.org/stream/243200-t-lang/major-changes/topic/Trait.20Upcasting.20lang-team.2398.html#242876426). It's a hybrid approach taking the benefits of both a "flat" design, and a "pointer"-based design.

This is implemented in [#86461](https://github.com/rust-lang/rust/pull/86461).

The vtable is generated by this algorithm in principle for a type `T` and a trait `Tr`:
1. First emit the header part, including `MetadataDropInPlace`, `MetadataSize`, `MetadataAlign` items.
2. Create a tree of all the supertraits of this `TraitRef`, by filtering out all of duplicates.
3. Collect a set of `TraitRef`s consisting the trait and its first supertrait and its first supertrait's super trait,... and so on. Call this set `PrefixSet`
4. Traverse the tree in post-order, for each `TraitRef` emit all its associated functions as either `Method` or `Vacant` entries. If this `TraitRef` is not in `PrefixSet`, emit a `TraitVPtr` containing a constant pointer to the vtable generated for the type `T` and this `TraitRef`.

### Example

```rust
trait A {
    fn foo_a(&self) {}
}

trait B: A {
    fn foo_b(&self) {}
}

trait C: A {
    fn foo_c(&self) {}
}

trait D: B + C {
    fn foo_d(&self) {}
}
```

```text
Vtable entries for `<S as D>`: [
    MetadataDropInPlace,
    MetadataSize,
    MetadataAlign,
    Method(<S as A>::foo_a),
    Method(<S as B>::foo_b),
    Method(<S as C>::foo_c),
    TraitVPtr(<S as C>),
    Method(<S as D>::foo_d),
]

Vtable entries for `<S as C>`: [
    MetadataDropInPlace,
    MetadataSize,
    MetadataAlign,
    Method(<S as A>::foo_a),
    Method(<S as C>::foo_c),
]
```

## Implications for unsafe code

One of the major points of discussion in this design was what validity rules are required by unsafe code constructing a `*mut dyn Trait` raw pointer. The full detail of the discussion are [documented on the design repository](https://github.com/rust-lang/dyn-upcasting-coercion-initiative/blob/master/design-discussions/upcast-safety-3.md). This RFC specifies the following hard constraints:

* **Safe code can upcast:** Rust code must be able to upcast `*const dyn Trait` to `*const dyn Supertrait`.
    * This implies the safety invariant for raw pointers to a `dyn Trait` requires that they have a valid vtable suitable for `Trait`.
* **Dummy vtable values can be used with caution:** It should be possible to create a `*const dyn SomeTrait` with *some* kind of dummy value, so long as this pointer does not escape to safe code and is not used for upcasting.

This RFC does not specify the validity invariant, instead delegating that decision to the ongoing operational semantics work. One likely validity invariant is that the vtable must be non-null and aligned, which both preserves a niche and is consistent with other values (like `fn` pointers).

# Drawbacks
[drawbacks]: #drawbacks

## Larger vtables

Although the precise layout of vtables is not stabilized in this RFC (and is never expected to be), adopting this feature does imply that vtables must *somehow* support upcasting. For "single-inheritance" scenarios, where traits have a single supertrait, this is not an issue, but for "multiple inheritance" scenarios, where traits have multiple supertraits, it may imply that vtables become larger. Under the current vtable design, we generate one additional vtable for each supertraits after the first. This leads to larger binaries, which can be an issue for some applications (particularly embedded).

Note that the we are already generating the larger vtables as of Rust 1.56, in anticipation of adopting this RFC. We do not have data about real-world impact, but some synthetic benchmarks have been generated. [afetisov writes:](https://github.com/rust-lang/rfcs/pull/3324#issuecomment-1308124173)

> I don't have any data from real-world projects, but I have made a test crate, which uses proc macro to generate a graph of traits and impls with width W and depth D, as in my example above. At least when generating rlibs, I did not see any exponential blowup of artifact size, which I predicted above. The rlib size seemed to grow roughly linearly in W and D.

It's not entirely clear why this is, however, and more investigation may be warranted.

## Multi-trait dyn is more complex

As described in the Future Possibilities section, if we move to support `dyn Foo + Bar + Baz` for arbitrary sets of traits, we would likely also want to support upcasting to arbitrary subsets (e.g., `Foo + Bar`, `Bar + Baz`, or `Foo + Baz`). This potentially requires a large number of vtables to be generated in advance, since we cannot know which sets of supertraits users will want to upcast to.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Why not mandate a "flat" vtable layout?

An alternative vtable layout would be to use a "flat" design, in which the vtables for all supertraits are embedded within the subtrait. Per the text of this RFC, we are not specifying a precise vtable layout, so it remains an option for the compiler to adopt a flat layout if desired (and the compiler currently does so, for the first supertrait only). Another option would be to mandate that flat layouts are ALWAYS used. This option was rejected because it can lead to exponential blowup of the vtable.

Consider a flat layout algorithm for a type `T` and a trait `Tr` as follows:

1. Create a tree of all the supertraits of this `TraitRef`, duplicate for the cyclic cases.
2. Traverse the tree in post-order, for each `TraitRef`, 
   1. if it has no supertrait, emit a header part, including `MetadataDropInPlace`, `MetadataSize`, `MetadataAlign` items.
   2. emit all its associated functions as either `Method` or `Vacant` entries.

Given `trait A(n+1): Bn + Cn {}, trait Bn: An { fn bn(&self); }, trait Cn: An { fn cn(&self); }`, the vtable for An will contain 2^n DSAs.

## Why not adopt a "pointer-based" vtable layout?

The current implementation uses a hybrid strategy that *sometimes* uses pointers. This was deemed preferable to using a *purely* pointer-based layout because it would be less efficient for the single-inheritance case, which is common.

## Are there other optimizations possible with vtable layout?

Certainly. Given that the RFC doesn't specify vtable layout, we still have room to do experimentation. For example, we might do special optimizations for traits with no methods.

## Why not make upcasting opt-in at a trait level?

The current proposal always permits upcasting from a trait to its supertraits. This implies however that when creating a `dyn Trait` vtable we must always allow for the possibility of an upcast, unless we can somehow prove that this particular dyn will never be upcast (we currently make no effort to "trim" vtables, although it is theoretically possible with "link-time-optimization"). One alternative would be to make upcasting opt-in, perhaps at a trait level. This has the advantage that adding a supertrait does not cause a larger vtable unless the trait "opts in" to upcasting, but the disadvantage of imposing additional complexity on users. Library authors would have to anticipate whether users may wish to upcast, and it is likely that failure to add such an annotation would be a frequent irritation. Furthermore, for the vast majority of use-cases, the additional binary size from supporting upcasting is minimal and not a problem. 

Apart from the complexity problem, it is not obvious that the trait level is the right place to opt-in to upcasting. It's unclear what guidance we would give to a user authoring a trait to indicate when they should enable opt-in, apart from "if you anticipate users wishing to upcast" (which of course begs the question, when would I anticipate upcasting?).

## Why not make add a lint if traits would permit upcasting?

Another proposal is to add a lint for traits that have multiple supertraits but which are not dyn safe, since they may require larger vtables. An allow-by-default lint may be acceptable, to help users identify this case if they should wish, but this RFC recommends against a warn-by-default lint. If we believe that larger vtables are enough of a problem to warn against multiple supertraits, we should prefer to make upcasting opt-in or to take some other approach to solve the problem.

# Prior art
[prior-art]: #prior-art

Other languages permit upcasting in similar scenarios.

C++ permits upcasting from a reference to a class to any of its superclasses. As in Rust, this may require adjusting the pointer to account for multiple inheritance.

Java programs can upcast from an object to any superclass. Since Java is limited to single inheritance, this does not require adjusting the pointer, but this implies that interfaces are harder.

[Haskell `forall` types permit upcasting](https://wiki.haskell.org/Existential_type#Dynamic_dispatch_mechanism_of_OOP).

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Should we make upcasting opt-in in some form to limit vtable size by default? The current inclination of the lang-team is "no", but it would be useful to gather data on how much supporting upcasting contributors to overall binary size.

# Future possibilities
[future-possibilities]: #future-possibilities

## Arbitrary combinations of traits

It would be very useful to support `dyn Trait1 + Trait2` for arbitrary sets of traits. Doing so would require us to decide how to describe the vtable for the combination of two traits. There is an intefaction between this feature and upcasting, because if we support upcasting, then we must be able to handle upcasting from some subtrait to some arbitrary combination of supertraits. For example a `&mut dyn Subtrait`...

```rust=
trait Subtrait: Supertrait1 + Supertrait2 + Supertrait3
```

...could be upcast to any of the following:

* `&mut dyn Supertrait1` (covered by this RFC)
* `&mut dyn Supertrait2` (covered by this RFC)
* `&mut dyn Supertrait3` (covered by this RFC)
* `&mut dyn (Supertrait1 + Supertrait2)` (not covered by this RFC)
* `&mut dyn (Supertrait2 + Supertrait3)` (not covered by this RFC)
* `&mut dyn (Supertrait1 + Supertrait3)` (not covered by this RFC)
* `&mut dyn (Supertrait1 + Supertrait2 + Supertrait3)` (not covered by this RFC)

In particular, this implies that we must be able to go from the vtable for `Subtrait` to any of the above vtables. 

Two ways have been proposed thus far to implement a "multi-trait" dyn like `dyn Trait1 + Trait2`...

* as a single, combined vtable
* as a "very wide" pointer with one vtable per trait

To support "arbitrary combination upcasting", the former would require us to precreate all the vtables the user might target in advance (as you can see, that's an exponential number). On the other hand, the latter design makes `dyn` values take up a lot of bits, and the current wide pointers are already a performance hazard in some scenarios. 

These challenges are inherent to the design space and not made harder by this RFC, except in so far as it commits to supporting upcasting.

## Sufficient safety conditions for raw pointer method dispatch

In the future we expect to support traits with "raw pointer" methods:

```rust
trait IsNull {
    fn is_null(*const Self) -> bool;
}
```

For this to work, invoking `n.is_null()` on a `n: *const dyn IsNull` must have a valid vtable to use for dispatch. This condition is guaranteed by this RFC.

## Allow traits to "opt out" from upcasting

We could add an option allowing traits to opt-out from upcasting. Adding this option to a trait would be a semver-breaking change, as consumers may already have been taking advantage of upcasting. Adding such an option to the language, however, is a pure extension and can be done at any time.

## Optimizations or options to trim binary size

The primary downside of this RFC is that it requires larger vtables, which can be a problem for some applications. Vtables are of course only one contributor to overall binary sizes (and we don't have data to indicate how large of a contributor they are). To get an idea of other sources, take a look at [min-sized-rust](https://github.com/johnthagen/min-sized-rust), a repository which documents a Best Practices workflow for reducing Rust binary size.

Looking forward, there are at least two potential ways we could address this problem:

* Optimization to remove unused parts of vtables: When generating a final binary artifact, we could likely reduce the size of vtables overall by analyzing which methods are invoked and which upcast slots are used. Unused slots could be made NULL, which may enable additional dead code elimination as well. This would require some rearchitecture in the compiler, since LTO currently executes at the LLVM level, and this sort of analysis would be much easier to do at the MIR level; no language changes are required, however.
* Target options to disable upcasting or other "space hogs": We could extend compilation profiles to allow targets to disable upcasting, either always or for select traits. This would lead to a compilation error if crates used upcasting, but permit generating smaller binaries (naturally, all crates being compiled would have to be compiled with the same target options).

Another option, though one that this RFC recommends against, would be to add a new form of `dyn` that does not support upcasting (e.g., `dyn =Trait` or some such). This would allow individual values to "opt out" from upcasting.

