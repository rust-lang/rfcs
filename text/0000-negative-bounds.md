- Start Date: 2015 Jan 15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Allow the compiler to verify that multiple blanket impls are not in conflict using negative bounds.

This RFC corresponds to issue #442.

1. [Motivation](#motivation)

2. [Design summary](#design-summary)

3. [Detailed design](#detailed-design)

    * [Design principles](#design-principles)
    * [Syntax of negative bounds](#syntax-of-negative-bounds)
    * [Evaluating disjoint collections](#evaluating-disjoint-collections)
    * [Interaction with other features](#interaction-with-other-features)
    * [No implicit specialization](#no-implicit-specialization)
    * [Miscellaneous](#miscellaneous)

4. [Drawbacks](#drawbacks)

5. [Alternatives](#alternatives)

6. [Unresolved questions](#unresolved-questions)

7. [Related discussions](#related-discussions)

# Motivation

## Clearing conflicts

Currently, implementing a trait is impossible if a blanket impl is present. For instance it is impossible to implement `FnMut` ([#18835][18835]):

```rust
struct Counted<F: Fn<A, R>, A, R> {
    counts: uint,
    function: F,
}
impl<F: Fn<A, R>, A, R> FnMut<A, R> for Counted<F, A, R> {
    extern "rust-call" fn call_mut(&mut self, input: A) -> R {
        self.counts += 1;
        self.function.call(input)
    }
}
```

because there is an existing blanket impl in libcore ([#19032][19032]):

```rust
impl<F: ?Sized, A, R> FnMut<A, R> for F where F : Fn<A, R>
```

While it is possible that we do implement `Fn` to `Counted` with some third-party F, A or R, it is never the intention of the programmer. It should be possible to tell the compiler that `Counted` will *never* implement `Fn`, and thus clearing the conflict.

As an extension of above, one may like to have multiple blanket impls to the same trait ([RFC issue #442][rfc442]),

```rust
trait Average {
    fn average(self, other: Self) -> Self;
}
impl<T: Int> Average for T {
    fn average(self, other: Self) -> Self {
        if self >= other {
            other + (self - other) / 2
        } else {
            self + (other - self) / 2
        }
    }
}
impl<T: Float> Average for T {
    fn average(self, other: Self) -> Self {
        self * 0.5 + other * 0.5
    }
}
```

Again, it is possible that a type implements both `Int` and `Float`, but we are sure this is nonsense. The programmer of the trait should be able to tell the compiler to not to accept types that satisfies both conditions.

## Specialization

Sometimes we would like to provide a solution with better performance for a particular type, even if an impl for a satisfying bound already exists ([#18404][18404]):

```rust
use std::borrow::ToOwned;

trait ToString {
    fn to_string(&self) -> String;
}
impl<T: std::fmt::String> ToString for T {
    fn to_string(&self) -> String { format!("{}", *self) }
}
impl<'a> ToString for &'a str {
    fn to_string(&self) -> String { (*self).to_owned() }
}
```

Or, to automatically adapt method based on properties of the subtrait ([#12517][12517], [#17884][17884]):

```rust
impl<T: Ord> PartialOrd for T {
    fn partial_ord(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl<T: PartialOrd, U: PartialOrd> PartialOrd for (T, U) {
    fn partial_ord(&self, other: &Self) -> Option<Ordering> { ... }
}
```

It is pretty clear that the impls in both examples are in conflict, but to a human reader it is also clear that one impl is definitely more specialized than the other. There should be a way to tell the compiler to resolve these conflict and choose the more specialized version.

# Design summary

## Negative bounds

We allow bounds to be negated by a `!`. If `T: !Copy`, this means `T` definitely will not implement `Copy`.

```rust
fn definitely_drop<T: !Copy>(_: T) {}

definitely_drop(Box::new(123u8));
// ok.

definitely_drop(123u8);
// error: the trait `core::kinds::Copy` is implemented for type `u8`
```

## Clearing conflicts

With negative bounds, we may combine some bounds together and cause no type be able to satisfy all of them. One trivial example is `T: Copy + !Copy`. Here we call such a *disjoint collection of bounds*. The compiler can easily check if some bound combinations are disjoint.

Therefore, the following implement should pass the coherence test, because the bounds from both impl together are disjoint, and therefore conflict-free:

```rust
impl<T: !Show> IsShow for T { ... }
impl<T: Show> IsShow for T { ... }
```

The compiler should still signal an error if it cannot rule out any possibility of conflict. Programmers should explicitly insert additional negative bounds. This also allows programmers to choose the conflict resolution method, e.g.

```rust
impl<T: Int> Average1 for T { ... }
impl<T: Float> Average1 for T { ... }
/// error: conflicting implementations for trait `Average1`

impl<T: Int + !Float> Average2 for T { ... }
impl<T: Float> Average2 for T { ... }
/// ok. if T: Int + Float, the Float variant would be chosen.

impl<T: Int + !Float> Average3 for T { ... }
impl<T: Float + !Int> Average3 for T { ... }
/// ok. if T: Int + Float, then T does not implement Average3 (raise error on use).

impl<T: Int + !Float> Average4 for T { ... }
impl<T: Float + !Int> Average4 for T { ... }
impl<T: Int + Float> Average4 for T { ... }
/// ok. special treatment for T: Int + Float.
```

# Detailed design

## Design principles

1. **Allow multiple blanket impls while preserving coherence**.

    * It should be possible to write multiple blanket impls, with explicit indicator that these impls are not in conflict, that the compiler be able to check. (Resolving issue #442.)
    * Any algorithm used must not produce *false negative* on the final outcome. That means, if two impl do in fact have conflict, the algorithm must not allow them to compile.
    * However, false positive is tolerated. That means, even if two impl are in fact conflict-free, we may still be rejecting it.

2. **Backward compatible**.

    * Since this RFC is slated for 1.x, any change should not affect type checking or inference of existing code.
    * It must be compatible with Rust 1.0. In particular, interaction with the following features are considered:
    * Default and negative impls (RFC #127),
    * Associated items (RFC #195),
    * Higher-ranked trait bounds (RFC #387).

4. **Restrictions**.

    * Here are some self-imposed restriction to try to keep the change to implementation simple.
    * This RFC must not introduce unions to where clauses. Internally parameter bounds should keep the simple linear `A + B + !C + D + …` form.
    * External crates must not be able to affect validity of the current crate. Compiler must not need to "see through" impls of traits appearing in bounds, e.g. it should not need to know that `Int` and `Float` are implemented by totally different types.

## Syntax of negative bounds

Rust 1.0 supports 4 kinds of bounds:

* **Trait bound** — `where T: Int, U: Eq<T>`
* **Equality bound** — `where T = u8`
* **Outlives bound** — `where T: 'a, 'b: 'a`
* **Projection bound** — `where <T as Iterator>::Item = u8` (commonly known as associated type binding)

On top of these bounds, we also allow trait bounds to be higher-ranked:

* `where T : for<'a, 'b: 'a> Trait<'a, 'b, X>`

In this RFC, we allow all these bounds to be negated. The syntax for each kind of negative bounds is like:

* **Negative trait bound** — `where T: !Int, U: !Eq<T>`
* **Inequality bound** — `where T != u8`
* **Negative outlives bound** — `where T: !'a, 'b: !'a`
* **Negative projection bound** —  `where <T as Iterator>::Item != u8`

> Note: `'b: !'a` is not the same as `'a: 'b`. In `'b: !'a`, the two lifetimes could merely be representing two irrelevant regions.

There are three possible ways combine negation with higher-ranked modifier:

* **Negated higher-ranked bound** — `where T: !for<'a> F<'a>`
* **Higher-ranked negated bound** — `where T: for<'a> !F<'a>`
* **Negated higher-ranked negated bound** — `where T: !for<'a> !F<'a>`

In this RFC we will investigate all three cases, but as HRTB is already quite rare, we may choose to only support one of these forms.

There is also a shorthand notation for projection bound:

* `where T: Iterator<Item=u8>`, equivalent to
* `where T: Iterator, T::Item = u8`

The shorthand notation *cannot* be negated in the outermost position, because it would need a union in the desugared form.

```rust
// the short-hand notation of output constraint cannot be negated, since its
// expansion cannot be written in the form `A + B + !C + !D + ...`.
impl<T> Trait for T where T: !Iterator<Item=u8> { ... }
// error: expected `,` or `>`, found `=`

// The above can be simulated using two impls if desired:
impl<T> Trait for T where T: !Iterator { ... }
impl<T> Trait for T where T: Iterator, T::Item != u8 { ... }
```

Negation cannot be combined with relaxation. The following should not parse:

```rust
// negative relaxed bound reduces to an impossible bound, so it is pointless.
struct NegativeRelaxedBound<T: !?Sized>;
// error: expected identifier, lifetime or `for`, found `?`

// relaxed negative bound and relaxed positive bound are just equivalent.
struct RelaxedNegativeBound<T: ?!Sized>;
// error: expected identifier, found `!`
```

## Evaluating disjoint collections

### Trait bounds

We could consider trait bounds to be sets, and concrete types satisfying the bound be elements in the set. Borrowing terminology from set theory, we call collections of bounds which have no common types be a *disjoint collection of bounds*. Bounds which are not disjoint are *overlapping*.

In Rust, all traits are open for extension. Any traits with no superbounds are able to be implemented by the same type, even if the original developer may not think they should be used together. Therefore, all traits with no superbounds should be considered overlapping.

The only way to create disjoint collection of trait bounds is by negative bounds. It is guaranteed that `!B` and `B` share no common types. In a collection `T: A + B + C + …`, as long as two of them are disjoint, the whole collection is also disjoint.

If a trait has superbounds, then as long as a type does not satisfy one of its superbounds, it cannot implement that trait either. Therefore, we get a fundamental rule for disjoint collections:

* [①] — `T: A` and `T: !B` are disjoint, if and only if `trait A: B` (A is stricter than or equals to B).

| Collection                  | Disjoint? | Note                                                                   |
|-----------------------------|:---------:|------------------------------------------------------------------------|
| `Copy + !Copy`              | ✓         | Disjoint because `Copy: Copy`                                          |
| `Eq + !PartialEq`           | ✓         | Disjoint because `Eq: PartialEq`                                       |
| `PartialEq + !Eq`           | ✗         | Note where the `!` is applied                                          |
| `Mul<u8> + !Mul<i8>`        | ✗         | There is no relation between traits with different type parameters     |
| `IntoCow<'a> + IntoCow<'b>` | ✗         | There is no relation between traits with different lifetime parameters |

Intuitively, if we have two impls that have the same shape, but the type parameters are bound by a disjoint collection, the type checker should still allow them since there is no possibility of conflict:

```rust
// These should work, since `Copy` and `Clone + !Copy` are disjoint.
trait MyClone { fn my_clone(&self) -> Self; }
impl<T: Copy> MyClone for T { fn my_clone(&self) -> Self { *self } }
impl<T: Clone + !Copy> MyClone for T { fn my_clone(&self) -> Self { self.clone() } }
```

The compiler could reject a bound if it is disjoint.

```rust
fn foo<T: Copy + !Copy>(_: T) { ... }
// error: impossible bound for `T`. `core::kinds::Copy` is both required and disallowed.
```

### Equality and projection bounds

Equality bounds could be considered a trait that only one type can satisfy. Therefore, the rule for trait bounds still apply. Additionally, if two equality bounds refer to concrete types, they are trivally disjoint.

* [②] — `T = a` and `T = b` are disjoint, if `a` and `b` refer to distinct types.

Although project bounds may match multiple types, the above still applies, since there can only be one associated type for each implementation of a single trait (note bug [#20400](https://github.com/rust-lang/rust/issues/20400)).

* [③] — `<T as Trait>::Item = a` and `<T as Trait>::Item = b` are disjoint, if `a` and `b` refer to distinct types.

In an (in)equality bound which refer to two parameters e.g. `T == U` or `T != U`, it should be considered as a bound for both T and U. The following should work:

```rust
trait TypeEqual1<T, U> { type Result; }
impl<T, U> TypeEqual1<T, U> where T = U { type Result = TrueType; }
impl<T, U> TypeEqual1<T, U> where T != U { type Result = FalseType; }

trait TypeEqual2<T, U> { type Result; }
impl<T> TypeEqual2<T, T> { type Result = TrueType; }
impl<T, U> TypeEqual2<T, U> where T != U { type Result = FalseType; }
```

| Collection                   | Disjoint? | Note                                                                   |
|------------------------------|:---------:|------------------------------------------------------------------------|
| `T = u8, T: !Eq`             | ✓         | Disjoint because `u8: Eq`                                              |
| `T = u8, T = i8`             | ✓         | Two concrete equality bounds are trivially disjoint                    |
| `U, T = u8, T = U`           | ✗         | The unbound type parameter `U` could be anything, including u8.        |
| `U, T = u8, T != U`          | ✗         | The unbound type parameter `U` could be anything, including *not* u8.  |
| `U != u8, T = u8, T = U`     | ✓         |                                                                        |
| `U = u8, T = u8, T != U`     | ✓         |                                                                        |
| `U != u8, T != u8, T = U`    | ✗         | Inequality is not transitive.                                          |
| `T = U, U != T`              | ✓         |                                                                        |
| `V, T=Vec<V>, U=[V; 2], T=U` | ✓         | Although T and U are not concrete, they clearly have different shapes. |

### Outlives bounds

Lifetime bounds have the same treatment as trait bounds. The rule is same as [①]:

* [④] — `T: 'a` and `T: !'b` are disjoint, if and only if `'a: 'b` (`'a` outlives `'b`).

| Collection             | Disjoint? | Note                                                                          |
|------------------------|:---------:|-------------------------------------------------------------------------------|
| `'a: 'b, 'a: !'b`      | ✓         |                                                                               |
| `'a: 'b + !'b`         | ✓         | Equivalent to above.                                                          |
| `'a: 'b, 'b: 'a`       | ✗         | `'a == 'b` is still allowed by this bound.                                    |
| `'a: 'b + !'c, 'b: 'c` | ✓         | Outlives bounds are transitive. Consider `'c` a superbound of `'b`.           |
| `'a, T: 'static + !'a` | ✓         | `'static` outlives all lifetimes, so `'static + !'anything` must be disjoint. |

### Higher-ranked trait bounds

Higher-ranked trait bounds is actually expressing an infinite intersection of multiple traits. The following two bounds are equivalent:

* `where T: for<U: B> Trait<U>`
* `where T: Trait<U1> + Trait<U2> + Trait<U3> + Trait<U4> + …` (for all `Ui: B`)

Therefore, if we directly apply [①]:

* [⑤] — `T: for<U: B> A<U>` and `T: !C` are disjoint, if and only if there exists a `U: B`, that `A<U>: C`.
* [⑥'] — `T: !for<U: B> A<U>` and `T: C` are disjoint, if and only if **for all** `U: B`, that `C: A<U>`.

Rust uses Skolemization when solving HRTBs. In short, it will create an arbitrary `Ui: B`, and replace the whole `for<U: B> A<U>` with `A<Ui>` during type checking.

Currently HRTBs are possible only with traits. The superbounds of a trait is fixed in definition. Therefore, third-party crates cannot affect the validity of `A<U>: C`. If the Skolemized `A<Ui>` does indeed satisfy `C`, we can claim "there exists" condition is valid. That means, rule [⑤] reduced to rule [①] after Skolemization, and is still valid.

Rule [⑥'] requires a "for all" condition. If `C` is `T: for<U: D> A<U>`, then we require `(for<U: D> A<U>): (for<U: B> A<U>)`. This is possible only if D is larger than B, i.e. `B: D`. Thus, we revise rule [⑥'] as:

* [⑥] — `T: !for<U: B> A<U>` and `T: for<U: D> A<U>` are disjoint, if `B: D`.

Rule [⑥] is optional. Suppose we ignore rule [⑥], after Skolemization we get `T: !A<Ui> + A<Uj>`. Since the two traits are irrelevant, we claim they are overlapping. We tolerate false positive, so it is okay.

| Collection                                        | Disjoint? | Note                                                                          |
|---------------------------------------------------|:---------:|-------------------------------------------------------------------------------|
| `T: for<'a> A<'a> + !A<'static>`                  | ✓         | `A<'static>: A<'static>` with rule [⑤], or `'static: 'a` with rule [⑥]        |
| `T: !for<'a> A<'a> + A<'static>`                  | ✗         | Boundless lifetime is not stricter than 'static, with rule [⑥]                |
| `T: for<'a> A<'a> + !for<'b> A<'b>`               | ✓         |                                                                               |
| `T: for<'a> FnMut(&'a i8) + !for<'a> Fn(&'a i8)`  | ✗         | Currently Fn and FnMut are irrelevant traits                                  |
| `T: for<U> !PartialEq<U> + Eq`                    | ✓         | Disjoint because `!PartialEq: !Eq`, which is transposition of `Eq: PartialEq` |
| `T: !for<U> PartialEq<U> + Eq`                    | ✗         | `PartialEq<Ui>` and `Eq: PartialEq<Self>` are irrelevant                      |

## Interaction with other features

### Default and negative impls (#127)

When checking for conflicts, both positive and negative *impl* should be treated the same, as is today:

```rust
// These should be fine:
trait Foo {}
impl Foo for .. {}
impl<T: PartialEq + !Eq> !Foo for T {}
impl<T: Eq> Foo for T {}

// But these should be error:
trait Bar {}
impl Bar for .. {}
impl<T: PartialEq> !Bar for T {} // <-- note lack of '!Eq'.
impl<T: Eq> Bar for T {}
// error: conflicting implementations for trait `Bar`
```

Negatives bounds cannot replace default impls, since the latter will also check component fields.

```rust
trait NotSendGood {}
impl<T: !Send> NotSendGood for T {}

trait NotSendBad {}
impl NotSendBad for .. {}
impl<T: Send> !NotSendBad for T {}

struct Data {
    ptr: *const (),
    len: usize,
}

// Here, `*const()` does not implement Send, and `usize` implements Send.
// Therefore, `Data` itself also does not implement Send.
//
// Obviously this means `Data` implements NotSendGood.
//
// However, since `usize` has opt-out NotSendBad, it means not all components
// implement NotSendBad.
// Thus, `Data` does not implement NotSendBad.
```

Negative impl *could* be relaxed to allow implementing a non-empty trait. The impl must still be empty. This is one way to indicate that a particular (generic) type can never implement some trait:

```rust
struct Counted<F: Fn<A, R>, A, R> { ... }

// Using negative impl to prevent Counted to implement Fn by anybody.
impl<F: Fn<A, R>, A, R> !Fn<A, R> for Counted<F, A, R> {}
impl<F: Fn<A, R>, A, R> FnMut<A, R> for Counted<F, A, R> { ... }

// The following also work, but won't prevent an external crate implementing Fn
// for Counted for other use.
impl<F: Fn<A, R>, A, R> FnMut<A, R> for Counted<F, A, R>
    where Counted<F, A, R>: !Fn<A, R>
{ ... }
```

### Associated items (#195)

RFC #195 has [reserved a section][assoc_spec] about how associated types interacted with specialization. This case also applies to this RFC. We would not like to introduce any change to alter the current behavior. That means, an associated type of a non-concrete type must still be considered an arbitrary opaque type. If associated lifetime is implemented, we would also consider that is an opaque lifetime without a concrete type.

### Higher-ranked trait bounds (#387)

This has already been investigated above. The existing architecture (Skolemization) should be enough to make sure everything works correctly.

### Higher-kinded traits

Since there is no RFC for higher-kinded types, we would not know the exact design for higher-kinded types. If we consider the aspect of HKTs as atomic semantic elements, as long as higher-kinded trait bounds are still using the same semantics, a negative higher-kinded trait should still be work the same:

```rust
// Imaginary syntax. Don't complain. :)

// Assume we have these traits, with Option and Vec implementing Monad.
trait Applicative for type<type> { ... }
trait Monad for type<type> : Applicative { ... }

// The following should work, the first 3 bounds form disjoint collections.
trait MyFunctor for type<type> { ... }

impl<T<_>> MyFunctor for T where T: Applicative + !Monad { ... }
impl<T<_>> MyFunctor for T where T: Monad, T != Option { ... }
impl MyFunctor for Option { ... }

// error. this impl conflict with T: Monad.
impl MyFunctor for Vec { ... }
```

If we consider the aspect of HKTs as type constructors, they should behave similar to associated types — the output of the HKT should be considered opaque. This is out of scope of this RFC though.

## No implicit specialization

This RFC will not introduce implicit specialization. The following should still be rejected:

```rust
trait MyEq {}
impl<T: PartialEq> MyEq for T {}
impl<T: Eq> MyEq for T {}
// error: conflicting implementations for trait `MyEq`
```

Explicit specialization is possible by excluding the specialized instances from the generic case.

Inter-crate specialization is not possible with this method.

Implicit specialization may be added back in the future, see the [Alternatives](#alternatives) section.

## Miscellaneous

* We should allow equality type bound syntax be also written as `where T == a`, to preserve the symmetry with `where T != a`.

* The Clone trait may be changed to automatically implemented to those implementing Copy, as described in issue [#17884][17884]. This would require a `!Copy` bound to most custom implementations though. Similar for PartialEq/Eq and PartialOrd/Ord.

    ```rust
    impl<T: Copy> Clone for T {
        fn clone(&self) -> T { *self }
    }
    impl<T, U> Clone for (T, U) where T: Clone, U: Clone, (T, U): !Copy {
        fn clone(&self) -> (T, U) { (self.0.clone(), self.1.clone()) }
    }
    impl<T: Clone + !Copy> Clone for Option<T> {
        fn clone(&self) -> Option<T> {
            match *self {
                None => None,
                Some(ref e) => Some(e.clone()),
            }
        }
    }
    ```

# Drawbacks

* While negative bounds is a simple concept, it is also unintuitive. It is hard to understand `'a: !'b` without drawing figures for instance.

* Negative bounds are purely a device to make the compiler happy about conflicting impl. It does not add any capability to the type it is bound on. To a human it may look like line noise.

* Unlike specialization, it seems no other languages support negative bounds.

* Extra syntax needs to be introduced.

* Resolving an obligation becomes more complex.

# Alternatives

## Syntax

### Negative bounds

Instead `!` we may denote a negative bound with `-`. This is more consistent with `+` for combining bounds, and it's natural to introduce the shorthand `A + -B == A - B`:

```rust
impl<T: -Int> Trait for T {}
impl<T: Int - Float> Trait for T {}
```

The advantage of `!` over `-` is that negative impls are already using `!`, and `!` seems more intuitive given that most old discussions raising this idea uses `!`.

```rust
impl<T> !Send for *const T {}
impl<T: !Send> NotSync for T {}
```

### Inequality bounds

Instead of `where T != u8`, we may write `where T: !u8` for an inequality bound. The advantage is we could add inequality to multiple types much more concisely:

* `where T: !u8 + !u16 + !u32 + !u64`, vs.
* `where T != u8, T != u16, T != u32, T != u64`

The disadvantage is apparent ambiguity in:

```rust
trait U {}
impl<T, U> Foo for T where T: !U {} // refering to type U, not trait U.
```

## Alternatives to negative bounds

### Status quo

Don't do anything. Don't fix anything mentioned here.

### Specialization

Instead of adding negative bounds, we could instead allow specialization, a.k.a. overlapping instances. The semantic is based on [GHC 7.10's overlapping instance pragmas][ghc_9242]:

```rust
// error. requires explicit annotation to enable specialization.
impl<T: PartialEq> Trait1 for T {}
impl<T: Eq> Trait1 for T {}

// ok.
#[specialize(base)]   impl<T: PartialEq> Trait2 for T {}
#[specialize(middle)] impl<T: Eq> Trait2 for T {}
#[specialize(final)]  impl Trait2 for u8 {}
```

If an `impl` is labeled with `#[specialize]`, the coherence checker will also accept an impl if it is strictly stricter or looser than the bound of another impl. The looser impl must be either labeled `#[specialize(base)]` or `#[specialize(middle)]`. The stricter impl must be either labeled `#[specialize(final)]` or `#[specialize(middle)]`.

* If X is disjoint from Y, pass.
* If `X <: Y` and `X != Y`, pass if X is `#[specialize(final)]` and Y is `#[specialize(base)]`.
* If `Y <: X` and `X != Y`, pass if X is `#[specialize(base)]` and Y is `#[specialize(final)]`.
* Else fail.
* `#[specialize(middle)]` is the same as `#[specialize(base)] #[specialize(final)]`.

Intercrate specialization is allowed. However, care must be taken to prevent incoherence. See [GHC's example on overlapping instance][ghc_overlap] for one possible test case.

Specialization will never allow two blanket impls of unrelated trait bounds (this is the main reason why negative bounds is chosen for this RFC instead of specialization):

```rust
// error.
#[specialize(middle)] impl<T: Int> Trait3 for T {}
#[specialize(middle)] impl<T: Float> Trait3 for T {}
```

As described in [RFC #195][assoc_spec], we may also require a trait attribute in order to opt-in for specialization.

```rust
#[specialize] trait Trait2 {}
```

Annotated specialization and negative bounds are compatible with each other. This part may be submitted as a separate RFC without affecting this one.

### Introduce dynamic trait checking built-in

Specialization of arbitrary complexity can be simulated at run-time using TypeId and unsafe code:

```rust
use std::intrinsics::TypeId;
use std::mem::transmute_copy;
use std::borrow::ToOwned;

trait FastToString {
    fn fast_to_string(&self) -> String;
}
impl<T: std::fmt::String + 'static> FastToString for T {
    fn fast_to_string(&self) -> String {
        if TypeId::of::<T>() == TypeId::of::<&'static str>() {
            let self_str: &'static str = unsafe { transmute_copy(self) };
            self_str.to_owned()
        } else {
            self.to_string()
        }
    }
}
```

Since value of typeid is known at compile-time, LLVM is able to optimize all the `if` checks away. If there is a `typeid_implements!(typeid, Trait)` built-in macro, many negative bound examples above could also be written using runtime checking.

```rust
fn do_something<T: PartialEq + 'static>(x: &T, y: &T) {
    if typeid_implements!(TypeId::of::<T>(), Eq) {
        // unsafe cast and do Eq stuff
    } else {
        // do PartialEq stuff.
    }
}
```

However, this method is ugly, requires the extremely unsafe `transmute_copy`, and only works with `'static` types (`&'a str` cannot work, for instance). I would strongly recommend against using this approach as standard.

### Remove coherence check

Check for ambiguity only at instantiation site. This is similar to how C++ templates work. Suggested by the OP in [issue #442][rfc442], when ambiguity arises, we could use UFCS to distinguish which instance to use:

```rust
impl<T: Deref> Foo for T { fn foo(&self) { ... } }
impl<T: Ord> Foo for T { fn foo(&self) { ... } }

fn main() {
    (&123.0).foo();     // ok choose the Deref impl.
    (456).foo();        // ok choose the Ord impl.
    (vec![1]).foo();    // error, ambiguity.
    <vec![1] as Deref>::foo();  // explicitly want the Deref impl
    <vec![1] as Ord>::foo();    // explicitly want the Ord impl
}
```

The problem is the resolution syntax is only good for blanket impls. It is hard to write down the exact UFCS syntax when the `impl for` is not just a simple `T`, but `Box<Rc<RefCell<Vec<HashMap<...>>>>>`. Also, losing early check for impl coherence seems too much sacrifice compared to multiple blanket impls.

# Unresolved questions

None so far?

# Related discussions

Negative bounds and specialization have been mentioned many times before, here are some old discussions:

* [#10414][10414]: "Unfortunate type bound on Range"
* [#10879][10879]: "RFC: Generalize Freeze, etc. into custom-defined does-not-contain traits" (addressed by negative impl already)
* [#12517][12517]: "use correct naming for the comparison traits and add trait inheritance"
* [#17884][17884]: "avoid having two distinct systems for copying"
* [#18404][18404]: "`"".to_string()` is slower than `String::new()`"
* [#18835][18835]: "Unable to manually implement FnOnce"
* [#19032][19032]: "Coherence and blanket impls interact in a suboptimal fashion with generic impls" (also contains pros and cons of negative bounds)
* [RFC #290][rfc290]: "Function specialization"
* [Issue #442][rfc442]: "Support multiple blanket impls with differing bounds for traits"
* [RFC #536][rfc536]: "Mark `std::mem::drop` as unstable until negative bounds are implemented"
* http://discuss.rust-lang.org/t/type-exclusion-in-trait-implementation/976
* http://discuss.rust-lang.org/t/trait-implementation-priority/1006


[assoc_spec]: https://github.com/rust-lang/rfcs/blob/master/text/0195-associated-items.md#future-proofing-specialization-of-impls
[ghc_overlap]: https://downloads.haskell.org/~ghc/7.8.4/docs/html/users_guide/type-class-extensions.html#instance-overlap
[ghc_9242]: https://ghc.haskell.org/trac/ghc/ticket/9242
[10414]: https://github.com/rust-lang/rust/issues/10414
[10879]: https://github.com/rust-lang/rust/issues/10879
[17884]: https://github.com/rust-lang/rust/issues/17884
[18404]: https://github.com/rust-lang/rust/issues/18404
[18835]: https://github.com/rust-lang/rust/issues/18835
[19032]: https://github.com/rust-lang/rust/issues/19032
[rfc290]: https://github.com/rust-lang/rfcs/issues/290
[rfc442]: https://github.com/rust-lang/rfcs/issues/442
[rfc536]: https://github.com/rust-lang/rfcs/issues/536
[12517]: https://github.com/rust-lang/rust/issues/12517#issuecomment-61997227

