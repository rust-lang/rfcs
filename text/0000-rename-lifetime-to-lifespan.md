- Start Date: 2014-11-27
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Rename the term `lifetime` to `lifespan` in Rust's ownership model. The change would mostly effect compiler messages and documentations.

# Motivation

The ownership model is a center piece of the Rust programming language. In it, lifetime is a very important concept yet stumps many newcomers. This can be seen from the amount of related questions asked by newcomers. The difficulties may come from not only the concept itself (i.e. it's new to the reader), but also the term `lifetime` used. An alternative, more accurate term `lifespan` is thus proposed, with reasons listed below.

### Lifetime vs lifespan

Here are the reasons why some difficulties may result from the term `lifetime`, and why the term `lifespan` is better:

1. Lifetime is **temporal**. Usually, a temporal concept is more difficult to comprehend or visualize than a spatial one, because human cannot see the time. Lifespan has a spatial sense (the one-dimensional length), which is easier to understand.

1. Lifetime is tied to **runtime and object lifetime** in [many languages](http://en.wikipedia.org/wiki/Object_lifetime). We are arguably overloading the term. For example, [C++ object lifetime](http://en.cppreference.com/w/cpp/language/lifetime) is a *runtime* property, but in Rust, the lifetime of a reference pointer is a static, purely compile-time thing irrelevant to program execution. Furthermore, the lifetime in Rust's ownership model refers to the span of the validity of the reference pointer, not the *object lifetime* of an object.

1. Lifetime is often related to **ongoing, dynamic or non-deterministic** things. For example, we say “in my lifetime” or “once in a lifetime” but not “my lifespan”, because “lifespan” is a static measurement. (Hence it makes sense that other languages use lifetime for runtime properties, as mentioned in point #2.)

### Example

These sample snippets from the [Lifetimes Guide](http://doc.rust-lang.org/guide-lifetimes.html) read more naturally with `lifespan`: (try to compare it with `lifetime` in your head)

> A lifespan is a static approximation of the span of execution during which the pointer is valid:

or

> Here the identifier `r` names the lifespan of the pointer explicitly. So in effect, this function declares that it takes a pointer with lifespan `r` and returns a pointer with that same lifespan.

### Related usage

Due to the choice of the term, we often describe `lifetime` as if it were an ongoing thing at runtime, which adds confusion. For example, with this [code](http://is.gd/GoBBdb):

    main() {
        let a = Some(&0i);
    }

We got an error:

    <anon>:2:19: 2:21 error: borrowed value does not live long enough
    <anon>:2     let a = Some(&0i);
                               ^~
    <anon>:1:11: 3:2 note: reference must be valid for the block at 1:10...
    <anon>:1 fn main() {
    <anon>:2     let a = Some(&0i);
    <anon>:3 }
    <anon>:2:5: 2:22 note: ...but borrowed value is only valid for the statement at 2:4
    <anon>:2     let a = Some(&0i);
                 ^~~~~~~~~~~~~~~~~
    <anon>:2:5: 2:22 help: consider using a `let` binding to increase its lifetime
    <anon>:2     let a = Some(&0i);
                 ^~~~~~~~~~~~~~~~~

If we use `lifespan` and related terms, the error message could become:

    <anon>:2:19: 2:21 error: the life of borrowed value does not span long enough
    <anon>:2     let a = Some(&0i);
                               ^~
    ...
    <anon>:2:5: 2:22 help: consider using a `let` binding to increase its lifespan
    <anon>:2     let a = Some(&0i);
                 ^~~~~~~~~~~~~~~~~

Similarly, an error message like

    captured variable `x` does not outlive the enclosing closure

can be rephrased as

    the lifespan of captured variable `x` does not exceed the enclosing closure

or

    the life of captured variable `x` does not span the enclosing closure


to turn its dynamic charasteristic into a more static, accurate one.

# Detailed design

1. Rename `lifetime` to `lifespan` (and other terms such as `live` or `outlive` to `span`) for Rust compiler, mainly the compiler messages.
1. Rename `lifetime` to `lifespan` in documentations.

# Drawbacks

* The term `lifetime` has been widely used and accepted by the Rust community. It even created some identity.
* “Lifespan system” sounds less cool. (!)

# Alternatives

* scope ([RFC](https://github.com/rust-lang/rfcs/pull/431))
* lifescope, liverange (from [Reddit](http://www.reddit.com/r/rust/comments/2nfu5r/)); borrow scope, borrow lifetime (from [RFC ](https://github.com/rust-lang/rfcs/pull/431) conversation)

If we do not make the change, we are arguably overloading the term `lifetime` with a purely static, compile-time meaning. Renaming it to `lifespan` should also make the concept (and the language) more accessible.

# Unresolved questions

No.
