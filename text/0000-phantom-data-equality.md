- Feature Name: phantom_data_equality
- Start Date: 2015-11-20
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

PhantomData is always empty and thus equal. As a result the existance of PhantomData<T> in
a structure A which does not otherwise contain T should not require T to implement
equality for equality to be defined on A.

# Motivation
[motivation]: #motivation

I as a user want to use the following structure:

```rust
struct Id<T>(i32, PhantomData<T>)
```

I want to have equality defined on an `Id<T>` even if `T` itself does not implement
`Eq` or `PartialEq`. The following:

```rust
#[derive(Eq, PartialEq)]
struct Id<T>(i32, PhantomData<T>)
```

Will not work, complaining that equality is not defined for `T`. By having 
`PhantomData<T>` implement `Eq` and `PartialEq` as functions which are always true
would resolve this issue.

# Detailed design
[design]: #detailed-design

The implementation of this change should be trivial, and very similar to the
existing implementations of `Sync`, `Send` and `Sized`.

```rust
impl PartialEq<Rhs = T> for PhantomData<T> {
  fn eq(&self, _other: &Rhs) -> bool { true }
  fn ne(&self, _other: &Rhs) -> bool { false }
}

impl Eq for PhantomData<T>
```

`PartialEq` will only be implemented by default on values of the same enclosed
type. This is likely the expected behaviour in this case, as in the following
example.

```rust
#[derive(Eq, PartialEq)]
struct Id<T>(i32, PhantomData<T>)

struct Apple;
struct Orange;

fn main() {
  let a = Id::<Apple>(10, PhantomData);
  let b = Id::<Orange>(10, PhantomData);
  println!(a.eq(b))
}
```

The equality of two things based on their types is a buisness rule, and too
much for us to infer.

# Drawbacks
[drawbacks]: #drawbacks

None that I can see. Comments here appreciated.

# Alternatives
[alternatives]: #alternatives

Alternatively we could define equality for `PhantomData<T>` to `PhantomData<R>` for
any `R`. I belive this to be unexpected behavior, as shown in the example above.

# Unresolved questions
[unresolved]: #unresolved-questions

None.
