- Feature Name: placement_traits
- Start Date: 2015-12-21
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC amends RFC #0809 to reduce the number of traits involved, take
allocators into account, and pin down the story on DST placement.

## Traits

There are now four traits:

1. `Placer` -- Placement in syntax:  `let owner = PLACE <- value`.
2. `Boxer` -- Box syntax: `let boxed: Box<_> = box value`.
3. `Place` -- An "out" pointer.
4. `Allocate` -- A trait that allows boxing and placing into `Allocator`s.

## Allocators

This RFC adds a new `Allocate` trait that can be implemented by any type that
takes an allocator. Implementing `Allocate` automatically implements `Boxer`
for boxing into allocators that implement `Default` and allows the type to be
allocated with a custom allocator using the `ALLOCATOR <- value` syntax.

## DSTs

`Boxer::make_place` and `Placer::make_place` are bounded by `Data: Sized` to
future proof against DST placement.

Furthermore, this RFC explicitly defines the guarantees of when/where
`Placer::make_place` will be called.

# Detailed design
[design]: #detailed-design

The new trait hierarchy is as follows:

```rust
/// Interface to implementations of  `PLACE <- EXPR`.
///
/// `PLACE <- EXPR` effectively desugars into:
///
/// ```rust,ignore
/// let p = PLACE;
/// let mut place = Placer::make_place(p);
/// let raw_place = Place::pointer(&mut place);
/// let value = EXPR;
/// unsafe {
///     std::ptr::write(raw_place, value);
///     Place::finalize(place)
/// }
/// ```
///
/// The type of `PLACE <- EXPR` is derived from the type of `PLACE` and the
/// context. If the type of `PLACE` is `P`, then the final type of the
/// expression is some owner such that `P` implements `Placer<typeof(EXPR), Owner>`.
///
/// Values for types implementing this trait usually are transient
/// intermediate values (e.g. the return value of `Vec::back`)
/// or `Copy`, since the `make_place` method takes `self` by value.
pub trait Placer<Data: ?Sized, Owner> {
    /// `Place` is the intermedate agent guarding the
    /// uninitialized state for `Data`.
    type Place: Place<Data=Data, Owner=Owner>;

    /// Creates a fresh place from `self`.
    fn make_place(self) -> Self::Place
        where Data: Sized;
}

/// Core trait for the `box EXPR` form.
///
/// `box EXPR` effectively desugars into:
///
/// ```rust,ignore
/// let mut place = Boxer::make_place();
/// let raw_place = Place::pointer(&mut place);
/// let value = EXPR;
/// unsafe {
///     ::std::ptr::write(raw_place, value);
///     Place::finalize(place)
/// }
/// ```
///
/// The type of `box EXPR` is supplied from its surrounding
/// context; in the above expansion, the result type `T` is used
/// to determine which implementation of `Boxer` to use, and that
/// `<T as Boxer>` in turn dictates which implementation of `Place` to use
/// (specifically, `<T as Boxer<typeof(EXPR)>>::Place`).
pub trait Boxer<Data: ?Sized>: Sized {
    /// The place that will negotiate the storage of the data.
    type Place: Place<Data=Data, Owner=Self>;

    /// Creates a globally fresh place.
    fn make_place() -> Self::Place
        where Data: Sized;
}

/// Both `PLACE <- EXPR` and `box EXPR` desugar into expressions
/// that allocate an intermediate "place" that holds uninitialized
/// state.  The desugaring evaluates EXPR, and writes the result at
/// the address returned by the `pointer` method of this trait.
///
/// A `Place` can be thought of as a special representation for a
/// hypothetical `&uninit` reference (which Rust cannot currently
/// express directly). That is, it represents a pointer to
/// uninitialized storage.
///
/// The client is responsible for two steps: First, initializing the
/// payload (it can access its address via `pointer`). Second,
/// converting the agent to an instance of the owning pointer, via the
/// appropriate `finalize` method.
///
/// If evaluating EXPR fails, then the destructor for the
/// implementation of Place is run to clean up any intermediate state
/// (e.g. deallocate box storage, pop a stack frame, etc).
pub unsafe trait Place {
    /// `Owner` is the type of the end value of both `PLACE <- EXPR` and
    /// `box EXPR`.
    ///
    /// Note that when `PLACE <- EXPR` is solely used for side-effecting an
    /// existing data-structure, e.g. `Vec::back`, then `Owner` need not carry
    /// any information at all (e.g. it can be the unit type `()` in that
    /// case).
    type Owner;

    /// `Data` is the type of the value to be emplaced.
    type Data: ?Sized;

    /// Returns the address where the input value will be written.
    /// Note that the data at this address is generally uninitialized,
    /// and thus one should use `ptr::write` for initializing it.
    fn pointer(&mut self) -> *mut Self::Data;

    /// Converts self into the final value, shifting deallocation/cleanup
    /// responsibilities (if any remain), over to the returned instance of
    /// `Owner` and forgetting self.
    unsafe fn finalize(self) -> Self::Owner;
}

/// All types implemeinting this trait can be constructed using using the `box`
/// keyword (iff the associated `Allocator` implements `Default`) or by placing
/// (`<-`) a value into an instance of the associated `Allocator`.
///
/// ```norun
/// let boxed1: Box<i32, SomeAllocator> = box 1; // where SomeAllocator: Default
/// let boxed2: Box<i32, SomeAllocator> = SomeAllocator::new() <- 1;
/// ```
pub trait Allocate<Data: ?Sized>: Sized {
    /// The place that will negotiate the storage of the data.
    type Place: Place<Data=Data, Owner=Self>;
    /// The allocator for this type.
    type Allocator: Allocator;

    /// Create a `Place` for `Self` with the given `Allocator`.
    fn allocate(with: Self::Allocator) -> Self::Place
        where Data: Sized;
}

```

Additionally, we define the following blanket impls to allow anything
implementing `Allocate` to be allocated in any `Allocator`. When specialization
lands, these can be marked default.

```rust
// Allows `let owner: T = ALLOCATOR <- thing;`
impl<T, D, A> Placer<D, A> for T
    where T: Allocator,
          A: Allocate<D, Allocator=T>
{
    type Place = A::Place;

    fn make_place(self) -> Self::Place
        where Self: Sized
    {
        A::allocate(self)
    }
}

// Allows `let owner: T = box thing;`
impl<A, D> Boxer<D> for A
    where A: Allocate<D>,
          A::Allocator: Default
{
    type Place = A::Place;

    fn make_place() -> Self::Place
        where Self: Sized
    {
        A::allocate(Default::default())
    }
}
```

Finally, to support DST placement, this RFC explicitly loosens the placement
protocol guarantees. Specifically, the place in placement in/new is not
guaranteed to be allocated before the evaluation of the expression on the right
hand side. DST placement needs this to be able to compute the size of the DST
before allocating the place. This means that, in the following cases, whether or
not the `Box` is ever allocated is explicitly undefined:

```rust
let _: Box<_> = box panic!();
let _: Box<_> = HEAP <- panic!();
```

For completeness, I've included the following DST placement traits to
demonstrate that the current placement traits are compatible with DST placement.
Note: A concrete design for returning DSTs is well outside the scope of this
RFC.

```rust
trait DstPlacer<Data: ?Sized, Placer>: Placer<Data, Output> {
    fn make_place_dynamic(self, layout: Layout) -> Self::Place;
}

trait DstBoxer<Data: ?Sized, A>: Placer<Data, A> where A: Allocator {
    fn make_place_dynamic(allocator: A, layout: Layout) -> Self::Place;
}

impl<T, D, B> DstPlacer<D, B> for T
    where T: Allocator,
          B: DstBoxer<D, T>
{
    type Place = B::Place;

    fn make_place_dynamic(self, layout: Layout) -> Self::Place {
        B::make_place_dynamic(self, layout)
    }
}

// Assuming specialization.
default impl<D, O> Placer<D, O> for T where T: DstPlacer<D, O> {
    fn make_place(self, layout: Layout) -> Self::Place
        where Self: Sized,
    {
        self.make_place_dynamic(Layout::new::<D>())
    }
}

default impl<D, A> Boxer<D, A> for T
    where T: DstBoxer<D, A>
          A: Allocator
{
    fn make_place(allocator: A) -> Self::Place
        where Self: Sized,
    {
        self.make_place_dynamic(Layout::new::<D>())
    }
}
```

## Choices

### Placer::make_place

`Placer::make_place` takes self by value. Taking self by reference as discussed
in #1286 would either require an option dance, HKT to properly handle lifetimes
(`type Place<'a> = ...`), or shenanigans. Furthermore, taking self by value
ensures that non-copy placers are used only once. This allows panic-free
allocation using `try!(place) <- thing`.

### Parameterize Placer with Owner

`Owner` is now a type parameter in `Placer`. This allows placement in syntax to
be used with allocators. That is:

```rust
let boxed_thing: Box<_> = HEAP <- thing1;
let rced_thing: Rc<_> = HEAP <- thing2;
```

If `PLACER <- thing` can have precisely one type as in the original RFC, it
wouldn't be possible to produce both `Rc`s and `Box`s (see alternatives for an
alternative solution).

# Drawbacks
[drawbacks]: #drawbacks

## Placer::make_place self by value

Taking self by-value means that `vec <- value` won't work. However, `vec.back()
<- value` still works just fine so I'm not convinced this is a problem.

The other drawback is that `arena <- value` won't work either. One solution is
to autoref the placer if necessary but,

1. I'm not entirely convinced this use case is common enough to be worth it.
2. This is a backwards compatible change that can be made at any time.

## Place taking Owner as a type parameter

This could interfere with type inference but shouldn't be an issue. In most
cases, `Placer` will only be defined for one `Owner` per type.

# Alternatives
[alternatives]: #alternatives

## Get rid of Placer

Instead of having a `Placer`, we could allow placing directly into `Place`s.
Unfortunately, to allow expressions that may or may not be placed into (e.g.
`map.entry(k) <- v`), the `Place` would have to allocate in `Place::pointer`
instead of on construction to avoid allocating unnecessarily. This, in turn,
means that the `Place` would have to manually keep track of whether or not it
has allocated. Using a `Placer` allows us to store this in the type system
itself which:

1. Is less error prone.
2. Is probably more optimizable.


## Placer with an associated Owner

Instead of making the `Placer` trait take `Owner` as a type argument, we could
add the following default method to the `Allocator` trait:

```rust
trait Allocator {
    /* ... */
    fn emplace<B: Boxer>(self) -> BoxPlacer<Self, B> {
        BoxPlacer {
            allocator: self,
            _marker: PhantomData,
        }
    }
}

pub struct BoxPlacer<A: Allocator, B> {
    allocator: A,
    _marker: PhantomData(fn() -> B)
}

impl<A, B, D> Placer<D> for BoxPlacer<A, B>
    where B: Boxer<D>,
          A: Allocator,
{
    fn make_place(self) -> B::Place
        where Self: Sized,
    {
        <B as Boxer<D>>::make_place(self)
    }
}
```

We could then use `let boxed = HEAP.emplace::<Type<_>>() <- value;` to select
the output type. However, IMO, this is much less ergonomic.

## The Allocate trait

Instead of having an `Allocate` trait, we could have all `Boxer`s take
`Allocator`s. However, as @nagisa has pointed out, not all `Boxer`s will need
an `Allocator` (some may, e.g., use object pools).

# Unresolved questions
[unresolved]: #unresolved-questions

None.
