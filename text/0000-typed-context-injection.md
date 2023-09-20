- Feature Name: `typed_context_injection`
- Start Date: 2022-09-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC proposes the addition of a `Cx` structure to the `core` standard library alongside corresponding compiler support to allow Rust users to conveniently pass "bundles of context" around their applications. `Cx` objects are a thin wrapper around tuples of references which adorn the wrapped tuple with the ability to coerce into other `Cx` objects containing a subset of their references.

# Motivation
[motivation]: #motivation

As it stands, Rust has no mechanism for conveniently passing large amounts of context to a function deep in the call stack. To inject a component of type `NewSystem`, for example, one must manually forward that reference throughout the entire dispatch chain.

```rust
fn func_1(..., new_system: &mut NewSystem) {  // Updated!
    ...
    func_2(..., new_system);  // Updated!
}

fn func_2(..., new_system: &mut NewSystem) { // Updated!
    ...
    func_3(..., new_system);  // Updated!
}

fn func_3(..., new_system: &mut NewSystem) { // Updated!
    // We finally have access to `new_system`
}
```

Although one may be tempted to bundle up this context into convenient tuples to reduce the amount of explicit context passing, doing so will only work if only one descendant of the call stack relies upon that type. If several descendants rely on a given contextual type, you're out of luck!

```rust
type Descendant1Cx<'a> = (&'a mut SharedSystem, &'a mut Descendant1System);

fn descendant_1(cx: Descendant1Cx<'_>) {
    ...
}

type Descendant2Cx<'a> = (&'a mut SharedSystem, &'a mut Descendant2System);

fn descendant_2(cx: Descendant2Cx<'_>) {
    ...
}

fn callee(descendant_1_cx: Descendant1Cx<'_>, descendant_2_cx: Descendant2Cx<'_>) {
    descendant_1(descendant_1_cx);
    descendant_2(descendant_2_cx);
}

fn caller(
    shared_system: &mut SharedSystem,
    descendant_1_system: &mut Descendant1System,
    descendant_2_system: &mut Descendant2System,
) {
    callee(
        (shared_system, descendant_1_system),
        (shared_system, descendant_2_system),  // Whoops! We borrowed shared_system twice!
    );
}
```

Finally, although good software practices encourage programmers to keep their abstractions minimal and properly-decoupled, doing so only causes grief when trying to implement operations acting upon several of these well-split-up components:

```rust
pub struct AudioBuffer { ... }

impl AudioBuffer {
    pub fn play_samples(&mut self, data: &[f32], volume: f32) { ... }
}

pub struct VirtualFileSystem { ... }

impl VirtualFileSystem {
    pub fn load_file(&mut self, path: &str) -> &[u8] { ... }
}

pub struct AudioCache { ... }

impl AudioCache {
    pub fn resolve_from_cache(&mut self, path: &str, loader: impl FnOnce() -> &[u8]) -> &[f32] { ... }
}

pub struct SoundVolumeOverrides { ... }

impl SoundVolumeOverrides {
    pub fn get_volume_override(path: &str) -> f32 { ... }
}

// Ideally, we could just write:
cx.play_sound_at_path("cbat.ogg");

// But, unfortunately, we must write:
audio_buffer.play_sound_at_path(&mut virtual_fs, &mut audio_cache, &mut sound_volume_overrides, "cbat.ogg");
```

Yuck!

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Enter typed context injection. 

Context injection is accomplished with two new standard library items: `Cx` and `AnyCx`. `Cx` is a variadic type parameterized by the references comprising a function's context. For example, to write a function taking `&mut System1` and `&System2`, you can just write:

```rust
fn consumer(cx: Cx<&mut System1, &System2>) { ... }
```

From there, you can fetch your components by either coercing the context to its comprising elements (which we'll henceforth refer to as *components*) or by using the `.extract::<T>()` utility methods, which perform the coercion internally but expose it in an occasionally-more-convenient method form.

```rust
fn consumer(cx: Cx<&mut System1, &System2>) {
    let system_1: &mut System1 = cx;
    let system_2: &System2 = cx;

    // ...which is equivalent to:
    let system_1 = cx.extract_mut::<System1>();
    let system_2 = cx.extract::<System2>();
}
```

To provide a `Cx` to a method, you can either create it with the `Cx::new` constructor or coerce a superset context into the target subset of itself.

```rust
fn caller_1(cx: Cx<&mut System1, &mut System2, &System3>) {
    // `cx` has a mutable reference to `System1` and (at least) an immutable reference to `System2`
    // so this is fine. The unnecessary `&System3` reference is completely ignored.
    consumer(cx);
}

fn caller_2() {
    ...

    // We can give the context in the order it shows up in the target type.
    consumer(Cx::new((&mut system_1, &system_2)));

    // But we could also provide the context in any other order and let coercion
    // turn e.g. a `Cx<&mut System2, &mut System1>` into a `Cx<&mut System1, &System2>`.
    consumer(Cx::new((&mut system_2, &mut system_1)));

    // Heck, we could even pass the function additional completely unrelated components and coercion
    // would still have our back!
    //
    // Coerces `Cx<&mut SomeOtherSystem, &mut System2, &mut System1>` into
    // `Cx<&mut System1, &System2>`.
    consumer(Cx::new((&mut some_other_system, &mut system_2, &mut system_1)));

    // And if that flexibility wasn't enough, we can combine several contexts together to
    // construct a full context like so:
    let inherited_cx = Cx::new((&mut some_other_system, &mut system_2));

    // Coerces `Cx<Cx<&mut SomeOtherSystem, &mut System2>, &mut System1>` into
    // `Cx<&mut System1, &System2>`.
    consumer(Cx::new((inherited_cx, &mut system_1)));
}
```

Of course, this process is entirely borrow aware. When we coerce a context, only the components needed by the coercion are borrowed. All other components are completely untouched by the process.

```rust
fn example(cx: Cx<&mut CommonSystem, &mut System1, &mut System2, &mut OtherSystem>) {
    let borrow_1: Cx<&CommonSystem, &mut System1> = cx;
    let borrow_2: Cx<&CommonSystem, &mut System2> = cx;

    // Both borrows can overlap.
    let _ = (borrow_1, borrow_2);

    // ...because the coercions desugar to the following:

    // `CxRaw<T>` is an implementation detail we'll talk about in a bit. For right now,
    // just think of it as a newtype around an arbitrary tuple type `T`.
    let borrow_1: CxRaw<(&CommonSystem, &mut System1)> = CxRaw::new((
        &self.0.0,
        &mut self.0.1,
    ));
    let borrow_2: CxRaw<(&CommonSystem, &mut System2)> = CxRaw::new((
        &self.0.0,
        &mut self.0.2,
    ));

    let _ = (borrow_1, borrow_2);
}
```

We can use these features to implement a much more convenient version of the `play_sound_at_path` method demonstrated in the [motivation section](#motivation).

```rust
pub trait AudioBufferCxExt {
    fn play_sound_at_path(self, path: &str);
}

type AudioBufferCx<'a> = Cx<
    &'a mut AudioBuffer,
    &'a mut VirtualFileSystem,
    &'a mut AudioCache,
    &'a mut SoundVolumeOverrides,
>;

impl AudioBufferCxExt for AudioBufferCx<'_> {
    fn play_sound_at_path(self, path: &str) {
        self.extract_mut::<AudioBuffer>().play_samples(
            self.extract_mut::<AudioCache>().resolve_from_cache(
                path,
                || self.extract_mut::<VirtualFileSystem>().load_file(path),
            ),
            self.extract_mut::<SoundVolumeOverrides>().get_volume_override(path),
        );
    }
}

let cx: AudioBufferCx<'_> = ...;

// And now it just works!
cx.play_sound_at_path("cbat.ogg");
```

This feature is great at reducing the amount of typing required to pass context to function but does nothing to reduce the pain of propagating context components throughout a dispatch chain. To fix this problem, we have to look into the second super power of `Cx`: nesting and deduplication.

In addition to containing references and mutable references in its variadic parameters, `Cx` can also contain other `Cx` types, which it inherits into its own context. The flattened component list is then deduplicated to ensure that only one instance of each type is requested.

With these two features, it is now possible to rewrite our second example in the [motivation section](#motivation) to be convenient to refactor:

```rust
// We just have to change this line from a tuple to a `Cx`.
type Descendant1Cx<'a> = Cx<&'a mut SharedSystem, &'a mut Descendant1System>;

fn descendant_1(cx: Descendant1Cx<'_>) {
    ...
}

// Same here!
type Descendant2Cx<'a> = Cx<&'a mut SharedSystem, &'a mut Descendant2System>;

fn descendant_2(cx: Descendant2Cx<'_>) {
    ...
}

// We also need to give the callee its own context.
type CalleeCx<'a> = Cx<Descendant1Cx<'a>, Descendant2Cx<'a>>;

fn callee(cx: CalleeCx<'_>) {
    descendant_1(cx);
    descendant_2(cx);
}

fn caller(
    shared_system: &mut SharedSystem,
    descendant_1_system: &mut Descendant1System,
    descendant_2_system: &mut Descendant2System,
) {
    callee(Cx::new((
        // ...and now we only have to provide one reference to `shared_system`.
        shared_system,
        descendant_1_system,
        descendant_2_system,
    )));
}
```

So far, this system works great for concrete types but how does it fare for generics?

Consider the following simple example:

```rust
fn generic_demo<T, V>(cx: Cx<&mut T, &mut V>) {
    let a: &mut T = cx;
    let b: &mut V = cx;
    let _ = (a, b);
}
```

Should this be accepted? This proposes says yes: generic parameters should be assumed distinct.

But how does the proposal handle misuse like this:

```rust
let mut value = 3u32;
generic_demo::<u32, u32>(Cx::new((&mut value,)));  // Oh no!
```

Isn't this going to cause undefined behavior in the `generic_demo` function body?

To understand why this is sound, we need to dive deeper into the specific semantics of `Cx`. `Cx` is not an actual type; instead, it's a special type alias resolving to some parametrization of the type `CxRaw<T>`. `CxRaw<T>`, meanwhile, is just a thin newtype wrapper around a tuple of type `T` which is given special treatment by the Rust compiler to allow it to coerce with the semantics described above. `Cx`'s sole role in this scheme is to perform a best-effort deduplication of component types and, like all other type aliases, this deduplication is performed eagerly and is only performed once.

Hence, when a user specifies `Cx<&mut u32, &mut u32>`, this `Cx` alias is transformed into `CxRaw<(&mut u32,)>`. However, if we have function generic with respect to the `T` and `V` type parameters, the parameter `Cx<&mut T, &mut V>` will be resolved to `CxRaw<(&mut T, &mut V)>` and will never change its definition, even if `T` and `V` end up being the same type in a given monomorphization:

```rust
// This signature:
fn generics_demo<A, B>(cx: Cx<&mut A, &mut B>) { }

// Is rewritten as:
fn generics_demo<A, B>(cx: CxRaw<(&mut A, &mut B)>) { }

// Meanwhile, the signature:
fn non_generic_demo(cx: Cx<&mut u32, &mut u32>) {}

// Is rewritten as:
fn non_generic_demo(cx: CxRaw<(&mut u32,)>) {}

// This means that the parameter of `generics_demo::<u32, u32>` takes a parameter of the type
// `CxRaw<(&mut u32, &mut u32)>`, which is distinct from the type of `non_generic_demo`'s parameter,
// which takes on the type `CxRaw<(&mut u32,)>`.
```

`CxRaw` does not do any deduplication of its component types. If a component type is duplicated in the source or target `CxRaw` tuple type, the coercion is rejected for being ambiguous since the coercion logic will have no clue where to take references from and where to reborrow them into.

With these two rules in mind, let us now desugar the `generic_demo`:

```rust
// The user's function...
fn generic_demo<T, V>(cx: Cx<&mut T, &mut V>) {
    let a: &mut T = cx;
    let b: &mut V = cx;
    let _ = (a, b);
}

// Desugars to:
fn generic_demo<T, V>(cx: CxRaw<(&mut T, &mut V)>) {
    // We can assume that these do not alias because the tuple ensures that
    // they're different references.
    let a: &mut T = &mut cx.0.0;
    let b: &mut V = &mut cx.0.1;
    let _ = (a, b);
}

// So when we call it dangerously...
let mut value = 3u32;

// This coercion is rejected as ambiguous.
// `CxRaw<(&mut u32,)>` cannot coerce to `CxRaw<(&mut u32, &mut u32)>` because the target
// contains more than one component of type `&mut u32`.
generic_demo::<u32, u32>(CxRaw::new((&mut value,)));
```

Hence, it is safe to assume that distinct generic parameters are non-overlapping.

Now that we know generics are safe, we can now see how to use generics in a "refactor-safe" way:

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
    &'a mut <B as HasLogger>::Logger,
    &'a mut <B as HasDatabase>::Database,
>;

// Define the context for Child2
trait HasBundleForChildCx2: HasLogger + HasTracing {}

type Child2Cx<'a, B> = Cx<
    &'a mut <B as HasLogger>::Logger,
    &'a mut <B as HasTracing>::Tracing,
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

This technique of using type bundles to accept generic parameters in contexts works because all resolutions of `<B as HasLogger>::Logger` throughout the entire context type tree will resolve to the same generic type parameter when used in `parent`'s signature, meaning that we end up with a desugared signature for `parent` which looks like this:

```rust
// Our original function...
fn parent<B: ?Sized + HasBundleForParent>(cx: ParentCx<'_, B>) { ... }

// After inlining all our type aliases...
fn parent<B: ?Sized + HasBundleForParent>(cx: Cx<
    Cx<
        &'a mut B::Logger,
        &'a mut B::Database,
    >,
    Cx<
        &'a mut B::Logger,
        &'a mut B::Tracing,
    >
>) { ... }

// Desugars into:
fn parent<B: ?Sized + HasBundleForParent>(cx: CxRaw<(&mut B::Logger, &mut B::Database, &mut B::Tracing)>) {
    ...
}

// ...which was exactly what we wanted.
```

In addition to being safely parameterizable by their component types, `Cx` can also be parameterized by a generic context type implementing the `AnyCx` trait. This feature can be used like so:

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

By default, when `AnyCx` components are used to coerce into a destination context, ownership of the entire `AnyCx` instance is transferred. We can avoid this by specifying that the we actually intend to reborrow the type:

```rust
fn split_of_after_inspecting<L: AnyCx, R: AnyCx>(
    cx: Cx<L, R>,
    inspector: impl FnOnce(L::Reborrowed<'_>, R::Reborrowed<'_>),
) -> (L, R) {
    // Reborrows `L` and `R`
    inspector(cx, cx);

    // Moves `L` and `R`
    (cx, cx)
}
```

These methods involving `AnyCx` are sound for much the same reason that functions with generic component types are sound:

```rust
// This function:
fn split_off<L: AnyCx, R: AnyCx>(cx: Cx<L, R>) -> (L, R) { (cx, cx) }

// Desugars to:
fn split_off<L: AnyCx, R: AnyCx>(cx: CxRaw<(L, R)>) -> (L, R) { (cx.0.0, cx.0.1) }

// Preventing us from writing this dangerous code:
let cx_1: Cx<&mut u32, &i32> = ...;

// Cx<L, R> is substituted with CxRaw<(&mut u32, &i32, &mut u32)>, which once again causes an
// ambiguity.
let (cx_l, cx_r) = split_off::<Cx<&mut u32, &i32>, Cx<&mut u32>>(cx_1);
```

The aforementioned `split_off` and `map` functions are actually included in the standard library as inherent methods on `Cx`. Here are their definitions:

```rust
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

## Type Inference

There are many type inference situations that this proposal handles gracefully.

Because `Cx::new` is frequently coerced into a target type, it is not useful to infer its type from its usage destination.

```rust
fn consumer(cx: Cx<&mut System1, &System2>) { ... }

fn caller() {
    // Typical inference rules would infer the first parameter as having type
    // `(&mut System1, &System2)` but this is not useful because such an inference causes a type
    // error. The real type we're providing—`Cx<&mut System2, &mut System1>`—can easily be coerced
    // to the target type so that should be the real type of this expression.
    consumer(Cx::new((&mut system_2, &mut system_1)));
}
```

Coercion should attempt to infer the types of generic type parameters in a coercion target based off what is available.

```rust
fn generic_consumer_1<L: Logger, D: Database>(cx: Cx<&mut L, &mut D>) { ... }

fn generic_consumer_2<L: Logger, D>(cx: Cx<&mut L, &mut D>) { ... }

fn caller() {
    // Because `MyDatabase` is the only type in this context which we know to implement `Database`,
    // we infer `D` to be `MyDatabase`. Likewise, `MyLogger` is the only type in this context which
    // we know to implement `Logger` so `L` is inferred to be `MyLogger`. This inference then
    // allows this coercion to take place flawlessly.
    generic_consumer_1(Cx::new((&mut my_database, &mut my_logger, &mut my_tracer)));

    // Meanwhile, this coercion fails because, which we can successfully infer `L` to be `MyLogger`,
    // we can't unambiguously make a choice for `D` because both `MyTracer` and `MyDatabase` are
    // `Sized`.
    generic_consumer_2(Cx::new((&mut my_database, &mut my_logger, &mut my_tracer)));

    // This, however, does succeed because we can successfully infer `L` to be `MyLogger` and, in
    // doing so, we only leave one remaining type to be used as the database, allowing us to infer
    // `D` as being `MyDatabase`.
    generic_consumer_2(Cx::new((&mut my_database, &mut my_logger)));
}
```

To open up more opportunities for macro ~~magic~~ science, whenever several coercion targets are applicable during trait resolution, we always choose the applicable coercion target with the highest arity.

```rust
use core::{any::TypeId, marker:PhantomData};

struct AccessToken<T: ?Sized + 'static>(PhantomData<T>);

trait EnumerateAllTokens {
    fn enumerate(self) -> Vec<TypeId>;
}

impl<'a, A> EnumerateAllTokens for Cx<&'a mut AccessToken<A>>
where
    A: ?Sized + 'static,
{
    fn enumerate(self) -> Vec<TypeId> {
        vec![TypeId::of::<A>()]
    }
}

impl<'a, A, B> AcquireComponentsExt for Cx<&'a mut AccessToken<A>, &'a mut AccessToken<B>>
where
    A: ?Sized + 'static,
    B: ?Sized + 'static,
{
    fn enumerate(self) -> Vec<TypeId> {
        vec![TypeId::of::<A>(), TypeId::of::<B>()]
    }
}

// (continues for higher arities)

fn example() {
    let cx = Cx::new((&mut access_u32, &mut access_i32));

    assert_eq!(cx.enumerate(), vec![TypeId::of::<u32>(), TypeId::of::<i32>()]);
}
```

To make functions like `split_off` and `map` more useful, if all generic types besides one remaining type implementing `AnyCx` are resolved, the remaining `AnyCx` instance is resolved to be the remainder of the context.

```rust
fn split_off<L: AnyCx, R: AnyCx>(cx: Cx<L, R>) -> (L, R) { ... }

pub fn map_mut<R: AnyCx, T: ?Sized, V: ?Sized>(cx: Cx<&mut T, R>, f: impl FnOnce(&mut T) -> &mut V) -> Cx<L, V> { ... }

fn example(cx: Cx<&mut System1, &mut System2, &mut System3, &mut System4>) {
    let (left, right) = split_off::<Cx<&mut System1, &System2>, _>(cx);
    // `left`` has the type `Cx<&mut System1, &System2>`, leaving `right` with the type
    // `Cx<&System2, &mut System3, &mut System4>`.

    let mapped = map_mut(cx, |v: &mut System1| &mut v.inner);
    // `T` has the type `System1`. This leaves `R` with the type
    // `Cx<&mut System2, &mut System3, &mut System4>`, giving `mapped` the final type
    // `Cx<&mut System1Inner, &mut System2, &mut System3, &mut System4>`.
}
```

All the aforementioned coercions continue to work, even if the context is constructed in a nested manner. Note, however, that these inferences don't take lifetimes into account; in other words, they do not care whether a given type is already borrowed.

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

Implementation of this RFC is split up into several parts:

1. A `CxRaw` lang-item struct which provides the actual coercion mechanisms.
2. An `AnyCx` lang-item trait which provides a blanket `impl` over all `CxRaw` structures and exposes a mechanism for reborrowing these opaque contexts.
3. A `Cx` intrinsic type alias which implements the eager deduplication mentioned above.
4. Standard library helpers methods.

We begin with the semantics of `CxRaw`, `AnyCx`, and `ReborrowedFrom`. Here are their definitions in the `core` standard library:

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

==TODO: Ensure that the aforementioned guide-level inference rules are actually implemented here.==

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

# Prior art
[prior-art]: #prior-art

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

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

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
