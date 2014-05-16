- Start Date: 2014-05-16
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)


# Summary

Introducing an orthogonal handling of mutability and aliasing, which will hopefully make code clearer to read and increase the scope of what can be written without dropping to unsafe code.


# Motivation

The current system of handling mutability is confusing and requires unsafe code and compiler hacks. It is confusing because types like `Cell` can be mutated through `&T` references as they are inherently mutable (an invisible property) and requires compiler hacks in the way closures are modelled.

*Note: a concurrent RFC for [unboxed closures](https://github.com/rust-lang/rfcs/pull/77/) might solve the closure issue; it does not address the confusion issue though, nor the necessity to drop to unsafe code in order to mutate aliased objects.*


# Drawbacks

By refining the type system, this RFC introduces 4 reference types where there were only 2 previously. It also requires introducing a new Trait.

The benefits may not be worth the extra complexity overhead. It is hard to perform a costs/benefits analysis when everyone has a slightly different and vague idea of what the "system" being discussed is though therefore here comes a "reasonable" system, in all its ugly details.


# Detailed design

*Disclaimer: this proposal introduces several names, they are purposely overly verbose and they are stand-in to facilitate the discussion. If this proposal is ever accepted in whole or in parts, then we can have a grand bike-shed.*


## Interlude: Safe Mutation

The heart of the issue is achieving safe mutation. However, what exactly is *safe mutation* is unclear. Meeting an unclear goal is difficult, thus we will first define our goal.


### 1. A structural model for types

Let us define a model to talk about how types are laid out in memory.

For interoperability reasons, Rust lays its `struct`s out like C would. We will take the assumption that its `enum`s and arrays are laid out similarly.

The first interesting point is that the layout and the alignment of a type solely depend on its data members, and that two types with the same sequence of data members are laid out similarly and have the same alignment. Therefore, we are only interested in a type structure: `struct A { i: int, j: int }` and `struct Hello { world: int, really: int }` will thus be modelled as `(int, int)`.

*Note: for `enum` we introduce a dedicated `enum-tag` type and the `|` alternative marker such that `enum E { A(int), B(int, int) }` is modelled as `(enum-tag, (int) | (int, int))`.*

The second interesting point is that the C model guarantees that should two types `A` and `B` share a "common initial sequence" then this initial sequence is laid out similarly, irrespectively of what follows. The main consequence in our case is that should `A` be `(int, Char)` and `B` be `(int, Char, X...)` then using an instance of `B` through a `&A` is perfectly defined.

The third interesting point is that the C model guarantees that the layout of an array is independent of the number of its elements. That is, the stride between two consecutive elements is always the same, and therefore it is safe to access the first elements of a X elements array as if it were a N elements array, provided that N is less than or equal to X.


### 2. Layout compatibility

Based on this we can deduce rules for *layout compatibility*. We define two relationships.

Two types `A` and `B` are *layout-identical* if their structural models are identical.

An array `B` is *layout-substitutable* for an array `A` if:

 - `A` and `B` are composed of *layout-identical* elements
 - `B` is of greater or equal length than `A`

*Note: if the lengths of both arrays are strictly equal, they are actually layout-identical.*

A type `B` `(b0, b1, ...)` is *layout-substitutable* for a type `A` `(a0, a1, ..., aN)` if for `i` in `[0, N)`:

 - `ai` and `bi` are *layout-identical*
 - or `ai` and `bi` are both references, and `*bi` is *layout-substitutable* for `*ai`

and additionally `bN` is *layout-substitutable* for `aN`.

Regarding memory safety:

 - if `A` and `B` are *layout-identical*, then using an instance of `B` through a `&A` is safe, and vice-versa (the relation is symmetric)
 - if `B` is *layout-substitutable* for `A`, then using an instance of `B` through a `&A` is safe (the relation is asymmetric)


### 3. Defining safe mutation

Armed with the previous definition, we can define safe mutation.

> A mutation is *safe* if after its execution, for any remaining *usable* reference `ri` of static type `&Ai`, `ri` either points to:

> - an instance of type `Ai` itself
> - an instance of type `Bi` that is *layout-substitutable* for `Ai`

This is the upper-bound of memory safety, any system that respects this prescription is memory-safe. It might however be simpler to impose stricter restrictions on the mutations.

*Note: specifically, I suspect that the array substitutability would require tracking array lengths in the type system which requires, if I am not mistaken, dependent typing.*

*Note: because of moves, some references may still exist that cannot be used by the user; if a reference is unusable what happens to its pointee does not influence safety.*

*Note: a complete characterisation of safe mutation would also include thread-safety...*


## A type system for safe mutability

In order to achieve safe mutability, we simply need to ensure that any mutation is safe; the type system must thus allow us to guarantee this.


### 1. A focus on aliasing

The simplest way to guarantee that all the remaining usable references in the system can be safely used after the mutation is to guarantee that none of them point to changed content.

This proposal introduces the `exclusive` and `mutable` keyword:

 - a reference qualified by `exclusive` is guaranteed to have no alias
 - a reference qualified by `mutable` can be mutated

*Note: why `mutable` and not `mut`? Just so we can easily identify snippets written in the current system from those written in the proposed system.*

It is trivially provable that mutation through `&exclusive T` or `&exclusive mutable T` is safe, although we disable the former. The borrow-checker current implementation is sufficient to enforce the `exclusive` rule, and therefore we can easily convert today's code:

```rust
enum Variant { Integer(int), String(~str) }

fn replace(v: &mut Variant, new: &Variant) {
    *v = *new
}
```

to tomorrow's code:

```rust
enum Variant { Integer(int), String(~str) }

fn double(v: &exclusive mutable Variant, new: &Variant) {
    *v = *new
}
```

Essentially, this is simple bike-shedding for now. We add the `exclusive` and `mutable` keywords and declare that mutation may only be achieved through a `&exclusive mutable` reference. Pure bike-shedding.

Still, now aliasing and mutability are expressed through two distinct keywords and thus we can move on to:

 - having non-aliasing without mutability (which we will just note as an amusing side-effect, for now)
 - having *safe* mutability in the presence of aliasing

Let's go.


### 2. Introducing the `SafelyMutable` Trait

Types such as `int` or `Cell` can be safely mutated even when multiple aliases exist, yet the type system currently forbids it. This leads us to:

 - compile-time errors when attempting to borrow them into multiple `&mut` simultaneously
 - confusion when `let c = Cell::new(4);` can be mutated even though it is not apparently mutable

The goal of the `SafelyMutable` Trait is thus to mark such types that can be safely mutated in the presence of aliases.

Informally, a type is `SafelyMutable` if its mutation is safe. We restrict ourselves though to a small subset of safe mutation:

> A type is `SafelyMutable` if after its mutation any remaining usable reference `ri` of static type `&Ai` points to an instance of type `Ai`.

*Note: extending this definition to encompass any layout-identical or even, the full definition of safe mutation, can be done in a backward compatibility fashion, so let's start small.*

We propose that a type be declared `SafelyMutable` using the regular Trait syntax:

```rust
#![deriving(SafelyMutable)]
struct Cell<T> { ... }
```

or

```rust
struct Cell<T> { ... }

impl<T> SafelyMutable for Cell<T> {}
```

The compiler will enforce the necessary rules for a `SafelyMutable` type to really be safe. They are simple, overly conservative, and yet already allow `Cell` to be written without unsafe code:

 - a built-in integral or floating point type is `SafelyMutable`
 - a `struct` type declared `SafelyMutable` may only leak references to its `SafelyMutable` members
 - an `enum` type cannot be declared `SafelyMutable`

Of course, this trait would not be too useful without rules for its usage:

> An instance of a `SafelyMutable` type `T` may be mutated through a `&mutable T` reference.

This enables the following code:

```rust
fn main() {
    let mutable x = Cell::new(5);
    let mutable vec = vec!(&mutable x);
    *x = Cell::new(4);        // Mutable even though aliased!
}
```

> Furthermore, within a method of a `SafelyMutable` type `T`:
> - if the method is of `&self` kind, then references to non-`SafelyMutable` members are promoted to `&exclusive self`
> - if the method is of `&mutable self` kind, then references to non-`SafelyMutable` members are promoted to `&exclusive mutable self`

Only references to `SafelyMutable` members may have leaked to the exterior, thus it is safe to consider that the other members are effectively non-aliased. And even though the `SafelyMutable` members are aliased, they can still be modified through `&mutable self` references.

*Note: I avoid promoting the references to `SafelyMutable` members because I prefer avoiding lying to the compiler; lies usually end up exploited further down the road.*


### 3. A quick overview of `SafelyMutable` types

First of all, a C POD can always be declared `SafelyMutable`:

```rust
#![deriving(SafelyMutable)]
struct A { i: u32, f: f64 }


#![deriving(SafelyMutable)]
struct B { i: u32, a: A, array: [A..5] }
```

This is simple enough, and there is no special restriction for the methods of those types.

Since we use a trait system, we can use conditional implementation of `SafelyMutable`:

```rust
struct Cons<'b, T> {
    priv value: T,
    priv next: Option<&'b Cons>,
}

impl<'b, T: SafelyMutable> SafelyMutable for Cons<'b, T> {}
```

*Note: this is maybe too complicated, and we could require that a type either is or is not `SafelyMutable`.*

A `SafelyMutable` type can contain an `enum`, however it is forbidden to leak references to it:

```rust
#![deriving(SafelyMutable)]
struct Maybe<T> {
    value: Option<T>,       // error: only `SafelyMutable` members can be public,
                            //        and `Option<T>` is not one.
}

impl<T> Maybe<T> {
    pub fn get(&'a self) -> &'a Option<T> {
        self.value          // error: only references to `SafelyMutable` members can be leaked,
                            //        and `Option<T>` is not one.
    }

    pub fn apply(&'a self, closure: fn (&Option<T>)) {
        closure(self.value);    // OK, because the closure cannot "export" the reference
    }

    pub fn apply_export(&'a self, closure: fn (&'a Option<T>)) {
        closure(self.value);    // error: only references to `SafelyMutable` members can be leaked,
                                //        and `Option<T>` is not one.
    }
}
```

The same could probably be applied to arrays, except that dynamic arrays are not exactly part of Rust, indeed `Vec` is implemented with a `*mut T` backing array and capacity and actual length are tracked separately. Still, there could be some interesting cases:

```rust
impl<T> Vec<T> {
    /// Append an element to a vector *if* there is enough capacity already.
    ///
    /// Returns the element if it could not be appended.
    pub fn push_no_alloc(&mutable self, value: T) -> Option<T> {
        if self.len == self.cap {
            Some(value)
        } else {
            unsafe {
                let end = (self.ptr as *T).offset(self.len as int) as *mut T;
                move_val_init(&mut *end, value);
            }
            self.len += 1;
            None
        }
    }
}
```

*Note: since we are manipulating raw memory, it's impossible to avoid `unsafe`; however do note that we did not pass `&exclusive mutable self`, references to existing elements are safe as there is no re-allocation of the backing array.*


### 4. Playing with `SafelyMutable`

We can rewrite `Cell` without unsafe code:

```rust
#![deriving(SafelyMutable)]
pub struct Cell<T> {
    priv value: T,
}

impl<T:Copy> Cell<T> {
    /// Creates a new `Cell` containing the given value.
    pub fn new(v: T) -> Cell<T> {
        Cell {
            value: v,
        }
    }

    /// Returns a copy of the contained value.
    #[inline]
    pub fn get(&self) -> T {
        self.value
    }

    /// Sets the contained value.
    #[inline]
    pub fn set(&mutable self, v: T) {
        self.value = v;
    }
}
```

The key differences from the current implementation are:

 - `value` is no longer `Unsafe<T>`
 - no `unsafe` code in `get` and `set`
 - `set` now requires a `&mutable self` parameter

*Note: `noshare` should be unnecessary now, because the aliasing of this type is tracked properly.*

We can also give `RefCell` a shot, or part of it anyway:

```rust
#![deriving(SafelyMutable)]
pub struct RefCell<T> {
    priv value: T,
    priv borrow: BorrowFlag,
    priv nocopy: marker::NoCopy,
}

// Values [1, MAX-1] represent the number of `Ref` active
// (will not outgrow its range since `uint` is the size of the address space)
type BorrowFlag = uint;
static UNUSED: BorrowFlag = 0;
static EXCLUSIVE: BorrowFlag = -1;

impl<T> RefCell<T> {
    /// Consumes the `RefCell`, returning the wrapped value.
    pub fn unwrap(self) -> T {
        assert!(self.borrow == UNUSED);
        self.value
    }

    // No way around unsafe code here: there ARE potential aliases around after all!
    unsafe fn as_exclusive_mut<'a>(&'a mutable self) -> &'a exclusive mutable RefCell<T> {
        cast::transmute_exclusive_mut(self)
    }

    pub fn try_borrow<'a>(&'a mutable self) -> Option<Ref<'a, T>> {
        match self.borrow {
            EXCLUSIVE => None,
            _ => {
                self.borrow += 1;
                Some(Ref { parent: self })
            }
        }
    }

    pub fn try_borrow_mut<'a>(&'a mutable self) -> Option<RefMut<'a, T>> {
        match self.borrow {
            EXCLUSIVE => None,
            _ => {
                self.borrow += 1;
                Some(RefMut { parent: self })
            }
        }
    }

    pub fn try_borrow_exclusive<'a>(&'a mutable self) -> Option<RefExclusive<'a, T>> {
        match self.borrow {
            UNUSED => unsafe {
                let exclusive_mut_self = self.as_exclusive_mut();
                exclusive_mut_self.borrow = EXCLUSIVE;
                Some(RefExclusive { parent: exclusive_mut_self })
            },
            _ => None
        }
    }

    pub fn try_borrow_exclusive_mut<'a>(&'a mutable self) -> Option<RefExclusiveMut<'a, T>> {
        match self.borrow {
            UNUSED => unsafe {
                let exclusive_mut_self = self.as_exclusive_mut();
                exclusive_mut_self.borrow = EXCLUSIVE;
                Some(RefExclusiveMut { parent: exclusive_mut_self })
            },
            _ => None
        }
    }
}

pub struct Ref<'b, T> {
    priv parent: &'b mutable RefCell<T>,
}

impl<'b, T> Drop for Ref<'b, T> {
    fn drop(&exclusive mutable self) {
        assert!(self.parent.borrow != EXCLUSIVE && self.parent.borrow != UNUSED);
        self.parent.borrow -= 1;
    }
}

impl<'b, T> Deref<T> for Ref<'b, T> {
    #[inline]
    fn deref<'a>(&'a self) -> &'a T {
        &*self.parent.value
    }
}
```

*Note: requiring `&mutable self` is unfortunate. It is necessary unless we declare `borrow: mutable BorrowFlag` which would constitute a proposal of its own.*


# Opened doors

## Further down the rabbit hole

We can later on extend the number of `SafelyMutable` types or loose some of the restrictions in the allowed changes. For example:

 - we may relax the reference rules to allow substituting a *layout-identical* type for another

 - this would open the possibility that an `enum` with *layout-identical* variants be `SafelyMutable`, provided it meets a regular `struct` requirements... so `Either<A,B>` with `A` and `B` being *layout-identical* could possibly be `SafelyMutable` under some circumstances

 - and of course, we could push even further and make use of *layout-substitutable* types

This seems rather weird though, because Rust is a nominally typed language. Two different types have different methods and thus interpret the provided data differently. It is memory safe though.


## Mutable members and `&exclusive self`

In C++, one can use the `mutable` keyword to declare members that may be mutated even in `const` methods. This is often used to implement either lazy-computation or caching of values. Today, this code in Rust requires either:

 - unsafe code
 - exposing a `&mut self` method, even though the mutation is hidden to the user

If we allow such an extension to the language, then `&exclusive self` is necessary:

```rust
struct LazyComputation<T> {
    priv producer: mutable proc () -> T,
    priv value: mutable Option<T>,
}

impl<T> LazyComputation<T> {
    pub fn get(&'a exclusive self) -> &'a T {
        // since `self` is `&exclusive`, `self.value` is `&exclusive mutable`
        match self.value {
        None => self.value = producer();
        _ => ();
        }
        self.value.get()
    }
}
```


# Alternatives

There are several other RFCs that aim to tackle some issues, and Niko had a blog post about focusing on non-aliasing rather than mutability to enforce memory-safety.

For reference:

 - [RFC #77: Unboxed Closures](https://github.com/rust-lang/rfcs/pull/77/) aims at solving the closure issue without modifying the current type system
 - [Focusing on Ownership](http://smallcultfollowing.com/babysteps/blog/2014/05/13/focusing-on-ownership/) details how Rust would be better off focusing on aliasing than mutability
 - [RFC #58: Rename &mut to &only](https://github.com/rust-lang/rfcs/pull/58) also argues that aliasing is the better focus

There has, however, been an uproar of part of the community, the message was:

 - mutability is an important concept, even if memory-safety can be enforced without it
 - inherently mutable types (such as `Cell`) are confusing because even though `mut` is about inherited mutability it is viewed as a marker of the actual mutability of the object, causing a paper cut

On the other hand, this proposal has a complexity overhead. Still, I do believe that the resulting code is clearer as we stop conflating mutability and aliasing, and it also opens up interesting avenues (such as `Vec::push_no_alloc`).


# Unresolved questions

* The exact interactions with concurrent/parallel programming are still unclear. It should be possible for a type such as `Mutex` to be both `SafelyMutable` and `Share`, for example, however this flies way above my head...

* The exact names should be decided. I purposely refrain from giving my opinion on the matter.

