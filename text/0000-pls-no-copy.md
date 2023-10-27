- Feature Name: `pls-no-copy`
- Start Date: 2023-10-21
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Adds a special attribute `#[should_move]` that can be applied to function arguments which implement `Copy`. This allows "opting into" move semantics for types that implement `Copy` to avoid mistakes.

This is marked as an "experimental" RFC since the end goal of this feature, allowing API authors to implement `Copy` without fear of causing bugs, may not actually be achieved by the feature as described alone. A follow-up RFC will likely be written for the final version of this attribute if it ends up being substantially different than the one described.

A good benchmark for when this feature is good enough is being able to implement `Copy` for standard library iterators (like the `Range` types) and to retire the `clippy::copy_iterator` lint for good.

# Motivation
[motivation]: #motivation

Move semantics are a crucial part of Rust for ensuring correctness, but they're also extremely cumbersome for data that's easy to copy. This is why Rust also offers the `Copy` trait for things to explicitly opt out of move semantics, ensuring a bit-by-bit copy every time.

However, this falls apart completely when copyable values contain state, most notably for iterators. Before Rust 1.0, the libs team decided to [remove `Copy` implementations from iterators](https://github.com/rust-lang/rust/pull/21846#issuecomment-843201267), and it's worth revisiting a way this can be added back.

It's worth starting from the grounds that implementing `Copy` for a type doesn't *change* the behaviour of using that type; it simply *adds* new behaviors that weren't possible before. This is important to mention because it means that accidental copies could be converted from hard errors into deny-by-default lints and the result would be effectively the same, and that there aren't any other subleties to address.

## `#[must_use]`

Note that the space of potential problems is also substantially reduced by the `#[must_use]` attribute. Take this example of a simple builder API:

```rust
let mut builder = Credentials::new();
builder.with_name("Developer");
builder.with_quest("Development");
builder.with_favorite_color("As an AI language model, I");
builder.build()
```

With a builder API that works with mutable references, this API will work correctly regardless of whether the builder implements `Copy`. However, if the builder uses by-value methods, this will fail unless the builder implements `Copy`, and the resulting `build()` call will act as if none of the methods had been called at all.

With the addition of `#[must_use]`, this problem effectively goes away, since we can simply mark the builder and/or its methods as `#[must_use]`, and the compiler will emit a warning whenever a builder method is called and the result is discarded.

## Accidental `Copy`

The main issue with `Copy` arises for users who aren't used to the borrow mechanics of Rust and are used to "passing references by value" as in languages like Java:

```rust
let mut tasks = vec![/* ... */];
let mut iter = tasks.iter();
let h1 = std::thread::spawn(move || handler(iter));
let h2 = std::thread::spawn(move || handler(iter));
h1.join();
h2.join();
```

If only concurrency were that easy! A user might think that these separate handlers are both working on the same iterator, and any given task would be processed by a single handler, but actually, the iterator is copied to both threads and the tasks are processed twice.

And these are the kinds of "beginner papercuts" that removing `Copy` from types in general is avoiding: while the *data* that constitutes a state can be easily passed around, the ability to *manipulate* that data is not, and users might conflate these two ideas.

Sure, for this tiny iterator example, most experience Rust users will be able to point this out as incorrect. However, in general, it's very easy to accidentally conflate state with a handle to modify that state, and more advanced APIs can often obfuscate this behind types like `Handle<'_>` or `State` and might not be as clear as `&mut T` and `T`. Overall, the point stands that the community prefers not implementing `Copy` for some types as a result of this, and it avoids the possibility of running into these issues.

## What We Lose

The principal example of why removing `Copy` from iterators is harmful is it makes parsers more annoying. There are definitely other reasons, but this is just an obvious one that feels like pointing out, and it's pretty representative of the issues we create by removing `Copy`.

In parsers, it's extremely common to keep track of a "span" of input, which is just a range that corresponds to the original source of the parsed value. While spans could be recorded as strings, they're less useful this way, since spans can be combined in ways ordinary strings cannot: if I give you the spans of an open and close parenthesis, I can compute the span of the thing in parentheses, which is useful, but if I just give you the strings "(" and ")", that is not useful. (I know what parentheses look like, thanks.)

The point is, `Range<usize>` is an iterator, and therefore, it doesn't implement `Copy`. This means that spanned tokens, an incredibly common piece of data when implementing parsers, cannot implement `Copy`, which is annoying. When writing parsers, I effectively create my own version of `Range<usize>` which implements `Copy` and add a bunch of `From` implementations so that anything containing a span can itself derive `Copy`.

And for what reason? Avoiding `Copy` for data that should implement it feels like a solution begging for a problem. So, now that Rust 1.0 is done and we have the time to think about a proper solution, let's make one.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `#[should_move]` attribute is a way of selectively opting out of copy semantics for types that implement `Copy`. In general, it's advisable to implement `Copy` for any type that's reasonable to implement it, although there are some cases where copying a value instead of moving one is likely a mistake.

`#[should_move]` is marked on function arguments that implement `Copy`. As an exception, immutable references (`&T`) are not allowed to be marked `#[should_move]` for reasons explained later. Here's an example:

```rust
fn manipulate_state(#[should_move] handle: Handle<'_>) -> StateResult<Handle<'_>> {
    // ...
}
```

Since `Handle`s take up little memory, they implement `Copy`. However, in this case, the handle is invalidated if an error occurs, and the user has to request a new one in this case. Without this annotation, the user could accidentally perform the following:

```rust
let handle = request_handle();
if let Err(err) = manipulate_state(handle) {
    eprintln!("oh no! {err}");
}
do_something_else(handle);
```

In this case, the user knows that they can continue if the function fails, but they don't request a new handle and instead (accidentally) reuse the old one. By annotating with `#[should_move]`, the user is correctly notified that the handle *should* have been moved by `manipulate_state`, and that they shouldn't just be reusing the old handle.

As a shorthand, `#[should_move]` can be applied to functions which have a `self` (note: not `&mut self` or `&self`) argument and it applies to that argument:

```rust
// equivalent version
impl Handle<'_> {
    #[should_move]
    fn manipulate_state(self) -> StateResult<Self> {
        // ...
    }
}
```

Note that `#[should_move]` will "poison" all possible copies of a value, and can't be fooled by simple tricks:

```rust
let handle = request_handle();
let copy = handle;
if let Err(err) = manipulate_state(handle) {
    eprintln!("oh no! {err}");
}
do_something_else(copy);
```

In this case, Rust knows that `copy` is still a copy of the original `handle`, and *any* copies in the presence of a `#[should_move]` trigger the lint. In order to get rid of the lint, the code must explicitly copy the value, using either `clone()` or this nonsense:

```rust
let handle = request_handle();
let clone = handle.clone(); // reasonable
let copy = *&handle; // if for some reason you hate typing
```

The reason why `&T` arguments aren't allowed to be marked as `#[should_use]` is because these fixes no longer work for them: calling `.clone()` on a reference will clone the value, not the reference itself, and that can be confusing. Plus, references were *made* for copying, and once you've got that floppy you can't stop copy.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

`#[should_move]` can be applied to arbitrary function arguments and applies to those arguments. If `#[should_move]` is applied to a function which takes a `self` value argument, it's automatically applied to just the `self` argument. Doing both is weird and redundant, but should just trigger a lint instead of an error.

Whenever a value is passed for one of these marked arguments, that value and any of its past, present, or future copies are marked as having move semantics, and the borrow checker should emit a (non-fatal) lint if any copies occur. Any borrows to the value are excluded from this checking, and copies from these references are allowed as usual. An automatically applicable fix can suggest annotating these copies with `.clone()` to silence the lint.

`#[should_move]` should error if used on an argument with type `&T`, and that includes `&self`. `self` arguments which are not references are still allowed (e.g. `Pin<Self>`) since these can still be passed by-value.

# Drawbacks
[drawbacks]: #drawbacks

It's a very weird mechanism to explain, but that hasn't stopped us yet.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The primary alternative is to *not* allow this, although the other alternative is to instead have a custom syntax for arguments instead of an attribute:

```rust
fn manipulate_state(move handle: Handle<'_>) -> StateResult<Handle<'_>> {
    // ...
}
```

The primary reasoning for not doing this is that `#[should_move]` is ultimately a lint and doesn't actually change the semantics of the function call, and giving it a proper keyword would indicate otherwise. Plus, *maybe* deref patterns might use that eventually. I'm not psychic.

This would be the first *commonly used* argument attribute, and that does incur some learning penalty for users, but that's not really that big of a problem.

# Prior art
[prior-art]: #prior-art

There really isn't much prior discussion here that isn't already mentioned. Clippy's `copy_iterator` lint is currently marked as allow-by-default, and not much discussion on revisiting `Copy` for iterators has been had since before 1.0.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

* Should this be allowed for non-`Copy` types as a no-op? (After all, these types *should* be moved, and they also *must* be moved.)
* How should this work for generic types? This also plays into the above, since simply allowing it (but emitting a warning) for non-`Copy` types means that generic types that are *maybe* copyable work very naturally with it.

# Future possibilities
[future-possibilities]: #future-possibilities

I hope that `Copy` doesn't become so complicated in the future that we have to extend this. Please don't.
