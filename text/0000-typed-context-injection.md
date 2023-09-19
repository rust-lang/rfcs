- Feature Name: `typed_context_injection`
- Start Date: 2022-09-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

This RFC proposes the addition of a `Cx` structure to the `core` standard library alongside corresponding compiler support to allow Rust users to conveniently pass "bundles of context" around in their applications. `Cx` objects act as unordered tuples of references to objects the user wishes to pass around. `Cx` objects can be coerced into `Cx` objects containing a subset of their references.

# Motivation

[motivation]: #motivation

Barring interior mutability, Rust's object graph is tree shaped and forbids objects in the tree from creating references to ancestors, references to indirect descendants, and references to siblings.

In practice, this means two things:

1. Users must store object instances at the common ancestor of all its consumers.
2. To pass a reference pointing at an object near the root of the tree to a descendant deep in the tree, the user must manually forward that reference through the entire dispatch chain.

This reference forwarding is massively inconvenient for refactoring purposes. Every time the user wishes to grant a deeply nested object access to an ancestor object, they must modify many different places in the program to properly forward that reference to its children—even if those places don't end up using the reference directly!

```rust
pub struct GameEngine {
    audio_system: AudioSystem,  // New!
}

pub struct World {
    // ...
}

pub struct Player {
    // ...
}

pub struct Tool {
    // ...
}

impl GameEngine {
    pub fn update(&mut self) {
        // ...
        some_world.update(
            // ...
            &mut self.audio_system,  // New!
        );
    }
}

impl World {
    pub fn update(
        &mut self,
        // ...
        audio_system: &mut AudioSystem,  // New!
     ) {
        // ...
        some_player.update(
            // ...
            audio_system,  // New!
        );
    }
}

impl Player {
    pub fn update(
        &mut self,
        // ...
        audio_system: &mut AudioSystem,  // New!
     ) {
        // ...
        some_tool.update(
            // ...
            audio_system,  // New!
        );
    }
}

impl Tool {
    pub fn update(
        &mut self,
        // ...
        audio_system: &mut AudioSystem,  // New!
     ) {
        // We finally have access to the audio system!
        audio_system.play_sound("tool_use.ogg");
    }
}
```

Much of this complexity is irreducible, unfortunately. Although it may be tempting to just pass the entire `GameEngine` throughout the entire dispatch chain, doing so will prevent other systems from creating their own concurrent borrows to objects owned by the `GameEngine`.

```rust
impl World {
    pub fn update(engine: &mut GameEngine) {
        for player in engine.players.iter_mut() {
            player.update(engine);  // Whoops!
        }
    }
}
```

Worst of all, when users wish to split up an application system into several semi-dependent objects for the purpose of improving parallelism or code reuse, they must forward those new references throughout the entire context-passing chain, including in the call sites of the previously "atomic" methods!

```rust
// New!
pub struct AudioResourceLoader {}

// New!
pub struct AudioPlayer {}

// ...

pub struct GameEngine {
    audio_loader: AudioResourceLoader,  // Updated!
    audio_player: AudioPlayer,  // Updated!
}

impl GameEngine {
    pub fn update(&mut self) {
        // ...
        some_world.update(
            // ...
            &mut self.audio_loader,  // Updated!
            &mut self.audio_player,  // Updated!
        );
    }
}

impl World {
    pub fn update(
        &mut self,
        // ...
        audio_loader: &mut AudioResourceLoader,  // Updated!
        audio_player: &mut AudioPlayer,  // Updated!
     ) {
        // ...
        some_player.update(
            // ...
            audio_loader, audio_player,  // Updated!
        );
    }
}

impl Player {
    pub fn update(
        &mut self,
        // ...
        audio_loader: &mut AudioResourceLoader,  // Updated!
        audio_player: &mut AudioPlayer,  // Updated!
  ) {
        // ...
        some_tool.update(
            // ...
            audio_loader, audio_player,  // Updated!
        );
    }
}

impl Tool {
    pub fn update(
        &mut self,
        // ...
        audio_loader: &mut AudioResourceLoader,  // Updated!
        audio_player: &mut AudioPlayer,  // Updated!
    ) {
        // We finally have access to the audio system!

        // Oh no, our method got more tedious to use!
        audio_player.play_sound(audio_loader.load("tool_use.ogg"));

        // ...and we can't really simplify it in any meaningful way.
        audio_player.player_sound_from_path(&mut audio_loader, "tool_use.org");
    }
}
```

Of course, in passing all these objects through the dispatch chain, we are left with massive and very unwieldy function signatures:

```rust
// The full, unabridged Player signature.
impl Player {
    pub fn update(
        &mut self,
        // Some systems operate on just the position so we should probably separate those into their
        // own object.
        positions: &mut HashMap<ActorId, Position>,

        // Some systems operate on just the base information common to all items so we should separate
        // those into their own object as well.
        common_item_data: &mut HashMap<ItemId, CommonsItemData>,

        // Turns out, the world sometimes accesses tools directly without going through the player so
        // we need the world to own these and pass these along to the player.
        tools: &mut HashMap<ItemId, Tool>,

        // Oh yeah, these still need to be passed.
        audio_loader: &mut AudioResourceLoader,
        audio_player: &mut AudioPlayer,

        // Ugh.
    ) {
        // ...
    }
}
```

Yuck! It's no wonder most Rust applications prefer designs with such flat object hierarchies.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

This RFC proposes the introduction of the `Cx` object to the Rust `core` standard library to solve this problem. `Cx` is a type parameterized by a variadic list of either references to objects—which we will henceforth call "components"—or other `Cx` objects we wish to "inherit."

To define a function taking types `&System1`, `&System2`, and `&mut System3` as context, we simply write:

```rust
fn child_function(cx: Cx<&System1, &System2, &mut System3>) {
    // ...
}
```

To pass context to this `child_function`, we either have to construct the appropriate context from a tuple or coerce from a `Cx` object containing a superset of the required context.

```rust
fn parent_function(cx: Cx<&mut System1, &System2, &mut System3, &mut System4, &mut System5>) {
    // ...
    child_function(cx);
}

fn root_function() {
    // ...

    // Contexts can be constructed directly from references...
    let systems_4_and_5 = Cx::new(&mut system_4, &mut system_5);

    // ...or from a mix of references and other "inherited" contexts.
    parent_function(Cx::new((&mut system_1, &system_2, &mut system_3, systems_4_and_5)));
}
```

Elements can be extracted from a `Cx` instance by implicitly coercing it into its target reference type or by calling the `.extract::<T>()` or `.extract_mut::<T>()` methods:

```rust
fn child_function(cx: Cx<&System1, &System2, &mut System3>) {
    let system_1: &System1 = cx;
    let system_2 = cx.extract::<System2>();
    let system_3 = cx.extract_mut::<System3>();
}
```

This mechanism is, as expected, borrow-aware, so passing one element of the context to another method will not affect borrows on another:

```rust
fn borrow_demo(cx: Cx<&mut System1, &System2, &mut System3, &mut System4>) {
    let borrow_1: Cx<&mut System1, &System2> = cx;
    let borrow_2: Cx<&mut System3, &System2> = cx;

    // Both context objects can be used at the same time.
    let _ = (borrow_1, borrow_2);
}
```

A user can union the list of component types borrowed by several contexts together as follows:

```rust
fn parent_function(cx: Cx<&mut ParentSystem, ChildCx<'_>>) {
    // ...
    child_function(cx);
}

type ChildCx<'a> = Cx<&'a mut ChildSystem1, &'a mut ChildSystem2, DescendantCx1<'a>, DescendantCx2<'a>>;

fn child_function(cx: ChildCx<'_>) {
    // ...
    descendant_1_function(cx);
    descendant_2_function(cx);
}

type DescendantCx1<'a> = Cx<&'a mut DescendantSystem1, &'a mut DescendantSystem2>;

fn descendant_function_1(cx: DescendantCx1<'_>) {
    // ...
}

type DescendantCx2<'a> = Cx<&'a DescendantSystem1, &'a mut DescendantSystem3>;

fn descendant_function_2(cx: DescendantCx2<'_>) {
    // ...
}
```

As implied above, this mechanism is type based. It is important  to note that type equality is based off non-monomorphized information. Hence it is sound to write:

```rust
fn generics_demo<A, B>(cx: Cx<&mut A, &mut B>) {
    let borrow_1: Cx<&mut A> = cx;
    let borrow_2: Cx<&mut B> = cx;

    let _ = (borrow_1, borrow_2);
}
```

...even if `A` and `B` happen to take on the same type in a given parameterization of this function. We'll see later why this is sound.

In addition to being safely parameterizable by their component types, `Cx` can also be parameterized by a generic context type like so:

```rust
fn split_off<L: AnyCx, R: AnyCx>(cx: Cx<L, R>) -> (L, R) {
    // It is assumed that generic parameters will never alias, making this sound.
    (cx, cx)
}

pub fn map_mut<R: AnyCx, T: ?Sized, V: ?Sized>(cx: Cx<&mut T, R>, f: impl FnOnce(&mut T) -> &mut V) -> Cx<L, V> {
    let mapped = f(cx);  // Cx<&mut T, R> -> &mut T
    let rest: R = cx;  // Cx<&mut T, R> -> R

    // Once again, it is assumed that generic parameters will not overlap, making this safe.
    Cx::new((mapped, rest))
}
```

`Cx` is secretly a type alias to a second underlying type `CxRaw`. `Cx`'s sole role in this scheme is to perform a best-effort deduplication of component types and, like all other type aliases, this deduplication is performed eagerly and is only performed once.

Hence, when a user specifies `Cx<&mut u32, &mut u32>`, this `Cx` alias is transformed into `CxRaw<(&mut u32,)>`. However, if we have function generic with respect to the `T` and `V` type parameters, the parameter `Cx<&mut T, &mut V>` will be resolved to `CxRaw<(&mut T, &mut V)>` and will never change its definition, even if `T` and `V` end up being the same type:

```rust
// This signature:
fn generics_demo<A, B>(cx: Cx<&mut A, &mut B>) { }

// Is rewritten as:
fn generics_demo<A, B>(cx: CxRaw<(&mut A, &mut B)>) { }

// Meanwhile, the signature:
fn non_generic_demo(cx: Cx<&mut u32, &mut u32>) {}

// Is rewritten as:
fn non_generic_demo(cx: CxRaw<(&mut u32,)>) {}

// This means that the parameter of `generics_demo::<u32, u32>` takes on the type
// `CxRaw<(&mut u32, &mut u32)>`, which is distinct from the type of `non_generic_demo`, which
// takes on the type `CxRaw<(&mut u32,)>`.
```

`Cx` coercion can fail if there is any ambiguity in "where" a component should come from or where it should be put. In other words, coercing to or from a context with overlapping component types will fail. This error is fortunately pretty difficult to achieve in practice.

```rust
// We can't just demonstrate this directly like so:
let cx_1: Cx<&mut u32, &mut u32> = ...;
let cx_2: Cx<&mut u32> = cx_1;

// ...since `Cx`'s always eagerly attempt to deduplicate their component list!

// We can, however, force this duplication by creating the context tuple ourselves.
let mut value_1 = 3;
let mut value_2 = 4;
let cx_1 = Cx::new((&mut value_1, &mut value_2));

// This fails because `cx_1` genuinely contains more than one mutable reference.
let cx_2: Cx<&mut u32> = cx_1;
```

These two aforementioned rules explain why it sound to assume that distinct generic type parameters refer to distinct components in generic function bodies.

```rust
fn generics_demo<A, B>(cx: Cx<&mut A, &mut B>) {
    let borrow_1: Cx<&mut A> = cx;
    let borrow_2: Cx<&mut B> = cx;

    let _ = (borrow_1, borrow_2);
}

let mut value_1 = 3;
let cx_1: Cx<&mut u32> = Cx::new((&mut value1,));

// Cx<&mut A, &mut B> is substituted with CxRaw<(&mut u32, &mut u32)> in this invocation.
// Unlike the component list above, this component list is not deduplicated and, hence,
// we do indeed trigger an error about the ambiguity in "where" we should provide our `u32`
// reference.
let cx_2 = generics_demo::<u32, u32>(cx_1);

// A similar error can be observed with a bad call to `split_off`.
fn split_off<L: AnyCx, R: AnyCx>(cx: Cx<L, R>) -> (L, R) { ... }

let cx_1: Cx<&mut u32, &i32> = ...;

// Cx<L, R> is substituted with Cx<&mut u32, &i32, &mut u32>, which once again causes an
// ambiguity.
let (cx_l, cx_r) = split_off::<Cx<&mut u32, &i32>, Cx<&mut u32>>(cx_1);

// Finally, we can cause a source ambiguity like so:
fn reverse_generics_demo<'a, A, B>(a: &'a mut A, b: &'a mut B) -> Cx<&'a mut A, &'a mut B> {
    Cx::new((a, b))
}

let mut my_u32_1 = 3;
let mut my_u32_2 = 4;

// Here, Cx takes on the type Cx<&mut u32, &mut u32> as it is, once again, not subject to de-duplication.
let cx_1 = reverse_generics_demo::<u32, u32>(&mut my_u32_1, &mut my_u32_2);

// Which `u32` do we take? This is, again, an ambiguous coercion error.
let cx_2: Cx<&mut u32> = cx_1;
```

## Usage Examples

Using all the features described above, we can finally see how our nasty example described in the "motivation" section can be cleaned up significantly!

```rust
// New!
pub struct AudioResourceLoader {}

// New!
pub struct AudioPlayer {}

pub struct GameEngine {
    audio_loader: AudioResourceLoader,  // New!
    audio_player: AudioPlayer,  // New!
}

pub struct World {
    // ...
}

pub struct Player {
    // ...
}

pub struct Tool {
    // ...
}

impl GameEngine {
    pub fn update(&mut self) {
        // ...
        some_world.update(Cx::new((
            // ...
            &mut self.audio_system,  // New!
        )));
    }
}

type WorldCx<'a> = Cx<
    // ...
    PlayerCx<'a>,  // Unchanged!
>

impl World {
    pub fn update(&mut self, cx: WorldCx<'_>) {
        // ...
        some_player.update(cx);  // Unchanged!
    }
}

type PlayerCx<'a> = Cx<
    // ...
    ToolCx<'a>,  // Unchanged!
>;

impl Player {
    pub fn update(&mut self, cx: PlayerCx<'_>) {
        // ...
        some_tool.update(cx);  // Unchanged!
    }
}

type ToolCx<'a> = Cx<
    // ...
    &'a mut AudioResourceLoader,  // New!
    &'a mut AudioPlayer,  // New!
>;

impl Tool {
    pub fn update(&mut self, cx: ToolCx<'_>) {
        // We very quickly got access to the audio system!
        // ...and, oh, what's that?
        cx.play_sound_from_path("tool_use.ogg");
    }
}

// Ah, it's an extension method designed to make it easier to call `play_sound` with a file path!
pub trait AudioFsExt {
    fn play_sound_from_path(self, path: &str);
}

impl AudioFsExt for Cx<&'_ mut AudioResourceLoader, &'_ mut AudioPlayer> {
    fn play_sound_from_path(self, path: &str) {
        self.extract_mut::<AudioPlayer>().play_sound(
            self.extract_mut::<AudioResourceLoader>().load(path),
        );
    }
}
```

The mechanisms described above—with a little bit of help by the type bundle trick—can be used to pass generic context items as well:

```rust
// Define "bundle" traits for every type of generic component we're interested in
// accepting. 
trait HasLogger {
    type Logger: Logger;
}

trait HasDatabase {
    type Database: Database;
}

trait HasTracing {
    type Tracing: Tracing;
}

// Define the context for Child1
trait HasBundleForChildCx1: HasLogger + HasDatabase {}

type Child1Cx<'a, B> = Cx<
    &'a mut <B as HasLogger>::Logger>,
    &'a mut <B as HasDatabase>::Database,
>;

// Define the context for Child2
trait HasBundleForChildCx2: HasLogger + HasTracing {}

type Child2Cx<'a, B> = Cx<
    &'a mut <B as HasLogger>::Logger,
    &'a mut <B as HasDatabase>::Tracing,
>;

// Define the context for parent
type ParentCx<'a, B> = Cx<Child1Cx<'a, B>, Child2Cx<'a, B>>;

trait HasBundleForParent: HasBundleForChildCx1 + HasBundleForChildCx2 {}

// Define our functions
fn parent<B: ?Sized + HasBundleForParent>(cx: ParentCx<'_, B>) {
    let logger: &mut B::Logger = cx;
    let tracing &mut B::Tracing = cx;

    // This already works because of the no-alias assumptions
    // between distinct generic parameters.
    let _ = (logger, tracing);
}

fn caller(cx: Cx<&mut MyLogger, &mut MyDatabase, &mut MyTracing>) {
    consumer::<
        // We'll likely have to augment `Cx`'s inference system to allow
        // this to be inferred automatically.
        dyn HasBundleForParent<
            Logger = MyLogger,
            Database = MyDatabase,
            Tracing = MyTracing,
        >,
    >(cx);
}
```

How convenient!

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

Implementation of this RFC is split up into several parts:

1. A `CxRaw` lang-item struct which provides the actual coercion mechanisms.
2. An `AnyCx` lang-item trait which provides a blanket `impl` over all `CxRaw` structures and exposes a mechanism for reborrowing these opaque contexts.
3. A `Cx` intrinsic type alias which implements the eager deduplication mentioned above.
4. Standard library helpers methods.

We begin with the semantics of `CxRaw` and `AnyCx`. Here are their definition in the `core` standard library:

```rust
// Somewhere in `core`...
pub mod cx {
    mod cx_raw_unnamable {
        #[lang_item = "cx_raw"]  // Grants the special coercion semantics described below.
        pub struct CxRaw<T>(T);
    }

    use cx_raw_unnamable::CxRaw;

    impl<T> CxRaw<T> {
        pub fn new(value: T) -> Self {
            Self(value)
        }
    }

    // Implementations of this trait are generated by the compiler. Users should not be able
    // to derive this trait themselves.
    #[lang_item = "any_cx"]
    pub trait AnyCx: Sized {
        type Reborrowed<'a>: ReborrowedFrom<'a, Self>;
    }

    #[lang_item = "reborrowed_from"]
    pub trait ReborrowedFrom<'a, S: AnyCx>: AnyCx {
        #[unstable(
            feature = "cx_internals",
            reason = "this is an internal method called by the compiler when coercing `AnyCx` instances; to use this behavior, just coerce the `AnyCx` instance directly",
            issue = "none",
        )]
        fn reborrow(target: &'a mut S) -> Self;
    }
}
```

`AnyCx` is a trait which is automatically implemented for every `CxRaw` object which can be safely reborrowed. That is, in order for `CxRaw<T>` to implement `AnyCx`...

- `T` must be a tuple.
- Every element in this tuple must either be a reference, a mutable reference, or another instance implementing `AnyCx`.

The autogenerated `Reborrowed` type is derived as expected:

- The lifetime of (mutable) references are is replaced with the GAT lifetime parameter `'a`.
- `AnyCx` instances are reborrowed using their associated `Reborrow<'a>` gat.

`ReborrowedFrom` is another trait which is automatically implemented for every `CxRaw` whose components can be reborrowed from another target type `T` given a reference to it for lifetime `'a`. That is, in order for `CxRaw<T>` to implement `AnyCx` for the type `S` and the lifetime `'a`...

- `T` must be a tuple.
- `S` must be a tuple.
- The arity of `T` and `S` must match.
- Working element-wise, for every pair of elements `s` in `S` and `t` in `T`, either...
  - `t` and `s` are both references with the same pointee type. The reference mutability of `s` must be greater than or equal to that of `t`. The lifetime of `'a` must exceed the lifetime of the `t` reference and the lifetime of the `s` reference must outlive `'a`.
  - `t` implements `ReborrowedFrom<'a, s>`.

The autogenerated `reborrow` method is derived as expected:

- A new `CxRaw` is created form a tuple of the same arity as `S` and `T`. For every element in the tuple...
  - References are reborrowed using typical reference reborrowing.
  - `ReborrowedFrom` targets are reborrowed using the corresponding `reborrow` method.

The compiler then provides the coercion semantics for this type. Specifically, given a source type `A` and a target type `B`...

- If `A` and `B` are both ADTs which correspond to the `cx_raw` lang-item...
- ...and `A` and `B`'s sole generic parameters are provably tuples
- ...we can begin determining the coercion from one tuple to the other!
- Start by flattening the potentially nested tuples in `A` and `B` to a flat tuple of...
  - references whose pointee has been successfully inferred but potentially generic
  - instances implementing `AnyCx` which have been successfully inferred but potentially generic
  - *In the destination type*, at most one uninferred type implementing `AnyCx`.
  If this fails, ignore the coercion attempt.
- Now, determine a way to coerce the source to the target, refusing the coercion if this process turns up any ambiguities.
  - For every target pointee, try to find the unique corresponding pointee in the source context, keeping in mind that generics parameters unify *iff* they are constrained to be the same type. This process ignores lifetimes entirely.
    - If the number of candidate pointees is zero, reject the coercion attempt with a missing context hint on type check failure.
    - If the number of candidate pointees is greater than one, reject the coercion attempt with a source ambiguity hint on type check failure.
    - If the mutability of the destination is strictly greater than the source, reject the coercion attempt with an incompatible mutability hint on type check failure.
  - Ensure that, in establishing this mapping, we don't map the same source pointee to several target pointees. If we did indeed map the same source to several targets, we have a target ambiguity. In that case, we reject the coercion attempt with a destination ambiguity hint on type check failure.
  - For every generic type which implements `AnyCx` in the destination `CxRaw` tuple, match each to the corresponding `AnyCx` using a similar process as described above. Unification is determined by whether the target candidate type implements `ReborrowedFrom` against the source candidate type.
  - If we have a remaining uninferred type in the destination tuple implementing `AnyCx`, infer it to be the remaining elements of the source context which have not been borrowed by this coercion.
- If all this succeeds, produce a `Adjust::Context` adjustment detailing how to map from the source tuple to the target tuple.

The entire coercion is implemented in the THIR level by emitting the appropriate THIR to go from the source tuple to the target tuple. If the source expression is a place, access its subfields directly to avoid reborrowing the entire thing. Otherwise, bind the source to a temporary and access the subfields of that temporary.

Now, we can describe the semantics of the `Cx` type alias. Here is its definition in the `core` standard library:

```rust
// Somewhere in `core`...
pub mod cx {
    #[lang_item = "cx"]
    pub type Cx = ();  // (actual implementation provided by the compiler)
    //         ^ I'm not sure what to put here.
    //           If we have variadic type alias type parameters and they are somewhat functional,
    //           we could use that. Otherwise, we could define 12 existing type alias parameters
    //           each with a default of `CxRaw<()>` and pretend that this is essentially variadic.
}
```

As far as I can tell, type aliases are resolved fairly early in the compilation process. This is perfect for what we're doing.

When resolving a type alias...

- If the type alias corresponds to the `cx` lang item...

- Flatten all properly parameterized parameters of type `CxRaw` into the main list of components to borrow.

- Perform a best-effort deduplication of all known reference types based off their inferred pointees. The "strongest" mutability is used as the mutability of the deduplicated pointer. The check for type equality used to determine duplicates, unlike the type equality check for context passing, is *not* lifetime blind and, if one lifetime is not known to be longer than another, the two types will be considered unequal and therefore will not be deduplicated. We do, however, consider generic parameter equality opaquely.

- Perform a best-effort deduplication of all other arguments. This deduplication is also sensitive to lifetimes.

- Return a `CxRaw` ADT parameterized with a tuple of these deduplicated types.

Finally, we implement the following extension methods in the `core` standard library:

```rust
impl<'a, T: ?Sized> Cx<&'a T> {
    pub fn extract(self) -> &'a T {
        self
    }
}

impl<'a, T: ?Sized> Cx<&'a mut T> {
    pub fn extract_mut(self) -> &'a mut T {
        self
    }
}

impl<'a, R: AnyCx, T: ?Sized> Cx<R, &'a T> {
    pub fn map<V: ?Sized>(self, f: impl FnOnce(&T) -> &V) -> Cx<R, &'a V> {
        let mapped = f(self);
        let rest: R = self;

        Cx::new((mapped, rest))
    }
}

impl<'a, R: AnyCx, T: ?Sized> Cx<R, &'a mut T> {
    pub fn map_mut<V: ?Sized>(self, f: impl FnOnce(&mut T) -> &mut V) -> Cx<R, &'a mut V> {
        let mapped = f(self);
        let rest: R = self;

        Cx::new((mapped, rest))
    }
}

impl<L: AnyCx, R: AnyCx> Cx<L, R> {
    pub fn split(self) -> (L, R) {
        (self, self)
    }
}
```

---

==TODO: Is this finished?==

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks

[drawbacks]: #drawbacks

==TODO==

Why should we *not* do this?

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

==TODO==

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

# Prior art

[prior-art]: #prior-art

==TODO==

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

==TODO==

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities

[future-possibilities]: #future-possibilities

==TODO==

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
