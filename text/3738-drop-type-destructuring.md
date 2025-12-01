- Feature Name: `drop_type_destructuring`
- Start Date: 2024-12-08
- RFC PR: [rust-lang/rfcs#3738](https://github.com/rust-lang/rfcs/pull/3738)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Rust does not allow destructuring types which implement the `Drop` trait. This means that moving data out of such types is hard and error prone. The rationale is that once fields are moved out, the type's `Drop` implementation cannot run, which can be undesired. This RFC proposes to allow destructuring anyway, in certain situations.

# Motivation
[motivation]: #motivation

There are real use-cases in which you do want to move out of types that implement `Drop`. Two main types of patterns that often need it are:

- implementing methods like `into_raw_parts` which explicitly hand over their internals and move the burden of dropping those
- "guard" types which might need to be "disarmed"

In both cases, you want to avoid the regular `Drop` implementation from running at all and get ownership of the fields.

Right now, code that wants to do this needs to use `unsafe`. For example, [`std::io::BufWriter::into_parts()`](https://doc.rust-lang.org/stable/std/io/struct.BufWriter.html#method.into_parts) has to perform the following gymastics using `ManuallyDrop`:

```rust
pub fn into_parts(self) -> (W, Result<Vec<u8>, WriterPanicked>) {
    let mut this = ManuallyDrop::new(self);
    let buf = mem::take(&mut this.buf);
    let buf = if !this.panicked { Ok(buf) } else { Err(WriterPanicked { buf }) };

    // SAFETY: double-drops are prevented by putting `this` in a ManuallyDrop that is never dropped
    let inner = unsafe { ptr::read(&this.inner) };

    (inner, buf)
}
```

When writing this RFC, we even spent some time to make sure this example from `std` was even correct. By exposing `drop_type_destructuring`, we can reduce the complexity of such use cases.

Through this, we can avoid `Drop`ping types for which running `Drop` is very important. However, this is already the case because you could use `mem::forget`.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`drop_type_destructuring` adds a new built-in macro, `destructure`. You can use this macro to destructure types that implement `Drop`. For example, assuming `Foo` is a type that implements `Drop` and has fields `a` and `b`, you could write the following:

```rust
fn example(x: Foo) {
    destructure!(let Foo { a, b: _ } = x);
    // yay we own `a` now, we can even drop it!
    drop(a);
}
```

This moves the field `a` out of `x`, and immediately drops `b`.
Instead of creating a new binding, you can also assign to an existing one by leaving out the `let`, similarly to destructuring assignment:

```rust
fn example(x: Foo) {
    let mut a = vec![1, 2, 3];
    
    destructure!(Foo { a, b: _ } = x);
    // yay we own `a` now, we can even drop it!
    drop(a);
}
```

When a type implements `Drop`, this can be because the order in which fields are dropped is very important. It could be unsound to do it in the wrong order. When you write a destructuring, soundly dropping the fields is now your responsibility.

This can be a heavy burden, so if you are the author of a module or crate, you might want to limit other people destructuring your type. The rule for this is that you can only use `destructure!()` on types in a location where you could also construct that type. This means that in that location, all fields must be visible.

Importantly, this means that this code is not valid:

```rust
mod foo {
    pub struct Bar {
        pub a: Vec<u8>,
        b: Vec<u8>,
    }
}

fn example(x: foo::Bar) {
    destructure!(let Foo{ a, .. } = x)
}
```

By using `..`, we could ignore the fact that `b` was not visible and move out `a` anyway. This is undesirable, as it would implicitly run `drop(b)` even though `b` was not accessible here. In fact, if there was a requirement that `a` was dropped before `b`, it would never be sound to destructure a type `Bar` in a location where `b` is not accessible.

Finally, you might ask why we need a macro for this at all. This is because for some types, the behavior is slightly different. 
Let's use the following example. In Rust right now, this does not compile:

```rust
struct Foo {
    a: Vec<u64>
}

impl Drop for Foo {
    fn drop(&mut self) {}
}

fn example(x: Foo) {
    // error: cannot move out of type `Foo`, which implements the `Drop` trait
    let Foo { a } = x;
}
```

This is because we move the field `a` out, and `Foo` implements `Drop`.
The following does compile:

```rust
struct Foo {
    a: Vec<u64>
}

impl Drop for Foo {
    fn drop(&mut self) {}
}

fn example(x: Foo) {
    // fine, because we don't actually move any fields out
    let Foo { .. } = x;
}
```

Similarly, the following also works because we don't have to move out any fields, we can just copy `a`, because `u64` implements `Copy`.

```rust
struct Foo {
    a: u64
}

impl Drop for Foo {
    fn drop(&mut self) {}
}

fn example(x: Foo) {
    // Totally fine, we just copy a
    let Foo { a } = x;
}
```

In the two examples above, `Drop` still runs for `x`.
`destructure!()` represents a different operation. When we use it, we actually move the fields out of the type, and prevent `Drop` from running.

```rust
struct Foo {
    a: u64
}

impl Drop for Foo {
    fn drop(&mut self) {}
}

fn example(x: Foo) {
    // `a` is moved out of `x`, Drop never runs for `x`
    destructure!(let Foo { a } = x);
}
```

Preventing `Drop` from running is not inherently problematic in Rust. You can already do this for any type through `mem::forget` (and other means), which is safe.


Finally, the following is an example of how one could implement the example from [Motivation](#motivation):

```rust
pub fn into_parts(self) -> (W, Result<Vec<u8>, WriterPanicked>) {
    let mut this = ManuallyDrop::new(self);
    let buf = mem::take(&mut this.buf);
    let buf = if !this.panicked { Ok(buf) } else { Err(WriterPanicked { buf }) };

    // SAFETY: double-drops are prevented by putting `this` in a ManuallyDrop that is never dropped
    let inner = unsafe { ptr::read(&this.inner) };

    (inner, buf)
}
```

Now with `destructure!`:

```rust
pub fn into_parts(self) -> (W, Result<Vec<u8>, WriterPanicked>) {
    destructure!(let Self { buf, inner, panicked } = self);
    let buf = if !panicked { Ok(buf) } else { Err(WriterPanicked { buf }) };

    (inner, buf)
}
```

We don't even need unsafe anymore :tada:

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

A built-in macro with the following "signature" is added to the standard library:

```rust
#[macro_export]
macro_rules! destructure {
    ((let)? $pat:pat = $e:expr) => { /* built-in */ }
}
```

The result of the expansion checks the following requirements and produces a compilation error if any of them are not met:
- `$pat` must be an irrefutable pattern
- `$e` must be an owned expression that can be matched by pattern `$pat`
- `$e`'s type must be an ADT (`struct` or `enum`)
- The type of `$e` must be constructable in the current context, i.e.
    - All fields must be visible to the current module wrt privacy
    - If the type is marked with `#[non_exhaustive]`, it must be defined in the current crate

The semantic of this macro is equivalent to the following:
1. Wrapping `$e` in a `ManuallyDrop`
2. Copying **all** fields of `$e` (for enums all fields of the active variant)
3. If the `let` token is present in the macro invocation, fields that are listed in `$pat` are binded to their respective bindings or patterns. Otherwise, they are assigned similarly to destructuring assignment
4. If `..` is present in `$pat`, all fields that are not mentioned in it are dropped in the order of definition (simialrly to how [normal `let`/destructuring assignment works](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=3abd3aebd3378690ff3d2006e12d4120))

# Drawbacks
[drawbacks]: #drawbacks

When implementing this change as a macro expanding to a statement, we do not think there are many drawbacks. However, in the coming section with alternatives we discuss another possible expansion to a pattern which may have a few advantages, but also has many drawbacks which we discuss there.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Do not support destructuring assignment

Instead of allowing optional `let` token, we could always ommit it, and consider `destructure!` to always introduce new bindings.

It's not evident that destructure assignment is particularly useful in situations where `destructure!` would be used.

## Expand to a pattern

Instead of making this macro expand to a statement, containing a pattern and a right hand side, one could imagine a macro that expands *just* to a pattern:

```rust
let destructure!(Foo { a, b }) = x;
```

On its own this could be considered, it also means that whether to use `let` or not (the previous alternative) isn't a concern for the macro. However, by doing this, we might imply that you can also use `destructure!` in other places you might see a pattern:

```rust
let destructure!(Foo::A { a }) = x else {
    panic!("not Foo::A")
};

match x {
    destructure!(Foo::A { a }) => /* ... */,
    Foo::B { .. } => panic!("unexpected Foo::B")
}
```

Now, it makes sense that patterns in `destructure!` can be refutable. Even this can be okay, as long as only the top-level pattern is refutable. If we allow nested patterns to be refutable as well, we can construct the following scenario:

```rust
match y {
    destructure!(Bar { a, b: 1 }) => /* ... */,
    _ => panic!(".b != 1")
}
```

Here, if we were to move fields out of `Bar` from left to right, we might move `a`, then check whether `b` matches `1`, and if it doesn't we already moved `a` out. 

In theory this is not impossible to implement. By making this expand to code that uses `ManuallyDrop`, and when a pattern is rejected semantically move values back. However, one can construct even more complicated scenarios than just this one, and actually defining the semantics of this operations quickly becomes incredibly complicated.

Just as a small example of that, consider this:

```rust
match z {
    destructure!(Baz { a, b: destructure!(Bar { a, b: 1 }) }) => /* ... */,
    _ => panic!(".b.b != 1")
}
```

Where both `Baz` and `Bar` implement `Drop`.

As a final note on this, making `destructure!()` expand to a pattern on its own is possible, and even refutability is possible if we only allow "one level" of refutability (`Some(_)`, but not `Some(1)`). However, we simply do not think there are many good uses for this. 
The one scenario we could imagine is that might be useful for `destructure!()` on `enum`s, but we also think that even using `destructure!()` on `enum`s will be very uncommon (as types implementing `Drop` are almost always `struct`s) and might not outweigh the increased implementation complexity and the fact that we need to communicate this one level matching rule to users.

## Macro expression and Magic type

Instead of having a macro be a statement or pattern, we could make it an expression returning a type (say `Destructure<T>`) with magic similar to that of a `Box` and `ManuallyDrop`. Specifically, it would:
1. Allow moving `T`'s fields out of `Destructure<T>` even if `T: Drop`
2. Not drop `T` when `Destructure<T>` is dropped
3. But drop all fields of `T` when `Destructure<T>` is dropped (except the moved-out ones)
4. Be able to be matched directly with a pattern that would match `T`

Note that having a macro produce `Destructure<T>`, as opposed to a function is still required, since we still want to check that `T` is constructable in the current context.

This would allow writing the `BufWriter::into_parts` example from [Motivation](#motivation) like this:

```rust
pub fn into_parts(self) -> (W, Result<Vec<u8>, WriterPanicked>) {
    let Self { buf, inner, panicked } = destructure!(self);
    let buf = if !panicked { Ok(buf) } else { Err(WriterPanicked { buf }) };

    (inner, buf)
}
```

Or closer to the original code:

```rust
pub fn into_parts(self) -> (W, Result<Vec<u8>, WriterPanicked>) {
    let this = destructure!(self);
    let buf = this.buf;
    let buf = if !this.panicked { Ok(buf) } else { Err(WriterPanicked { buf }) };

    (this.inner, buf)
}
```

The upside of this is that `Destructure` type can be used as input into functions which are expected to not drop `T`, such as potential `DropOwned`:

```rust
struct X(Vec<u8>);

// This is explicitly *not* proposed as part of this RFC,
// but is a kind of pattern that could be allowed by `Destructure`.
impl DropOwned for X {
    fn drop(self: Destructure<Self>) {
        drop(self.0); // ownership!
    }
}
```

The downside of this approach is that it is significantly harder to specify, implement, and teach.

# Prior art
[prior-art]: #prior-art

@WaffleLapkin wrote a crude polyfill for this using a macro and `ManuallyDrop`:

```rust
/// Destructures `$e` using a provided pattern.
///
/// Importantly, this works with types which implement `Drop` (ofc, this doesn't run the destructor).
#[macro_export]
macro_rules! destructure {
    ($Type:ident { $($f:tt $(: $rename:pat)? ),+ $(,)? } = $e:expr) => (
        // FIXME: use $crate:: paths
        let tmp = $crate::unstd::_macro_reexport::core::mem::ManuallyDrop::new($e);

        // assert that `$e` is an owned expression, rather than `&Type`
        if false {
            #[allow(unreachable_code)]
            let _assert_owned_expr = [&tmp, &$crate::unstd::_macro_reexport::core::mem::ManuallyDrop::new($Type { $($f: todo!()),* })];
        };

        $(
            let $crate::destructure!(@_internal_pat_helper $f $($rename)?)
                // safety: `$e` is of type `$Type<..>` (as asserted above),
                //         so we have ownership over it's fields.
                //         `$f` is a field of `$Type<..>` (as asserted above).
                //         `$e` is moved into a `ManuallyDrop`, which means its `drop` won't be run,
                //         so we can safely move out its
                //         the pointer is created from a reference and is thus valid
                = unsafe { $crate::unstd::_macro_reexport::core::ptr::read(&tmp.$f) };
        )+

        // remove the temporary we don't need anymore.
        // doesn't actually drop, since `ManuallyDrop`.
        _ = {tmp};
    );
    (@_internal_pat_helper $f:tt) => ($f);
    (@_internal_pat_helper $f:tt $rename:pat) => ($rename);
}
```
Example usage:
```rust
// `destructure` does not run the destructor, so this **doesn't** unlock the lock.
destructure!(Lock { file, mode: _ } = self);
```
This polyfill has multiple downsides, such as:
- It does not support ommiting fields with `..` (this is not possible to support in the polyfill, as you are unable to prove that an expression is owned, without specifying its full type)
- It doesn't work with enums with multiple variants, even if you could specify an irrefutable pattern (this theoretically can be supported, but is quite complicated)
- It does not work with tuple struct syntax (impossible to support arbitrary number of fields, hard to support a set number)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

These mainly relate to the above-mentioned alternatives:

- Do we support `let` inside?
- Should `destructure!()` expand to a statement, or a pattern?

# Future possibilities
[future-possibilities]: #future-possibilities

If we were to choose for the version that expands to a pattern, we could try to define the semantics of refutable patterns with more than one level, assuming this is even possible in a satisfactory and importantly sound manner.

In general, we could add some lints in relation to this feature.
General usage of `destructure!` is often okay, especially in places where there is visibility of all fields, but it could be used in unintentional ways so a lint in `clippy::pedantic` for using `destructure` might be considered.
A lint for using `destructure!` on types that don't implement `Drop` at all makes sense regardless. The suggestion there can be to remove `destructure!` completely as it is not necessary.

