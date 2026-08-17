- Feature Name: `own_ref`
- Start Date: 2026-07-22
- RFC PR: [rust-lang/rfcs#4000](https://github.com/rust-lang/rfcs/pull/4000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

Introduce owning references `&own` into the language, which allows passing ownership of the pointee without moving its value.
Owning references can be moved out of (including partial moves), and drop their pointee when dropped.


## Motivation
[motivation]: #motivation

Owning references (`&own`, sometimes called `&move`) come up every so often in discussions around Rust.
Their primary purpose is to allow passing owned values behind an indirection, but agnostic of the backing allocation.
There have been multiple attempts at bringing them to the Rust language in the past, each of which has been **postponed**.

<details><summary><h4 style="display:inline">A brief history of <code>&own/&move</code></h4></summary>

This is a short list of Rust related resources that inspired us in certain aspects of this RFC, provided in chronological order.
    
#### [RFC #965](https://github.com/rust-lang/rfcs/pull/965) Add `&own T`

Already back in 2015, this RFC proposed owning references.
It shares some of the motivations with this RFC:
- Provide an allocation-agnostic owned reference type.
- Pass ownership of values while avoiding (large) copies.
- Pass ownership of unsized types (slices and trait objects).
    
It was **postponed** after some comments by @nikomatsakis:
- It is *too soon* for a new pointer type, especially for teaching Rust.
- Many use cases can be discussed using library types and custom allocators.

#### [Issue #998](https://github.com/rust-lang/rfcs/issues/998) Owned references to contents in an earlier stack frame (`&own`, `&move`, etc)
    
Niko also opened this issue afterwards, presumably to keep an eye on this topic.
It occasionally received comments, and is still open today.
    
#### [RFC #1617](https://github.com/rust-lang/rfcs/pull/1617) Add an owning "borrowed" pointer type `&move`

Just one year later, another short RFC proposed owning references with minor differences:
- It calls the reference `&move` to reuse the existing keyword.
- Add a simple (but incomplete) `DerefMove` trait.

Once again, it was **postponed** until more design bandwidth is available.
Primary concerns were that "owned borrowing" would be confusing, as well as some syntactic discussions.
Niko then mentions he wanted to re-open the RFC to allow further discussions, but by then another RFC already appeared:


#### [RFC #1646](https://github.com/rust-lang/rfcs/pull/1646) &move, DerefMove, DerefPure and box patterns

Less than a month after the previous RFC was closed, this new one was opened.
It touches once again on some of the same topics, but also proposes a "pure" version of `DerefMove` to allow pattern matching.
    
Once more, this RFC was **postponed** a year later, since it was a "good idea at a bad time".
Niko once again left an [elaborate comment](https://github.com/rust-lang/rfcs/pull/1646#issuecomment-279123028):
- Concerns about a steeper learning curve.
- Simpler solutions exist, specifically for unsized types.
  - I believe this is talking about `unsized_rvalues`, but the provided link is dead.
- More elaborate problems exist, such as out-pointers, and should possibly be considered at the same time.

#### [URLO Thread](https://users.rust-lang.org/t/aside-some-ramblings-about-move-references) Aside – some ramblings about `&move` references

In mid 2021, Yandros provided a sketch of owning references, based on their own experience implementing the `stackbox` crate.
This topic also already mentions how `unsized_fn_params` are effectively implicit owning references.

#### [IRLO Thread](https://internals.rust-lang.org/t/a-sketch-for-move-semantics) A sketch for `&move` semantics

In mid 2023, another thread appeared discussing owning references.
This time, the interaction of `Pin` and `&own` was also discussed.
In another [lengthy comment](https://internals.rust-lang.org/t/a-sketch-for-move-semantics/18632/20), Yandros once again dives into the details of how a library solution is not sufficient, how this could replace `unsized_fn_params`, and how pinning is too much to handle.

#### [Zulip Thread](https://rust-lang.zulipchat.com/#narrow/channel/549962-t-lang.2Fmove-trait/topic/Leak.2C.20Forget.2C.20and.20.26own) Leak Forget and `&own`

This thread discussed how `Forget` would interact with `&own`. While this is not directly related to the contents of this RFC, it is the reason why I (the RFC author) decided to tackle this topic:
There seems to be some sort of community consensus how an owning reference would work, even though we neither have it in the language nor a formal design for it.

While further discussion (including [pre-RFC discussion](https://rust-lang.zulipchat.com/#narrow/channel/213817-t-lang/topic/.60.26own.60.20appetite.3F.20.28and.20Pre-RFC.20I.20guess.29/with/615156849)) revealed that not all details are easy, we believe that the learning curve is not that high, but that owning references may even be very intuitive.

---

</details>

A number of things have changed since the last proposals:
- After over 10 years of Rust, it is no longer "too soon" to add another fundamental reference type.
- The `unsized_rvalues` RFC has been merged, and demonstrated the challenges with handling unsized values implictly.
- We have prior art in external crates, which show that pure library implementations are not sufficient.
- Non-movable types have become important in Rust (`async`-like self-referential types)

As such, this RFC introduces owning references `&own` to allow passing ownership of both **unsized and immovable types**, as well as an **explicit optimization**, in a **allocation agnostic** way.
At the same time, we try to choose the **most intuitive** behavior for owning references where possible.


### Passing ownership of large objects

When writing a function that takes ownership of some `LargeStruct`, idiomatic Rust suggests passing by value:

```rust
fn consume_by_value(x: LargeStruct) { }
```

However, this may cause our `LargeStruct` to be copied to/from the stack, depending on compiler optimizations.
If we want to guarantee that such a copy does not happen, we are forced to use indirection, and instead pass a pointer to the struct itself.
The idiomatic safe way to do this would be via `Box`:

```rust
fn consume_by_box(x: Box<LargeStruct>) { }
```

This avoids unnecessary copies when passing through functions, but requires putting `LargeStruct` into the heap up front.
This may be undesirable in some cases, e.g. when the value is already part of a different heap allocation (e.g., a `Vec`), or even impossible in others (e.g., if `alloc` is unavailable).

Using an owning reference, we can pass ownership of our value, while ensuring that the compiler will not perform additional copies:

```rust
fn consume_by_ref(x: &own LargeStruct) { }
```


<details><summary><h4 style="display:inline">Example: Statically fused Iterators</h4></summary>

One possible application of this is the `Iterator` trait, which is currently defined like this:

```rust
pub trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
```

This definition leaves it unclear what should happen, if an iterator is run past its end.
Iterators should return `None` to indicate their end.
If they are advanced after that, they may return arbitrary values or panic.

An alternative design could use `&own` references to statically guarantee that an iterator may not be polled past its end, without passing the (possibly large) iterator by value:

```rust
pub trait Iterator {
    type Item;

    fn next(&own self) -> Option<(&own Self, Self::Item)>;
}
```

> This RFC doesn't propose that we make this change, but uses it as an interesting example application for `&own`.

</details>

### Consuming unsized types

In some cases we don't just want indirection, we *need* it.
In current safe Rust, this primarily comes up with unsized types such as trait objects and slices.
As an example, the following code fails to compile:

```rust
// error[E0277]: the size for values of type `(dyn FnOnce() + 'static)` cannot be known at compilation time
fn consume_by_value(x: dyn FnOnce()) {
    x()
}
```

Again, the solution is to use a `Box`, costing a heap allocation.
Alternatively, we could use an owning reference, which provides similar capabilities to a `Box`, but without the heap allocation:

```rust
fn consume_by_ref(x: &own dyn FnOnce()) {
    x()
}
```

<details><summary><h4 style="display:inline">Example: Owned slices</h4></summary>

Probably the most commonly used unsized types are slices `[T]`.
Since they cannot be passed by value, they are always passed with indirection.
We use `&[T]` for read-only slices, `&mut [T]` for mutable slices.
Slices are well built into the language and support some convenient features, such as slice pattern matching.
Currently, there exists no way to use slice pattern matching for owned values.
Boxed slices `Box<[T]>` are often used to fill this gap, but they cannot be used in slice patterns.

```rust
fn foo(boxed: Box<[String]>) {
    match *boxed {
        // error[E0277]: the size for values of type `[String]` cannot be known at compilation time
        [first, rest @ ..] => { },
        _ => { }
    }
}
```

This code fails, because we cannot provide a useful type for `rest`.
While `first` could in theory move out the first string by value (it doesn't today),
`rest` would be an unsized local.
We cannot split the box into two boxes, as it would be unclear which value is responsible for freeing the backing allocation.

Owning slices `&own [T]` don't suffer from this problem, since they are only responsible for freeing their values, not the backing allocation.
It is not problematic to split an owning slice in two, so most known slice operations can be applied to owned values as well.


```rust
fn foo(owned: &own [String]) {
    match owned {
        [first, rest @ ..] => { },
        _ => { }
    }
}
```

> Even without pattern matching, a `<[T]>::split_at_owned(&own self, usize) -> (&own [T], &own [T])` supports this behavior in a way that neither `Box` nor `Vec` can.

</details>

<details><summary><h4 style="display:inline">Example: Owned <code>self</code> in trait methods</h4></summary>

A very useful application of this is to make traits which take `self` arguments `dyn`-compatible.
As a real-world example, let us look at a slightly simplified version of the `FnOnce` trait:

```rs
pub trait FnOnce<Args: Tuple> {
    /// The returned type after the call operator is used.
    type Output;

    /// Performs the call operation.
    fn call_once(self, args: Args) -> Self::Output;
}

// Call a boxed and type erased closure
fn call_func(func: Box<dyn FnOnce()>) {
    // error[E0161]: cannot move a value of type `dyn FnOnce()`
    (*func).call_once(());
}
```

This code fails, because we cannot pass an unsized value to a function.
However, we could change the trait method to take an `&own self` instead, which is dyn-compatible:

```rs
pub trait FnOnce<Args: Tuple> {
    /// The returned type after the call operator is used.
    type Output;

    /// Performs the call operation.
    fn call_once(&own self, args: Args) -> Self::Output;
}

// Call a boxed and type erased closure
fn call_func(func: Box<dyn FnOnce()>) {
    // With auto-deref, this could be func.call_once(())
    (&own *func).call_once(());
}
```

> For more discussion on `FnOnce`, see [below](#Change-FnOnce-to-use-ampown-and-remove-unsized_fn_params).

</details>

### Consuming immovable types

Rust does not *currently* have immovable types.
The closest we have is `Pin`, which additionally enforces the drop guarantee, which we cannot provide with `&own` (see [drawbacks](#Pinning)).
However, in a possible future where Rust splits immovability from the drop guarantee, `&own` would play a central role in passing ownership of immobile types.

> The "Immobile types and guaranteed destructors" project goal[^move-trait] is (among other options) considering a combination of `T: !Move + !Forget` to replace `Pin<T>`.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

An *owning reference* type is written `&own T` or `&'a own T` for any type `T`.
Owning references behave like mutable references, except that they own the value behind the reference, which means that:
- It is possible to move out of an `&own T`.
- Dropping an `&own T` drops the inner `T`.

Owning references should be used when we want to transfer ownership and either do not want to or can not move the value.
For example, you might want to write a function that prints a list of values.
For efficiency, we will use a homogeneous list of trait objects.
Note that this function accepts a list of trait objects, but does not require putting them on the heap!
Additionally, we can consume both the slice and its elements, which is not possible with other reference types.

```rust
fn print_all(producers: &own [&own dyn FnOnce() -> String]) {
    for producer in producers {
        println!("Got value: {}", producer());
    }
}

let hello = String::from("Hello");
print_all(&own [
    &own || hello,
    &own || "World".to_string(),
    &own || format!("{}", 42),
]);
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Owned reference type

The reference type syntax is extended to allow the contextual keyword `own` instead of `mut`:

```grammar,types
ReferenceType -> `&` Lifetime? (`mut` | own`)? TypeNoBounds
```

In this position, `own` may also appear as an identifier at the beginning of `TypeNoBounds`.
To resolve this conflict, `own` is always parsed as the keyword in this position.
If the developer intends to reference a type which begins with the `own` identifier, they must wrap it in parentheses.

### Owned borrow expression

The syntax of borrow expressions is extended in the same way.
Similarly to the owned reference type expression, if `Expression` begins with an the identifier `own`, it must be wrapped in parentheses.

```grammar,expressions
BorrowExpression ->
      (`&`|`&&`) Expression
    | (`&`|`&&`) `mut` Expression
    | (`&`|`&&`) `own` Expression
    | (`&`|`&&`) `raw` `const` Expression
    | (`&`|`&&`) `raw` `mut` Expression
```

When using the borrow expression `&own Expression` on a place expression with type `T`, the expression produces an owning reference of type `&own T` in the same fashion as existing reference types.
The place must be an *owned place* and is considered exclusively borrowed for the lifetime of the created owning references, as well as considered "moved out" once the lifetime expires.

The following expressions can be owned place expression contexts:

* Variables which are not currently borrowed.
* Temporary values.
  * This must perform lifetime extension.
* Fields: this evaluates the sub-expression in an owned place expression context.
* Dereference of a `*mut T` pointer.
* Dereference of a place, or field of a place, with type `Box<T>`.
* Dereference of a place, or field of a place, with type `&own T`.

When using the borrow expression on a value expression, the behavior is analogous to that of other borrow expressions. 

### Properties

The type `&'a own T` behaves similar like other references, except it owns the value of the pointee:

- It is **covariant** in both `'a` and `T`.
- It is an **exclusive** borrow.
- It may be reborrowed as a shared or mutable reference.
- It must be aligned, non-null, point to a valid `T`, and be "dereferencable".
- When dereferenced, it produces an *owned* place (instead of a mutable one), which may be (partially) moved out of (like `Box`).
- When dropped, it also drops its pointee (but does not free the allocation).

## Drawbacks
[drawbacks]: #drawbacks

### Pinning

This simple approach to owning references cannot support pinning.
A `Pin<&'a own T>` is unsound, since forgetting the reference violates the drop guarantee (unless `'a: 'static`).
This may be confusing to users, since they often only consider the immovability guarantee of `Pin`.
Unfortunately, the [alternative](#Alternative-Add-remote-drop-flags-to-support-pinning) is more complex, and we believe that it is not worth it.

### Some easy APIs are actually hard

As Rust users, we are used to APIs of the form `fn into_foo(&own Self) -> &own Foo`.
However, these APIs are often difficult or impossible to write.
For example, we cannot write a function `Box<T>::into_inner(&own self) -> &own T` without leaking the allocation.
In general, we cannot write such an API if `Self` requires dropping (and the dropping involves the returned value).
This is the same problem as providing a `DerefOwn` trait ([see below](#Introduce-DerefOwn)).

### Teaching

Many Rust resources teach that Rust has two kinds of references: Shared and mutable.
With owning references, we would add a third one to the mix.
One challenge here is, that owning references are not borrows like the other two:
After a borrow expires, you can keep using the original value.
But with owning references, once the borrow expires, the original value is gone.

On the one hand, we believe that we do not need to bother Rust beginners with owning references.
[The Rust Book](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html), addresses the current primary alternative `Box` only in chapter 15, and owning references would be considered similar advanced usage.
However, we would still need to adjust teaching to mention a third reference type, to be explained later.

### Documentation

Besides implementing this, documentation changes would be a big churn.
While we believe that the behavior of `&own` is mostly intuitive,
many places in Rust documentation would need to be adjusted to account for a third reference type, which does not share the behavior of the others (e.g., does something when dropped).

Additionally, many places in Rust use `Box<T>` as the canonical "owned pointer", which in some cases would need adjustment.
For example, the [`std` docs](https://doc.rust-lang.org/std/#containers-and-collections) call `Box<[T]>` an "owned slice", which would be confusing in the presence of `&own [T]`.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The proposed design of `&own` references is kept as simple as possible.
It tries to be an intuitive addition to the Rust programming language, where most existing programmers would understand what it does without having to read the reference.

This design intentionally omits some additional complexities mentioned in drawbacks and alternatives, in favor or a straightforward design.

### Alternative: Do nothing

This is the current situation, and Rust users found a number of ways to work around the lack of owning references.
Primarily, users will use `Box` if they can and it isn't performance critical.
If boxing is not available (too expensive, or no heap present), alternatives include using a `&mut Option<T>` to pass runtime-checked ownership of `T`, or use a crate like [stackbox](https://docs.rs/stackbox/latest/stackbox/struct.StackBox.html).

However, as stated by [the `stackbox` maintaner themselves](https://internals.rust-lang.org/t/a-sketch-for-move-semantics/18632/19), there are some ergonomic challenges with a library implementation:

- Creating a stack box is cumbersome.
- Especially lifetime extension is missing.
- `&own self` receivers are not (currently) possible.
- It is non-straightforward to write functions are "allocation-agnostic", i.e., work with both `Box` and `StackBox`.

### Alternative: Syntax options

While many prior discussions use the `&own` notation, other options are available:

- `&move` has been used in the past, indicating the "movement of ownership".
    - We believe that this syntax isn't as clear in the implied semantics.
    - This could re-use the existing `move` keyword
        - However, there is ambiguity with closures: `&move || { }`
- `&ref(own)` does not require a contextual keyword
    - This allows for more reference types to be added in the future
    - Doubles the amount of typing and adds visual clutter
- `Own<'_, T>` could use normal type syntax, avoiding additional parsing complexity.
    - This would visually more closely resemble `Box` rather than other reference types
    - This would likely need a macro to perform (re-)borrowing

### Alternative: Wait for custom references

As part of the "field projections" project goal[^field-projections], we aim to support custom reference types in the compiler.
Supporting an `Own<'_, T>` reference type with the same semantics as described in this RFC should become possible via a library type.

This is likely the best alternative, since it in principle can provide everything a specialized owning reference can.
However, we believe that `&own` is fundamental enough to deserve custom syntax, even with custom references.
Additionally, most of the work that goes into owning references would need to happen even if we waited for fully custom references.

### Alternative: Model as `Box<T, NoOp<'a>>`

Semantically, `&own` is identical to `Box<T, NoOp<'a>>`, where `NoOp<'a>` is a zero-sized `Allocator`, which cannot perform allocations and does nothing on deallocations.
A type alias could be used to name this type `Own<'a, T>`.
With this alternative, a macro would be used to create an owning reference to a place, such as `own!(place)` instead of `&own place`.

On the upside, this would allow functions accepting an allocator-generic `Box` would to also accept an `Own`.
However, there are a number of downsides to this approach:

- The `Box` API is too general.
  For example, it would mean that `Own<'a, T>: Clone` if `T: Clone`. However, calling this method would have to panic with the `Noop` allocator.
- There might be subtle differences in the types that are currently unknown. 
  For example, we might want different aliasing rules for the types.
- `Box` is (currently) not available in `no_std` (unless `alloc` is enabled), so APIs compatible with `no_std` cannot take `Box` as an argument.
  We might be able to change this, but it seems non-trivial.

Additionally, it might just be *too weird*.
By its own docs:
> `Box<T>`, casually referred to as a ‘box’, provides the simplest form of heap allocation in Rust.
> Boxes provide ownership for this allocation, and drop their contents when they go out of scope.

A `Box<T, Noop<'a>>` is neither a heap allocation, nor does it provide ownership for its allocation.

### Alternative: Add remote drop flags to support pinning

Unfortunately, `Pin<&own T>` is unsound with regards to the drop guarantee.
This is because forgetting `Pin<&own T>` avoids running the drop implementation of `T`, but we have no control over the backing memory or the reference.

If we'd want to fix this design to allow for pinning, we would have to change `&own` to track and possibly modify the drop flags of its allocation.
In this scenario, `Pin<&own T>` would behave similarly to an `Pin<&mut Option<T>>`, which is already expressible today.

While this could be an interesting design approach, we believe that remote drop flags would make the language design much more complicated.
Instead, we proposed this much simpler design, even if that forbids pinning.

> See the [`own-ref` crate](https://docs.rs/own-ref/0.1.0-alpha/own_ref/pin/index.html), which allows adding drop-flags via a generic on the reference, and how that helps with pinning.

> See [future possibilities](#Interactions-with-Move-and-Forget) for how the `Move` and `Forget` traits could avoid the need for pinning and use the current design of `&own`.

### Alternative: Explore design space of additional reference types

This RFC proposes a single new reference type, `&own`.
There may be additional reference types we may want to add in the future, such as `&pin`, `&uninit`, `&out`, or `&init`.
It may be advantageous to increase the scope of this RFC to include other references, to avoid short-sighted decisions in this design space.

Besides the complexity in the language, we believe that major point of contention is the syntax space.
Contextual keywords, as suggested here with `&own` work well, but might become annoying when there are too many.

We could consider changing the syntax to avoid the need for contextual keywords, e.g. `&ref(own) T`, although this significantly increases syntactic load on the reader.

## Prior art
[prior-art]: #prior-art

### The `stackbox` and `own_ref` crates

These crates provide library types that emulate `&own`.
The `stackbox` crate frames itself as a stack-allocated box.
The creation of a `StackBox` is somewhat cumbersome.

The `own_ref` is essentially an updated version of `stackbox`, which additionally allows pinning via a optional built-in drop flag.
It improves reference creation, but still lacks ergonomic features.

In particular, no library can currently emulate moving out of field.

### The `bumpalo` crate and its `Box` type

The [`bumpalo` crate](https://docs.rs/bumpalo/latest/bumpalo/) provides bump allocation via the `Bump` arena.
The `Bump::alloc<T>(&self, val: T) -> &mut T` method allocates a `T` in the arena, and returns a mutable reference to it.
Since the arena does not keep track of the types of all of its allocations, it cannot drop `T`, and the mutable reference does not allow dropping the value either (without unsafe code).

As a solution, it provides its own `Box` type, which drops its pointee when dropped.
This `Box` is effectively the same as `&own`, except for missing ergonomics.

> Note that `bumpalo:box:Box` provides a pinning API, which is an known to be unsound.

## Unresolved questions
[unresolved-questions]: #unresolved-questions

### Does `&'a own T` need to be covariant in `'a`?

While it is undisputed that it is *possible* to enable this property, there are concerns about its usefulness.
If it is not necessary to be covariant, we could alternatively be invariant in `'a`.
This could allow us, for example, to use `&own` as an initialization prove for certain in-place-init proposals.

### Should is be allowed to borrow through pointers?

The unsafe operation `&own *ptr` would allow effectively casting the pointer to an owning reference.
This is allowed with other references, so it seems reasonable to allow this here too.
However, this operation additionally causes a deferred drop of the pointee as a side-effect.

As an alternative, we could disallow this behavior (for now), and instead introduce some more explicitly named function `unsafe fn assume_owned<'a, T>'(ptr: *mut T) -> &own T`.

### How does `&own place` work if `place: Copy`?

As per the design, taking an owning reference to a place means that the place may be mutated and will be considered moved out of.
This results in some possibly unexpected interactions.
Currently it is not possible in rust to move out of a place which is `Copy`, since we will always just copy the value instead.
So this behavior is consistent, but might be surprising (and new in the Rust language):

```rs
let x = 5u32;
{
    let owned = &own x;
    *owned += 2;
}
assert_eq!(x, 5); // Error: Use of moved value
```

If we consider the place not moved out, the referee may be modified after the borrow ends (the assert would fail at runtime), which would indicate that we require a `mut` binding.
```rs
let mut x = 5u32;
{
    let owned = &own x;
    *owned += 2;
}
assert_eq!(x, 5); // Assertion failed; 7 != 5
```

The final option is to use the "trivial copy" property of `Copy` types, and simply restore the value after the borrow.

```rs
let x = 5u32;
{
    let temp = x; // Compiler generated
    let owned = &own x;
    *owned += 2;
    x = temp; // Compiler generated
}
assert_eq!(x, 5); // Now the assertion passes
```

We suggest the first option, one of the others could also be added later.

## Future possibilities
[future-possibilities]: #future-possibilities

### Add an owned raw pointer `*own T`

Analogous to mutable and shared raw pointers, we could add an owned raw pointer.
Currently, when needing an owned raw pointer, the general choice is `*const T`, since it is covariant in `T`.
An `*own T` would then be a mix of a `*mut T`, which allows mutable borrows, and a `*const T`, which is covariant.

This again places strain on the syntax space, since a contextual keyword may not work in this position.

### Pattern matching with an owning binding mode

Analog to other reference types, we could introduce a new binding mode `own`.
As an example, this enables an ergonomic way to recursively consume a slice:

```rust
fn consume_slice<T>(elements: &own [T]) {    
    match *elements {
        [entry, ref own rest @ ..] => {
            consume_entry(entry);
            consume_slice(elements);
        },
        [] => { }
    }
}
```

Syntactically, this is slightly more challenging since a contextual keyword is more tricky in this position (compared to, e.g., `ref(own)`).
    
### Introduce `DerefOwn` (see [#997](https://github.com/rust-lang/rfcs/issues/997))

Given the simple design of the `Deref` and `DerefMut` traits, it seems obvious to add another trait which produces a value.
However, the obvious design for `DerefMove` using values does not support unsized types, such as slices:

```rs
trait DerefMove: Deref {
    fn deref_own(self) -> Self::Target;
}

fn foo(func: Box<dyn FnOnce()>) {
    // error[E0161]: cannot move a value of type `dyn FnOnce()`
    (*func)();
}

```

With owning references, we could provide a `DerefOwn` trait to allow working with owned unsized types behind an indirection:
```rs
trait DerefOwn: Deref {
    fn deref_own(&own self) -> &own Self::Target;
}

fn foo(func: Box<dyn FnOnce()>) {
    // Could just be `func()` with auto-deref
    (&own *func)();
}
```

However, implementing such a trait is difficult, since it is unclear when the "shell" of a type (e.g., the allocation of `Box`) will be freed.
Therefore, the design of such a trait is left for a future RFC, especially since it may involve additional missing Rust features (e.g., self-referential types);

There is ongoing work in the "field projections" project goal[^field-projections] that attempts to allow equivalent behavior via a more generic `DerefPlace`.

### Interactions with `Move` and `Forget`

The "Immobile types and guaranteed destructors" project goal aims to introduce the `Move` and `Forget` traits in order to supersede pinning.

A type `T: !Move` cannot be moved by value.
However, due to the indirection, `&own T: Move` regardless of `T`.
This makes `&own` essential for transferring ownership of immovable types.

A type `T: !Forget` must be dropped before its backing allocation may be reused.
This trivially makes `&own T: !Forget`, since forgetting the reference is equivalent to forgetting the value.
However, we would additionally require that `&own T: !Leak`, since leaking the reference would allow reusing the backing allocation without ever dropping `T`.

#### Statically-fused futures 

Without `Pin`, the future trait could then mirror the `Iterator` trait mentioned earlier:

```rust
pub enum Poll<'a, F: Future> {
    Ready(F::Output),
    Pending(&'a own F),
}

pub trait Future {
    type Output;

    // Required method
    fn poll(self: &own Self, cx: &mut Context<'_>) -> Poll<'_, Self>;
}
```

In this design, `poll` consumes the future, and returns it again if further polling is required.
With this design, futures are able to produce their final output by moving out of their internal state;
It is statically guaranteed that they cannot be polled again.

Self-referential futures `F: !Move` prevent using `Self` directly, but `&own Self` preserves the ownership-passing semantics while being movable due to the indirection.

### Change `FnOnce` to use `&own`, replacing `unsized_fn_params`

In order to implement `Box<dyn FnOnce()>: FnOnce()`, Rust currently uses the internal and unstable `unsized_fn_params` feature.
If it was possible to migrate the `FnOnce` trait to use `&own self` instead of `self`, we could remove the `unsized_fn_params` feature, or at least not rely on it and provide similar capabilities to user code.

Luckily, the trait method is not stable, so we could change its signature.
However, there are additional concerns regarding compiler internal details, like closure to fn-pointer casts.

Additionally, there exist many other trait methods today that would benefit from changing their signature from `self`  to `&own self`.
However, this transition is semver breaking.
It could be interesting to find a general migration path here for existing code.

## Appendix: Some possibly interesting APIs enabled by `&own`

```rust
// Divide an owned slice into two.
// Note that is impossible with both `Vec` and `Box`, without creating an allocation.
<[T]>::split_at_owned(&own self, usize) -> (&own [T], &own [T]) { ... }

// Pop a value off a vec without copying it.
// Useful if the elements are very large or immovable.
Vec<T>::pop_own(&mut self) -> &own T { ... }

// Extract an owning slice from the `Vec`, without dropping the allocation.
// This way the allocation can be reused, and the slice processed further
// via pattern amtching or slice methods (e.g., splitting).
Vec<T>::take_all(&mut self) -> &own [T] { ... }

// Allows the return value of `Vec::drain` to be converted to an owned slice.
// This way, we can support by-value slice patterns on any subslice of `Vec`.
// Also note that we can reuse the backing allocation, even though we consume the elements.
// `vec.drain().as_owned()` is similar to `vec.take_all()` (with temporary lifetime extension)
Drain<'a, T, A>::as_owned(&mut self) -> &own [T] { ... }
```


[^field-projections]: https://rust-lang.github.io/rust-project-goals/2026/field-projections.html

[^move-trait]: https://rust-lang.github.io/rust-project-goals/2026/move-trait.html
