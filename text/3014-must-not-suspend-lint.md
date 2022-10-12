- Feature Name: `must_not_suspend_lint`
- Start Date: 2020-11-09
- RFC PR: [rust-lang/rfcs#3014](https://github.com/rust-lang/rfcs/pull/3014)
- Rust Issue: [rust-lang/rust#83310](https://github.com/rust-lang/rust/issues/83310)

# Summary
[summary]: #summary

Introduce a `#[must_not_suspend]` lint in the compiler that will warn the user when they are incorrectly holding a struct across an await boundary.

# Motivation
[motivation]: #motivation

Enable users to fearlessly write concurrent async code without the need to understand the internals of runtimes and how their code will be affected. The goal is to provide a best effort warning that will let the user know of a possible side effect that is not visible by reading the code right away.

One example of these side effects is holding a `MutexGuard` across an await bound. This opens up the possibility of causing a deadlock since the future holding onto the lock did not relinquish it back before it yielded control. This is a problem for futures that run on single-threaded runtimes (`!Send`) where holding a lock after a yield will result in a deadlock. Even on multi-threaded runtimes, it would be nice to provide a custom error message that explains why the user doesn't want to do this instead of only a generic message about their future not being `Send`. Any other kind of RAII guard which depends on behavior similar to that of a `MutexGuard` will have the same issue.

The big reason for including a lint like this is because under the hood the compiler will automatically transform async fn into a state machine which can store locals. This process is invisible to users and will produce code that is different than what is in the actual rust file. Due to this it is important to inform users that their code may not do what they expect.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Provide a lint that can be attached to structs to let the compiler know that this struct can not be held across an await boundary.

```rust
#[must_not_suspend = "Your error message here"]
struct MyStruct {}
```

This struct if held across an await boundary would cause a warn-by-default warning:

```rust
async fn foo() {
  let my_struct = MyStruct {};
  my_async_op.await;
  println!("{:?}", my_struct);
}
```

The compiler might output something along the lines of:

```
warning: `MyStruct` should not be held across an await point.
```

Example use cases for this lint:

- `MutexGuard` holding this across a yield boundary in a single threaded executor could cause deadlocks. In a multi-threaded runtime the resulting future would become `!Send` which will stop the user from spawning this future and causing issues. But in a single threaded runtime which accepts `!Send` futures deadlocks could happen.

- The same applies to other such synchronization primitives such as locks from `parking-lot`.

- `tracing::Span` has the ability to enter the span via the `tracing::span::Entered` guard. While entering a span is totally normal, during an async fn the span only needs to be entered once before the `.await` call, which might potentially yield the execution.

- Any RAII guard might possibly create unintended behavior if held across an await boundary.

This lint will enable the compiler to warn the user that the code could produce unforeseen side effects. Some examples of this are:

- [`std::sync::MutexGuard`](https://doc.rust-lang.org/std/sync/struct.MutexGuard.html)
- [`tracing::span::Entered`](https://docs.rs/tracing/0.1.15/tracing/span/struct.Entered.html)

This will be a best effort lint to signal the user about unintended side-effects of using certain types across an await boundary.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `must_not_suspend` attribute is used to issue a diagnostic warning when a value is not "used". It can be applied to user-defined composite types (structs, enums and unions), traits.

The `must_not_suspend` attribute may include a message by using the [`MetaNameValueStr`] syntax such as `#[must_not_suspend = "example message"]`.  The message will be given alongside the warning.

When used on a user-defined composite type, if a value exists across an await point, then this lint is violated.


```rust
#[must_not_suspend = "Your error message here"]
struct MyStruct {}

async fn foo() {
  let my_struct = MyStruct {};
  my_async_op.await;
  println!("{:?}", my_struct);
}
```

When used on a [trait declaration], if the value implementing that trait is held across an await point, the lint is violated.

```rust
#[must_not_suspend]
trait Lock {
    fn foo(&self) -> i32;
}

fn get_lock() -> impl Lock {
    1i32
}

async fn foo() {
    // violates the #[must_not_suspend] lint
    let bar = get_lock();
    my_async_op.await;
    println!("{:?}", bar);
}
```

When used on a function in a trait implementation, the attribute does nothing.

[`MetaNameValueStr`]: https://doc.rust-lang.org/reference/attributes.html#meta-item-attribute-syntax
[trait declaration]: https://doc.rust-lang.org/reference/items/traits.html

# Drawbacks
[drawbacks]: #drawbacks

- There is a possibility it can produce a false positive warning and it could get noisy. But using the `allow` attribute would work similar to other [`warn-by-default`] lints. One thing to note, unlike the `#[must_use]` lint, users cannot silence this warning by using `let _ = bar()` where `bar()` returns a type which has a `#[must_use]` attribute. The `#[allow]` attribute will be the only way to silence the warning.

[`warn-by-default`]: https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Going through the prior art we see two systems currently which provide similar/semantically similar behavior:

## Clippy `await_holding_lock` lint
This lint goes through all types in `generator_interior_types` looking for `MutexGuard`, `RwLockReadGuard` and `RwLockWriteGuard`. While this is a first great step, we think that this can be further extended to handle not only the hardcoded lock guards, but any type which is should not be held across an await point. By marking a type as `#[must_not_suspend]` we can warn when any arbitrary type is being held across an await boundary. An additional benefit to this approach is that this behaviour can be extended to any type which holds a `#[must_not_suspend]` type inside of it.

## `#[must_use]` attribute
The `#[must_use]` attribute ensures that if a type or the result of a function is not used, a warning is displayed. This ensures that the user is notified about the importance of said value. Currently the attribute does not automatically get applied to any type which contains a type declared as `#[must_use]`, but the implementation for both `#[must_not_suspend]` and `#[must_use]` should be similar in their behavior.

### Auto trait vs attribute
`#[must_use]` is implemented as an attribute, and from prior art and [other literature][linear-types], we can gather that the decision was made due to the complexity of implementing true linear types in Rust. [`std::panic::UnwindSafe`][UnwindSafe] on the other hand is implemented as a marker trait with structural composition.

[linear-types]: https://gankra.github.io/blah/linear-rust/
[UnwindSafe]: https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html

# Prior art
[prior-art]: #prior-art

* [Clippy lint for holding locks across await points](https://github.com/rust-lang/rust-clippy/pull/5439)
* [Must use for functions](https://github.com/iopq/rfcs/blob/f4b68532206f0a3e0664877841b407ab1302c79a/text/1940-must-use-functions.md)
* Reference link on how mir transforms async fn https://tmandry.gitlab.io/blog/posts/optimizing-await-2/
# Unresolved questions
[unresolved-questions]: #unresolved-questions


# Common behavior with `#[must_use]` lint

Both `#[must_use]` and `#[must_not_suspend]` are [`warn-by-default`] lints, and are applied to types decorated with the attribute. Currently the `#[must_use]` lint does not automatically propagate the lint in nested structures/enums due to the additional complexity that it adds on top of the possible breaking changes introduced in the wider ecosystem

Automatically propagating the lint for types containing a type marked by one of these attributes would make for a more ergonomic user experience, and would reduce syntactic noise.

While tradeoffs exist for both approaches, in either case, both lints should exhibit the same behavior.

The `#[must_use]` lint is being used in stable rust for a long time now(The earliest reference I could find was in the release notes for [1.27]) with existing behavior.

[1.27]: https://github.com/rust-lang/rust/blob/master/RELEASES.md#version-1270-2018-06-21
