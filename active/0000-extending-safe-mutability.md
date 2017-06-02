- Start Date: 2014-05-16
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)


# Summary

Introducing an orthogonal handling of mutability and aliasing, which will hopefully make code clearer to read and increase the scope of what can be written without dropping to unsafe code.


# Motivation

The current system of handling mutability is confusing and requires unsafe code and compiler hacks. It is confusing because types like `Cell` can be mutated through `&T` references as they are internally mutable (an invisible property) and requires compiler hacks in the way closures are modelled.

*Note: a concurrent RFC for [unboxed closures](https://github.com/rust-lang/rfcs/pull/77/) might solve the closure issue; it does not address the confusion issue though, nor the necessity to drop to unsafe code in order to mutate aliased objects.*

The confusion is not helped by the compiler diagnostics:

```rust
use std::cell::Cell;

fn main() {
    let c = Cell::new(4);   // note: prior assignment occurs here
    c.set(3);
    c = Cell::new(5);       // error: re-assignment of immutable variable `c`
}
```

So, it's okay to mutate `c` via `set`, but not to assign to `c` because it is *immutable*...


# Drawbacks

By refining the type system, this RFC introduces 4 reference types where there were only 2 previously. It also requires introducing a new Trait.

The benefits may not be worth the extra complexity overhead. It is hard to perform a costs/benefits analysis when everyone has a slightly different and vague idea of what the "system" being discussed is though, therefore here comes a "reasonable" system, in all its ugly details.


# Detailed design

*Disclaimer: this proposal introduces several names, they are purposely overly verbose as they are place-holders to facilitate the discussion. If this proposal is ever accepted in whole or in parts, then we can have a grand bike-shed.*


## 1. Defining safe mutation

Rust is nominally typed, and therefore it seems dangerous to allow a `&A` to point to an instance of anything else than a `A`. Not only methods could be called that would be unexpected, but it would possibly confuse the Type-Based Alias Analysis in LLVM.

> A mutation is safe if after its execution any remaining usable reference `ri` of static type `&Ai` still points to an instance of type `Ai`.

*Note: this definition is valid only when no concurrent thread executes, aliases between threads have to properly synchronize their writes and reads on top of respecting this definition.*


## 2. A focus on aliasing

The simplest way to guarantee that all the remaining usable references in the system can be safely used after the mutation is to guarantee that none of them point to changed content.

This proposal introduces the `exclusive` and `mutable` keyword:

 - a reference qualified by `exclusive` is guaranteed to have no alias
 - a reference qualified by `mutable` allows mutation of the pointee

*Note: why `mutable` and not `mut`? Just so we can easily identify snippets written in the current system from those written in the proposed system.*

It is trivially provable that mutation through `&exclusive T` or `&exclusive mutable T` is safe, although we disallow the former. The borrow-check current implementation is sufficient to enforce the `exclusive` rule, and therefore we can easily convert today's code:

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

Essentially, this is simple bike-shedding for now. We add the `exclusive` and `mutable` keywords and declare that mutation may only be achieved through a `&exclusive mutable` reference. `exclusive` guarantees the safety and `mutable` guarantees the programmer intent.

Still, now aliasing and mutability are expressed through two distinct keywords and thus we can move on to:

 - having non-aliasing without mutability (which we will just note as an amusing side-effect, for now)
 - having *safe* mutability in the presence of aliasing

Let's go.


## 3. Introducing the `SafelyMutable` Trait

Types such as `int` or `Cell` can be safely mutated even when multiple aliases exist, yet the type system currently forbids it. This leads us to:

 - compile-time errors when attempting to borrow them into multiple `&mut` simultaneously
 - confusion when `let c = Cell::new(4);` can be mutated even though it is not apparently mutable (and attempts to assign to it fail because it is *immutable*)

The goal of the `SafelyMutable` Trait is thus to mark such types that can be safely mutated in the presence of aliases.

Informally, a type is `SafelyMutable` if its mutation is safe. We propose that a type be declared `SafelyMutable` using the regular Trait syntax:

```rust
#![deriving(SafelyMutable)]
struct Cell<T> { ... }
```

or

```rust
struct Cell<T> { ... }

impl<T> SafelyMutable for Cell<T> {}
```

The compiler will enforce the necessary rules for a `SafelyMutable` type to really be safe. Let us start simple:

 - a built-in integral or floating point type is `SafelyMutable`
 - a fixed-length array of `SafelyMutable` types, is itself `SafelyMutable`
 - a `struct` may be declared `SafelyMutable` if all its members are `SafelyMutable`
 - a `struct` may be declared `SafelyMutable` if it has an unsafe interior
 - an `enum` may be declared `SafelyMutable` if it is a C-like `enum` (no payload for any enumerator)

Of course, this trait would not be too useful without rules for its usage:

> An instance of a `SafelyMutable` type `T` may be assigned to even if potentially aliased.

This enables the following code:

```rust
#![deriving(SafelyMutable)]
struct Point {
    i: int,
    j: int,
}

fn main() {
    let mutable p = Point{i: 5, j: 10};
    let mutable vec = vec!(&mutable p);
    p = Point{i: 2, j: 5};        // Mutable even though aliased!
}
```


## 4. Further examples

`&mutable T` is not restricted to `SafelyMutable` types, which allows threading `mutable` down:

```rust
#![deriving(SafelyMutable)]
struct Point {
    pub i: int,
    pub j: int,
}

enum Line {
    Degenerate(Point),
    Regular(Point, Point),
}

fn switch_end(line: &mutable Line, point: Point) {
    match line {
    Degenerate(&mutable p) => *p = point;
    Regular(_, &mutable end) => *end = point;
    }
}
```

However, unless you derive from `SafelyMutable`, assignment is not allowed if aliases may exist:

```rust
fn other() {
    let mutable x = Line::new(Point{3,4}, Point{3,4});
    let vec = vec!(&mutable x);
    x = Line::new(Point{3,4}, Point{4,5}); // error: `x` is currently borrowed
}
```

And of course, you can mix and match `SafelyMutable` members, allowing:

```rust
struct Geom {
    pub origin: Point,
    pub vector: Line,
}

fn switch(left: &mutable Geom, right: &Geom) {
    left.origin = right.origin;     // OK: `origin` is `SafelyMutable`
    left.vector = right.vector;     // error: `Line` is not `SafelyMutable`
}
```


## 5. Extending safe mutability beyond PODs

I have yet to find a way for the compiler to check that a `SafelyMutable` type embeds non `SafelyMutable` members safely. This does not prevent us to use `unsafe` code to fix `Cell` though:

```rust
#![deriving(SafelyMutable)]
pub struct Cell<T> {
    value: Unsafe<T>,
    noshare: marker::NoShare,
}

impl<T:Copy> Cell<T> {
    pub fn new(v: T) -> Cell<T> { Cell { value: Unsafe::new(v), noshare: marker::NoShare, } }
    pub fn get(&self) -> T { unsafe { *self.value.get() } }
    pub fn set(&mutable self) { unsafe { *self.value.get() = value; } }
}
```

Benefits of the new version:

 - `Cell` can be assigned to whenever `Cell::set` can be called, and vice-versa, which matches our intuition
 - it is now possible to pass *read-only* references to a `Cell`

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
                let end = (self.ptr as *T).offset(self.len as int) as *mutable T;
                move_val_init(&mutable *end, value);
            }
            self.len += 1;
            None
        }
    }
}
```

*Note: since we are manipulating raw memory, it's impossible to avoid `unsafe`; however do note that we did not pass `&exclusive mutable self`: references to existing elements are safe as there is no re-allocation of the backing array.*


# Opened doors

## Conditional implementation of `SafelyMutable`

Since we use a trait system, we can use conditional implementation of `SafelyMutable`:

```rust
struct Cons<T> {
    pub t: T,
}

impl<T: SafelyMutable> SafelyMutable for Cons<T> {}
```

*Note: this is maybe too complicated, and we can start by requiring that a type either always is or is not `SafelyMutable` regardless of the parameters, as for `Cell`.*


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
 - internally mutable types (such as `Cell`) are confusing because even though `mut` is about inherited mutability it is viewed as a marker of the actual mutability of the object, causing a paper cut

On the other hand, this proposal has a complexity overhead. Still, I do believe that the resulting code is clearer as we stop conflating mutability and aliasing, and it also opens up interesting avenues (such as `Vec::push_no_alloc`).


# Unresolved questions

 * The exact interactions with concurrent/parallel programming are still unclear. It seems unsafe to attempt to share an instance of a `SafelyMutable` across multiple tasks, and thus I believe it more prudent to require non-aliasing, as it is today.

 * Should references be considered `SafelyMutable` ?

 * It is unclear how best to make `Cell` being `SafelyMutable` work; for now I used the rule that unsafe interior is OK, however it could be formulated by white-listing some types, etc...

 * The exact names should be decided. I purposely refrain from giving my opinion on the matter.
