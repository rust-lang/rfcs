- Feature Name: iter_try_all_any
- Start Date: 2022-02-16
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary

[summary]: #summary

Add the associated methods `try_any` and `try_all` to the `Iterator` trait, which work similar to `any` and `all`, but accept `Result` and `Option` as closure return type.

# Motivation

[motivation]: #motivation

In some cases where one would like to use `any`/`all`, the actual condition to check is fallible. For example, let's say we have a simple key-value-store like a `HashMap`. The keys are IDs to the objects. We further have a list of IDs we want to check some condition on. We would like to use `all` on an iterator over the list to ensure all objects match our condition. However, access to elements is fallable and returns `Option`s. In other cases, the checks might involve more complex validation on the objects. The validation method could involve access to external parts and fail. Therefore, it returns a `Result<bool, Error>`.

The implementor needs to return a `bool` for every element in `all`. They could use `.try_fold(true, |state, item| Ok(state && check(item)?))`, but this is less readable and does not short-circuit as `all` does. If they would do `.all(|item| check(item).unwrap_or(false))`, the error would be discarded and lost/ignored. To fully comply with expectations, there is only two possibilites (I see):

- a `for` loop is required to iterate over the elements and stop early on errors or invalid items
- collecting the elements in the iterator to a collected `Result`, then calling `all` on the inner collection

Both solutions are not very elegant. The `try_all` method would work as `.try_all(check)`, returning `Result<bool, Error>` as well. It should behave similar to `all`, except working with `Try<bool>` output, just as `try_fold` behaves in comparison to `fold`.

Everything applies similarly to `try_any`.

# Guide-level explanation

[guide-level-explanation]: #guide-level-explanation

These additions of `try_all` and `try_any` to the core library of Rust allow the use of fallible validators in iterators.

When every element in an iterator needs to be validated, it could be done like this:

```rust
fn file_is_empty(path: &str) -> std::io::Result<bool> {
  Ok(std::fs::read_to_string(path)?.is_empty())
}

fn main() {
    let files = ["fileA.txt", "fileB.txt"];
    let all_exist_but_empty = files.iter().try_all(file_is_empty).expect("file wasn't found");
    let any_exists_but_empty = files.iter().try_any(file_is_empty).expect("file wasn't found");
}
```

It works similarly with `Option`s, e.g. the mentioned key-value-store:

```rust
fn is_valid(object: &MyObject) -> bool {
    *object != MyObject::default()
}

fn main() {
    let mut kv = HashMap::new();
    kv.insert("id-1", MyObject::default());

    let object_list = ["id-1", "id-5"];
    let all_valid = object_list.iter().try_all(|id| kv.get(id).map(is_valid)).expect("id wasn't found");
}
```

# Reference-level explanation

[reference-level-explanation]: #reference-level-explanation

The addition `try_all` behaves to `all` like `try_fold` behaves to `fold`. The similar applies to `try_any`. They both should react as their simple counter-part: short-circuit on elements that already determine the overall result. However two things are different:

- They return the same type as the given function/closure does, so `Option<bool>` or `Result<bool, E>`.
- They also short-circuit on try-fails.

## Exemplary implementation

Inside the Iterator trait:

```rust
fn try_all<F, R>(&mut self, mut f: F) -> R
where
    Self: Sized,
    F: FnMut(Self::Item) -> R,
    R: Try<Output = bool>,
{
    for item in self {
        if !f(item)? {
            return try { false };
        }
    }
    try { true }
}

fn try_any<F, R>(&mut self, mut f: F) -> R
where
    Self: Sized,
    F: FnMut(Self::Item) -> R,
    R: Try<Output = bool>,
{
    for item in self {
        if f(item)? {
            return try { true };
        }
    }
    try { false }
}
```

# Drawbacks

[drawbacks]: #drawbacks

Adding new methods to the trait makes it even bigger, but I would argue that is a low cost (especially in this small case) for a nice benefit.

# Rationale and alternatives

[rationale-and-alternatives]: #rationale-and-alternatives

Another design could require the Iterator's `Item`s to be of the type `Result` already for easier handling fallible `map`s. However, `try_fold` and `try_for_each` already exist and would be expected to behave similar. Changing stable methods would be bad and consistency is highly valuable. Additionally, the proposed implementation could easily deal with `Result`-items as well, as it offers the `and_then` method.

The proposed additions could of course be left out, but they are small additions. They are intuitive to use and blend into the already available picture with `try_fold`, `try_find` and `try_for_each`.

Instead of reacting on `Result<bool, E>` or `Option<bool>`, it could be of arbitrary type and check only whether the result was successful as per `try`. Though, `all` and `any` are constructed to check conditions on a set of items, returning a bool. Returned items by the iterator would be simply dropped. Consequently, using `bool`s provides greater freedom to the user and is closer to the defintion of `all` and `any`.

# Prior art

[prior-art]: #prior-art

I do not know similar implementations in other languages. It is probably rare due to Rust's unique method of handling errors with `Try`, `Option` and `Result`.

There is the [fallible_iterator](https://docs.rs/fallible-iterator/latest/fallible_iterator/) crate, which re-invents the `Iterator` trait for fallible cases. It has `try_all` implemented named `all`, but it only works on `Result`s and leads to a lot ambiguity in the usage of iterators.

There is [this blog post](https://blog.yoshuawuyts.com/fallible-iterator-adapters/) about adding more fallible operations to iterators.

# Unresolved questions

[unresolved-questions]: #unresolved-questions

I do not know of any at this point in time.

# Future possibilities

[future-possibilities]: #future-possibilities

There is the option for many more methods for fallible iterator operation, as described in [this blog post](https://blog.yoshuawuyts.com/fallible-iterator-adapters/).

There is also the possibility to add `map_ok` and `map_err` to help handling iterators over `Result` items.
