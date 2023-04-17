- Feature Name: (`dropck_eyepatch_v3`)
- Start Date: (fill me in with today's date, YYYY-MM-DD)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Cleanup the rules for implicit drops by splitting `#[may_dangle]` into two separate attributes:
`#[only_dropped]` and `#[fully_ignored]`. Change `PhantomData` to get completely ignored
by dropck as its current behavior is confusing and inconsistent.

# Motivation
[motivation]: #motivation

The current rules around dropck and `#[may_dangle]` are confusing and have even resulted in
unsoundness [multiple](https://github.com/rust-lang/rust/issues/76367)
[times](https://github.com/rust-lang/rust/issues/99408). Even without `#[may_dangle]`,
dropping `PhantomData` is currently quite weird as you get "spooky-dropck-at-a-distance":

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
`#[fully_unused]` or `#[only_dropped]` to that parameter. This **unsafely** asserts
that the parameter is either completely unused when dropping your type or only
recursively dropped.

```rust
struct MyType<'s> {
    reference: &'s str,
    needs_drop: String,
}
// The impl has to be `unsafe` as the compiler does may not check
// that `'s` is actually unused.
unsafe impl<#[only_dropped] 's> Drop for MyType<'s> {
    fn drop(&mut self) {
        // println!("{}", reference); // this would be unsound
        println!("{}", needs_drop);
    }
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
    // This is accepted as `'s` is marked as `#[only_dropped]` in the
    // `Drop` impl of `MyType`.
}
```

The ability to differentiate between `#[fully_unused]` and `#[only_dropped]` is significant
for type parameters:

```rust
pub struct BTreeMap<K, V> {
    root: Option<Root<K, V>>,
    length: usize,
}

unsafe impl<#[only_dropped] K, #[only_dropped] V> Drop for BTreeMap<K, V> {
    fn drop(&mut self) {
        // Recursively drops the key-value pairs but doesn't otherwise
        // inspect them, so we can use `#[only_dropped]` here.
        drop(unsafe {ptr::read(self) }.into_iter())
    }
}
```

A type where `#[fully_unused]` would be useful is a `Weak` pointer for a variant of `Rc`
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
    - they are marked with `#[fully_unused]`, in which case they are ignored,
    - or they are marked with `#[only_dropped]`, in which case recurse into the generic argument.
- Regardless of whether `T` implements `Drop`, recurse into all types *owned* by `T`:
    - references, raw pointers, function pointers, function items and scalars do not own
      anything. They can be trivially dropped.
    - tuples and arrays consider their element types to be owned.
    - all fields (of all variants) of ADTs are considered owned. We consider all variants
      for enums. The only exception here is `ManuallyDrop<U>` which is not considered to own `U`. `PhantomData<U>` does not have any fields and therefore also does not consider
      `U` to be owned.
    - closures and generators own their captured upvars.

Checking drop impls may error for generic parameters which are known to be incorrectly marked:
- `#[fully_unused]` parameters which are recursively owned
- `#[only_dropped]` parameters which are required to be live by a recursively owned type

This cannot catch all misuses, as the parameters can be incorrectly used by the `Drop` impl itself.
We therefore require the impl to be marked as `unsafe`.

## How this differs from the status quo

Instead of `#[fully_unused]` and `#[only_dropped]`,there is only the `#[may_dangle]` attribute which
skips the generic parameter. This is equivalent to the behavior of `#[fully_unused]` and relies on the recursion
into types owned by `T` to figure out the correct constraints.

`PhantomData<U>` currently considers `U` to be owned while not having drop glue itself. This means
that `(PhantomData<PrintOnDrop<'s>>, String)` requires `'s` to be live while
`(PhantomData<PrintOnDrop<'s>>, u32)` does not. This is required for get the
behavior of `#[only_dropped]` for parameters otherwise not owned by adding `PhantomData` as a field.
One can easily forget this, which caused the [unsound](https://github.com/rust-lang/rust/issues/76367)
[issues](https://github.com/rust-lang/rust/issues/99408) mentioned above.

# Drawbacks
[drawbacks]: #drawbacks

It requires an additional attribute when compared with `#[may_dangle]` and also proposes checks that the
attributes are correctly used. This adds a small amount of implementation complexity to the compiler.
These new attributes are still not fully checked by the compiler and require `unsafe`.

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
in the near future. The benefits from such are change are likely to be fairly limited while
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

Extending or generalizing the dropck eyepatch... something something type state.

[`IndexVec`](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_index/vec/struct.IndexVec.html)