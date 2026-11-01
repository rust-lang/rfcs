- Feature Name: (`dropck_eyepatch_v3`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Cleanup the rules for implicit drops by adding an argument for `#[may_dangle]` on type
parameters: `#[may_dangle(droppable)]` and `#[may_dangle(must_not_use)]`. Change `PhantomData`
to get completely ignored by dropck as its current behavior is confusing and inconsistent.

# Motivation
[motivation]: #motivation

`PhantomData` is `Copy` but still adds outlives requirements when dropped as a part of
a larger value. This behavior is inconsistent and results in "spooky-dropck-at-a-distance" 
even without `#[may_dangle]`:

```rust
use std::marker::PhantomData;
struct PrintOnDrop<'a>(&'a str); // requires `'a` to be live on drop.
impl Drop for PrintOnDrop<'_> {
    fn drop(&mut self) {
        println!("{}", self.0)
    }
}

fn assign<T>(_: T) -> PhantomData<T> {
    PhantomData
}

// The type of `_x` is `AdtNoDrop<'not_live>` which doesn't have drop glue, OK
struct AdtNoDrop<'a>(PhantomData<PrintOnDrop<'a>>, u32);
fn phantom_data_adt_no_drop() {
    let _x;
    {
        let temp = String::from("temporary");
        _x = AdtNoDrop(assign(PrintOnDrop(&temp)), 0);
    }
}

// The type of `_x` is `AdtNoDrop<'not_live>` which has drop glue, ERROR
struct AdtNeedsDrop<'a>(PhantomData<PrintOnDrop<'a>>, String);
fn phantom_data_adt_needs_drop() {
    let _x;
    {
        let temp = String::from("temporary");
        _x = AdtNeedsDrop(assign(PrintOnDrop(&temp)), String::new());
    }
}
```
[playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=9ce9d368d2f13df9ddcbfaf9580721e0)

`#[may_dangle]` is currently difficult to use correctly. It has resulted in
unsoundness [multiple](https://github.com/rust-lang/rust/issues/76367)
[times](https://github.com/rust-lang/rust/issues/99408). `#[may_dangle]` restricts
the impl of `fn Drop::drop`. Whether this method is allowed to drop `T` depends
on the fields of the ADT. While changing `#[may_dangle]` to explicitly state its intended
behavior is necessary due to the change to `PhantomData`, this also simplify its usage.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

When a value goes out of scope the compiler adds drop glue for that value, recursively dropping it and all its fields.
Dropping a type containing a lifetime which is no longer live is accepted if that lifetime is never accessed:
```rust
struct MyType<'s> {
    reference: &'s str,
    needs_drop: String,
}
fn can_drop_dead_reference() {
    let _x;
    {
        let temp = String::from("I am only temporary");
        _x = MyType {
            reference: &temp,
            needs_drop: String::from("I have to get dropped"),
        };
    }
    // We drop `_x` here even though `reference` is no longer live.
    //
    // This is fine as dropping a reference is a noop and does not
    // acess the pointee.
}
```
The above example will however fail if we add a manual `Drop` impl as the compiler conservatively
assumes that all generic parameters of the `Drop` impl are used:
[playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=e604bcaecb7b2b4cf7fd0440faf165ac).

In case a manual `Drop` impl does not access a generic parameter, you can add
`#[may_dangle]` to that parameter. This **unsafely** asserts
that the parameter is either completely unused when dropping your type or only
recursively dropped. For type parameters, you have to declare whether you
recursively drop instances of `T`. If so, you should use `#[may_dangle(droppable)]`.
If not, you may use `#[may_dangle(must_not_use)]`.

```rust
struct MyType<T> {
    generic: T,
    needs_drop: String,
}
// The impl has to be `unsafe` as the compiler may not check
// that `T` is actually unused.
unsafe impl<#[may_dangle(droppable)] T> Drop for MyType<T> {
    fn drop(&mut self) {
        // println!("{}", self.generic); // this would be unsound
        println!("{}", self.needs_drop);
    }
}
fn can_drop_dead_reference() {
    let _x;
    {
        let temp = String::from("I am only temporary");
        _x = MyType {
            generic: &temp,
            needs_drop: String::from("I have to get dropped"),
        };
    }
    // We drop `_x` here even though `reference` is no longer live.
    //
    // This is accepted as `T` is marked as `#[may_dangle(droppable)]` in the
    // `Drop` impl of `MyType`.
}
```

`Drop` impls for collections tend to require `#[may_dangle(droppable)]`:

```rust
pub struct BTreeMap<K, V> {
    root: Option<Root<K, V>>,
    length: usize,
}

unsafe impl<#[may_dangle(droppable)] K, <#[may_dangle(droppable)] V> Drop for BTreeMap<K, V> {
    fn drop(&mut self) {
        // Recursively drops the key-value pairs but doesn't otherwise
        // inspect them, so we can use `#[only_dropped]` here.
        drop(unsafe {ptr::read(self) }.into_iter())
    }
}
```

A type where `#[may_dangle(must_not_use)]` would be useful is a `Weak` pointer for a variant of `Rc`
where the value is dropped when the last `Rc` goes out of scope. Dropping a `Weak` pointer
would never access `T` in this case.


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Whenever we use a value of a given type this type has to be **well-formed**, requiring
that all lifetimes in this type are live. An exception to this is the implicit drop when
a variable goes out of scope. While borrowck ensures that dropping the variable is safe,
this does not necessarily require all lifetimes to be live.

When implicitly dropping a variable of type `T`, liveness requirements are computed as follows:
- If `T` does not have any drop glue, do not add any requirements.
- If `T` is a trait object, `T` has to be live.
- If `T` has an explicit `Drop` impl, require all generic argument to be live, unless
    - they are marked with `#[may_dangle]`:
        - arguments for lifetime parameters marked `#[may_dangle]` and type parameters
          marked `#[may_dangle(must_not_use)]` are ignored,
        - we recurse into arguments for type parameters marked `#[may_dangle(droppable)]`.
- Regardless of whether `T` implements `Drop`, recurse into all types *owned* by `T`:
    - references, raw pointers, function pointers, function items and scalars do not own
      anything. They can be trivially dropped.
    - tuples and arrays consider their element types to be owned.
    - all fields (of all variants) of ADTs are considered owned. We consider all variants
      for enums. The only exception here is `ManuallyDrop<U>` which is not considered to own `U`.
      `PhantomData<U>` does not have any fields and therefore also does not consider
      `U` to be owned.
    - closures and generators own their captured upvars.

Checking drop impls may error for generic parameters which are known to be incorrectly marked:
- `#[may_dangle(must_not_use)]` parameters which are recursively owned
- `#[may_dangle(droppable)]` parameters which are required to be live by a recursively owned type

This cannot catch all misuses, as the parameters can be incorrectly used by the `Drop` impl itself.
We therefore require the impl to be marked as `unsafe`.

## How this differs from the status quo

Right now there is only the `#[may_dangle]` attribute which skips the generic parameter.
This is equivalent to the behavior of `#[may_dangle(must_not_use)]` and relies on the recursion
into types owned by `T` to figure out the correct constraints. This is now explicitly annotated
using `#[may_dangle(droppable)]`.

`PhantomData<U>` currently considers `U` to be owned while not having drop glue itself. This means
that `(PhantomData<PrintOnDrop<'s>>, String)` requires `'s` to be live while
`(PhantomData<PrintOnDrop<'s>>, u32)` does not. This is required for get the
behavior of `#[may_dangle(droppable)]` for parameters otherwise not owned by adding `PhantomData`
as a field. One can easily forget this, which caused the
[unsound](https://github.com/rust-lang/rust/issues/76367)
[issues](https://github.com/rust-lang/rust/issues/99408) mentioned above.

# Drawbacks
[drawbacks]: #drawbacks

This adds a small amount of implementation complexity to the compiler while not not being
fully checked and therefore requiring `unsafe`.

This RFC does not explicitly exclude stabilizing these two attributes, as they are clearer and far less
dangerous to use when compared with `#[may_dangle]`. Stabilizing these attributes will make it harder to
stabilize a more general solution like type state.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The status quo of `#[may_dangle]` and "spooky-dropck-at-a-distance" is far from ideal and has already
resulted in unsound issues. Documenting the current behavior makes is more difficult to change later
while not officially documenting it is bound to lead to more issues and confusion going forward.
It is therefore quite important to improve the status quo.

A more general extension to deal with partially invalid types is far from trivial. We currently
assume types to always be well-formed and any approach which generalizes `#[may_dangle]` will
have major consequences for how well-formedness is handled. This impacts many - often implicit -
interactions and assumptions. It is highly unlikely that we will have the capacity for any such change
in the near future. The benefits from such are change are also likely to be fairly limited while
adding significant complexity.

# Prior art
[prior-art]: #prior-art

`#[may_dangle]` is already a refinement of the previous `#[unsafe_destructor_blind_to_params]` attribute
([RFC 1327](https://github.com/rust-lang/rfcs/pull/1327)).

There is also [RFC 3390](https://github.com/rust-lang/rfcs/pull/3390) which attempts to define a more
general extension to replace `#[may_dangle]`. As mentioned in the rationale, such an approach is not
feasable right now.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should these attributes remain purely unstable for use in the standard library or do we want
to provide them to stable users?


# Future possibilities
[future-possibilities]: #future-possibilities

Part of the motivation for this RFC is that reasoning about `#[may_dangle]` through field
ownership is subtle and easy to get wrong. This applies equally to other
properties of types: variance and auto traits. We may want to look into reducing our
reliance on this kind of reasoning, at least in the presence of unsafe code.

Extending or generalizing the dropck eyepatch... something something type state.

[`IndexVec`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_index/vec/struct.IndexVec.html)