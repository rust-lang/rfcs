- Feature Name: `iter_macro`
- Start Date: 2025-09-22
- RFC PR: [rust-lang/rfcs#3861](https://github.com/rust-lang/rfcs/pull/3861)
- Rust Issue: [rust-lang/rust#142269](https://github.com/rust-lang/rust/issues/142269)

# Summary
[summary]: #summary

Add an `iter!` macro to provide a better way to create iterators.

# Motivation
[motivation]: #motivation

Implementing the `Iterator` trait directly can be tedious.
Generators (see [RFC 3513]) are available on nightly but there are enough open design questions that generators are unlikely to be stabilized in the foreseeable future.

On the other hand, we have an `iter!` macro available on nightly that provides a subset of the full generator functionality. Stabilizing this version now would have several benefits:

- Users immediately gain a more ergonomic way to write many iterators.
- We can better inform the design of generators by getting more real-world usage experience.
- Saves the `gen { ... }` and `gen || { ... }` syntax for a more complete feature in the future.

We note that other features have followed a similar progression.
For example, the `try!` macro became `?` and `.await` began life as `await!`.
However, in this case, we believe that even with full generator support, there will still be value in using the `iter!` macro for cases where the full power of generators is not needed.

[RFC 3513]: https://rust-lang.github.io/rfcs/3513-gen-blocks.html

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `iter!` macro is used to create *iterator closures*. These are closures that create iterators.

```rust 
// (import the macro; this is assumed in subsequent examples too)
use std::iter::iter;

let empty = iter!(|| {});
```

When we call `empty`, we get something we can iterator over.

```rust
for () in empty() {
    println!("it wasn't empty after all");
}
```

Of course, the body of this `for` loop never runs. The reason is that our iterator doesn't *yield* any values.

Let's change this.

```rust 
let count_to_three = iter!(|| {
    1.yield;
    2.yield;
    3.yield;
})

for i in count_to_three() {
    println!("{i}");
}
```

This program would print 1, 2, 3, each on a new line.

Iterator closures can also take arguments.

```rust 
let once = iter!(|item| item.yield);

for item in once(5) {
    println!("This only should happen once: {item}");
}
```

We can make more complicated iterators. For example, we can include loops.

```rust 
let count_to_n = iter!(|n| {
    for i in 1..=n {
        n.yield
    }
});
```

## Limitations

While you can use most Rust code within an iterator closure, there are some things to watch out for around borrowing.

The first is that you cannot yield references to the iterator's stack.
Doing so would make this a *lending iterator* which is not yet supported.
For example, the following is not allowed:

```rust
iter!(|| {
    let mut number = 0;
    (&mut number).yield;
    //^ ERROR yields a value referencing data owned by the current function

    println!("Number is now {number}");
});
```

Similarly, iterator closures cannot hold a reference to their stack over a yield point.
One common case where this scenario can arise is with a `Mutex` or `RefCell`:

```rust
iter!(|counter: Rc<RefCell<i32>>| {
    // This block is okay because there are no yields
    // while `counter_ref` is live.
    {
        let mut counter_ref = counter.borrow_mut();
        *counter_ref += 1;
    }

    // This is not okay because `counter_ref` lives
    // across a yield point.
    {
        let mut counter_ref = counter.borrow_mut();
        //^ ERROR borrow may still be in use when `gen` closure body yields
        *counter_ref += 1;
        ().yield;
        //^ possible yield occurs here
        *counter_ref -= 1;
    }
});
```

Finally, iterator closures should not be passed as arguments.
The reason is that although `iter!` creates an iterator closure, Rust has not specified how to name the type of an iterator closure.
This means there is not an appropriate type annotation to add to a function.
We recommend instead passing the result of calling the iterator closure as an `impl IntoIterator` instead.
For example:

```rust
fn takes_iterator(it: impl IntoIterator<Item = i32>) { ... }

let it = iter!(|| { ... });

takes_iterator(it()); // Instead of `takes_iterator(it)`
```

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

The `iter!` macro is defined in `core::iter::iter!` and re-exported as `std::iter::iter!`.

The `iter!` macro's body is parsed as a [closure expression](https://doc.rust-lang.org/stable/reference/expressions/closure-expr.html#closure-expressions):

    IterClosureExpr ->
        iter!(ClosureExpr)

It is an error to attempt to make an async iterator closure, such as `iter!(async || { ... })`.

The `iter!` macro creates an *iterator closure*.
Calling an iterator closure does no work, but instead evaluates to an object that implements the `Iterator` trait that corresponds to the computation of the body of the closure.

Example:

```rust
let make_iter = iter!(|| {
    1.yield;
    2.yield;
});
let mut it = make_iter();
assert_eq!(it.next(), Some(1));
assert_eq!(it.next(), Some(2));
assert_eq!(it.next(), None);
```

Note that iterator closure expressions can be directly called simply by adding parentheses after the expression, with no need to wrap the whole iterator expression in parentheses.
In other words, `iter!(|| {})()` is valid.

Within an iterator closure body, `yield` expressions are allowed.
These have the following syntax:

    YieldExpr ->
        Expr.yield

Yield expressions can be written as `foo.yield`.
A `yield` expression always has type `()`.

When evaluating a `yield`, the iterator suspends execution and passes the value of its argument to the iterating context (i.e. the caller of `next()`).
For example, for an iterator `iter` when executing a call to `iter.next()`, if the execution of the body proceeds to a `yield 3` expression, then `iter.next()` will return `Some(3)`.
The subsequent call to `iter.next()` will resume executing the iterator immediately after that `yield` expression.

When the iterator exits the closure's body, such as by executing to the end of the body or by executing a `return` expression, the corresponding call to `next()` returns `None`, indicating the end of the iterator.

The body of an iterator closure must have a return value compatible with the `()`.
In other words, it must either return `()` or never return.

Auto traits such as `Send` and `Sync` are computed separately for the iterator closure and the iterator returned by the closure.
For iterator closures, these are computed the same as for any other closure.
For the corresponding iterator, auto traits are computed based on the values that must be saved across a `yield` point, similar to how auto traits for an `async` block are computed based on the values that are saved across `await` points.

It is not possible to hold a borrow of a value local to the iterator across a `yield` point.
Similarly, it is not possible to `yield` a reference to an iterator-local value.
Note that holding and yielding references that completely outlive the iterator closure (such as a reference captured from the environment) are allowed.

# Drawbacks

- Iterator closures have some limitations around self-borrows and lending.
- The macro-based syntax looks less built-in than if it were actually built-in.
- Does not exactly support the plain iterator block use case. You always have to create and invoke a closure.

# Rationale and alternatives

This incremental step forward will help us make continued progress towards more advanced generators.
If we do not take this step, we expect we will see little further development in generators and users will remain frustrated that they do not have convenient syntax for creating iterators.
This is a feature that is often missed by Rust programmers.

## Why Iterator Closures?

It might seem more obvious to have `iter!` evaluate directly to an `impl Iterator` with no intermediate closure step.
We instead recommend returning an iterator closure.
This is largely as a result of what we have learned from our experience with `async`.

Having a two step process between creating the iterator-like object and beginning iteration allows us to support scenarios such as where the result of `iter!` is `Send` but the iterator is no longer `Send` once iteration starts.
See Yosh Wuyts' [The Gen Auto-Trait Problem] for more details.
In `async`, we've had a lot of discussion about using `IntoFuture` for this two stage process but decided that it is better represented through async closures.
For iterators and generators, we'd like to set the same precedent from the beginning.

[The Gen Auto-Trait Problem]: https://blog.yoshuawuyts.com/gen-auto-trait-problem/

Second, having convenient syntax for creating inline iterators will create an incentive to create more powerful combinators.
With async, people very quickly started writing functions that took arguments with types like `impl FnOnce() -> F where F: Future`.
Despite the clear desire to write this, these never worked particularly well until we had proper support for async closures.
Still, this created an ecosystem hazard, as we wanted what Rust supported to be broadly compatible with how the ecosystem had already been experimenting.
Again, using what we learned from async, we have the chance to do the right thing from the beginning with iterator closures.

As an example, below is an example of how iterator closures can create a combinator that does run-length encoding (RLE) on any other iterator:

```rust
fn main() {
    let rl_encode = iter!(|iter| {
        // do run length encoding on the items yielded by iter
        // and yield each value followed by the run length.        
    })

    for x in [1u8; 513].into_iter().then(rl_encode) {
        //   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
        //   ^ Produces an iterator that yields the run-length
        //     encoding of this array.
        println!("{:?}", x);
    }
}
```

[Full worked example in Playground](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2024&code=%2F%2F+This+demonstrates+run-length+encoding+using+gen+blocks+and+how+we%0A%2F%2F+might+use+this+via+a+method+combinator+on+%60Iterator%60.%0A%2F%2F%0A%2F%2F+Author%3A+TC%0A%2F%2F+Date%3A+2024-05-23%0A%0A%2F%2F%40+edition%3A+2024%0A%23%21%5Bfeature%28gen_blocks%29%5D%0Afn+rl_encode%3CI%3A+IntoIterator%3CItem+%3D+u8%3E%3E%28%0A++++xs%3A+I%2C%0A%29+-%3E+impl+Iterator%3CItem+%3D+u8%3E+%7B%0A++++gen+%7B%0A++++++++let+mut+xs+%3D+xs.into_iter%28%29%3B%0A++++++++let+%28Some%28mut+cur%29%2C+mut+n%29+%3D+%28xs.next%28%29%2C+0%29+else+%7B+return+%7D%3B%0A++++++++for+x+in+xs+%7B%0A++++++++++++if+x+%3D%3D+cur+%26%26+n+%3C+u8%3A%3AMAX+%7B%0A++++++++++++++++n+%2B%3D+1%3B%0A++++++++++++%7D+else+%7B%0A++++++++++++++++yield+n%3B+yield+cur%3B%0A++++++++++++++++%28cur%2C+n%29+%3D+%28x%2C+0%29%3B%0A++++++++++++%7D%0A++++++++%7D%0A++++++++yield+n%3B+yield+cur%3B%0A++++%7D.into_iter%28%29%0A%7D%0A%0Atrait+IteratorExt%3A+Iterator+%2B+Sized+%7B%0A++++fn+then%3CF%2C+I%3E%28self%2C+f%3A+F%29+-%3E+impl+Iterator%3CItem+%3D+Self%3A%3AItem%3E%0A++++where%0A++++++++%2F%2F+For+the+same+reasons+that+we+need+async+closures%2C+we%27d%0A++++++++%2F%2F+actually+want+to+use+gen+closures+here+in+a+real%0A++++++++%2F%2F+implementation.%0A++++++++F%3A+FnOnce%28Self%29+-%3E+I%2C%0A++++++++I%3A+IntoIterator%3CItem+%3D+Self%3A%3AItem%3E%2C%0A++++%7B%0A++++++++f%28self%29.into_iter%28%29%0A++++%7D%0A%7D%0A%0Aimpl%3CI%3A+Iterator%3E+IteratorExt+for+I+%7B%7D%0A%0Afn+main%28%29+%7B%0A++++for+x+in+%5B1u8%3B+513%5D.into_iter%28%29.then%28rl_encode%29+%7B%0A++++++++%2F%2F+++%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%0A++++++++%2F%2F+++%5E+Produces+an+iterator+that+yields+the+run-length%0A++++++++%2F%2F+++++encoding+of+this+array.%0A++++++++println%21%28%22%7B%3A%3F%7D%22%2C+x%29%3B%0A++++%7D%0A%7D%0A)

<details>
<summary>Other relevant links</summary>
- https://github.com/rust-lang/libs-team/issues/379
- https://github.com/rust-lang/libs-team/issues/379#issuecomment-2128076515
</details>

With async, `|| async { ... }` did not work particularly well because the future returned by the closure could not borrow from the closure.
Supporting this requires a generic associated type (GAT) on the function traits, which was the motivation for adding the [`async Fn*` family of traits][async-fn-traits].
Although we could write the previous RLE example using `|iter| iter! { ... }`, we would run into the same problem.

Instead, supporting `iter!(|| ... )` is compatible with a more powerful `iter Fn*` trait family (see [Future Work][iter-fn-traits]), so that without any code changes, all existing `iter!` blocks will continue to work and they will also gain more expressiveness.
What is still missing in the current RFC is the ability to specify `iter Fn*` bounds, for passing iterator closures as arguments, but this is also a strictly additive change.

In short, what we are proposing now is forward compatible with the desired end state, based on the experience from adding async closures.

[async-fn-traits]: https://doc.rust-lang.org/std/ops/trait.AsyncFnMut.html

## Why not Self-Borrows?

In `iter!` we are making the explicit choice not to support self-borrows (at least that are held across await points).
This means we are unable to support certain patterns, such as:

```rust
gen fn iter_set_rc<T: Clone>(xs: Rc<RefCell<HashSet<T>>>) -> T {
    for x in xs.borrow().iter() {
    //       ^^^^^^^^^^^ ERROR borrow may still be in use
        x.clone().yield; // during this yield
    }
}
```

For more such examples, see [Self-borrowing generator examples][self-borrow-examples].

This kind of self-borrow across a suspend *is* supported with async, but that is done by requiring `Pin<&mut Self>` for the future's `poll` function.
Because futures were new, the `Future` trait could be designed with this from the start.
We do not have that luxury with iterators.

Furthermore, there is active experimentation going on around pin ergonomics and alternative ways to model address-sensitive types.
We think it best to let some of these experiments develop further before attempting to support address-sensitive iterators.

See the discussion under [Future Work][self-borrows-future] for more detail about possible approaches to supporting this in the future.

[self-borrow-examples]: https://hackmd.io/DTSOVR4QRLyvaU1HQiQZvg

For more context, see the discussion notes from the [T-lang meeting on Self-Referential Generators](https://hackmd.io/7O9IyhHvRmqaMd-NS6dYyw).

## Why not a Library?

Several crates such as [genawaiter] attempt to provide generators in stable Rust.
They generally do this by combining a future with a mutable cell to pass values that are being yielded.
While these are clever uses of futures, they tend to feel like hacks and thus Rust developers a reluctant to use them for anything serious.
Even as a macro, having iterator blocks built into Rust would let users feel comfortable putting them into production.

As a built-in macro, `iter!` is able to have a cleaner and more efficient implementation by directly constructing internal compiler data structures to expose functionality the compiler already supports.

[genawaiter]: https://github.com/whatisaphone/genawaiter

## Prefix Yield

This RFC is written to allow `yield` in postfix position (`foo.yield`).
This is a departure from most programming languages, but it is inline with other Rust postfix operators such as `?` or `.await`.

This design decision is primarily forward looking.
We would like to support full coroutines (see [Future Work][full-coroutines]), at which point `yield` will be able to return a non-`()` value.
In this case, there will be cases where it is convenient to write `foo().yield.do_something()` instead of `(yield foo()).do_something()`;

Rather than choose one syntax now and either change it or accept a suboptimal syntax in the future, we propose to adopt the postfix `yield` syntax from the start.

# Prior art

Generators are a common feature in many popular programming languages, including Python, JavaScript, C#, Kotlin, PHP, Ruby, etc.

As with futures and `async`/`await`, the Rust version of generators or iterator closures will be more complex than in other languages due to the lack of a garbage collector, as well as the need to manage lifetimes and movability.

# Future possibilities

There are some fairly obvious extensions to this functionality that we have deferred in the interest in shipping a minimum viable product.
We discuss these in more detail now.

## `IterFn*` Traits

[iter-fn-traits]: #iterfn-traits

Analogously to the `async Fn*` family of traits, we can gain expressiveness by adding a family of `iter Fn*` traits.
These would allow iterator closures to create iterators that borrow from the closure.

Concretely, `iter Fn*` traits would allow cases like:

```rust
iter!(|x: &i32| x.yield);
```

Adding these traits would largely be a copy-and-paste operation from the `async Fn*` trait family and would reuse much of the machinery.
Still, in considering this, it is tempting to try to design some kind of uber trait family that would support async, iterator, and any future coroutine closures.

For this reason, we are deferring this effort for future work.
Our experience with async closures shows that there is a good migration path from `Fn* -> impl Iterator` closures to `iter Fn*` closures.

## Argument-less Shorthand

While we think it is important to support iterator *closures* from the beginning, we recognize that in many cases users will want an iterator block. We could support that by modifying the `iter!` macro to support syntax like:

```rust
iter!({
    1.yield;
    2.yield;
    3.yield;
})
```

The is some flexibility in the semantics of this form.
The most consistent option is to make it a shorthand for:

```rust
iter!(|| {
    1.yield;
    2.yield;
    3.yield;
})
```

In other words, it's a shorthand for an iterator closure that takes no arguments.

Another option is to have the macro evaluate to an iterator.
In other words, it would be equivalent to:

```rust
iter!(|| {
    1.yield;
    2.yield;
    3.yield;
})()
```

We note that we can achieve a similar effect by adding an `IntoIterator` impl, which we discuss next.

## `IntoIterator` for Thunks

It would be convenient to add a blanket impl or `IntoIterator` for functions of no arguments that return iterators.
After all, calling `f.into_iter()` is isomorphic to `f()`, as both apply a function of no arguments to `f`.
This blanket impl would allow the following, which is arguably clearer to understand:

```rust
let it = iter!(|| {
    1.yield;
    2.yield;
    3.yield;
}).into_iter();
```

But perhaps more importantly, we could pass the result of `iter!` into a `for` loop without an intervening `()`:

```rust
for i in iter!(|| {
    1.yield;
}) {
    println!("{i}");
}
```

We see that as likely a small ergonomic win, but not worth including in the first release.

## Self-Borrows

[self-borrows-future]: #self-borrows

We believe self-borrows across yields are an important feature that needs to be supported, while still making incremental progress by not supporting them now.
There are still a number of large design challenges that need to be resolved first, primarily around migration between and interoperability with existing iterators.
Some possibilities include:

- Adding a (pinned) `Generator` trait and blanket impls or conversions between existing `Iterators`.
- Augmenting `Iterator` with associated traits or bounds (a feature which itself still needs design and implementation work).
- Pinning or address sensitivity as an effect.
- A `Move` trait.

All of these possibilities have tradeoffs or need significant design work to determine the ecosystem impact.
We believe this effort will be aided by the real-world experience that comes from using `iter!` in the wild.

## Lending Iterators

Lending iterators are iterators that return references to their own internal state.
From a technique standpoint, it is possible to support them now using GATs.
The primary challenge to supporting these, then, is the migration story from the existing `Iterator` trait.

There is a lot of overlap between this migration and a possible migration to an iteration trait that supports self-borrows.
Furthermore, we believe there is substantial overlap between lending and self-borrowing use cases, in that iterators that self-borrow are likely to lend and vice-versa.
Therefore, we recommend exploring the combined design space for self-borrows and lending iterators rather than treating them as completely separable.

## `yield_all!`

It's often convenient to be able to `yield` all the items from another iterator.
For example:

```rust
let concat = iter!(|a, b| {
    for i in a {
        i.yield;
    }
    for i in b {
        b.yield;
    }
})
```

We may want to provide a shorthand for this patern, so that the example could be written as:

```rust
let concat = iter!(|a, b| {
    yield_all!(a);
    yield_all!(b);
})
```

This would be a small addition but a nice ergonomic improvement in some cases.

While we could add special syntax or new keywords to do this, it seems like a straightforward `macro_rules!` macro would be sufficient.

## Full Coroutines

[full-coroutines]: #full-coroutines

A more powerful form of iterators or generators is a *coroutine*.
The primary capability coroutines gain over iterators is the ability to provide resume arguments that are returned from `yield`.

Although not visible in the surface syntax, this is used in the desugaring of `await`, which includes something like:

```rust
task_context = ().yield;
```

Then when an executor calls `poll` on a future, as in `fut.poll(cx)`, the value of `cx` because the value of the `yield` and therefore assigned to `task_context`.

Given that rustc already supports this functionality internally, it would be useful to expose this to Rust users directly.

## Additional Iterator Traits

There are other iteration traits such as `ExactSizeIterator` and `FusedIterator`.
In this RFC, the iterator returned by an iterator closure only implements `Iterator`.
It's possible we could extend this in the future.
Some would be easier than others.
For example, we can probably make all `iter!` iterators implement `FusedIterator` without much trouble (we'd need to make `FusedIterator` a lang item, which it isn't currently).
`ExactSizeIterator` would likely need an annotation and/or analysis of the iterator body to see if it can be proven to iterate a known and fixed number of times.
On the other hand, `DoubleEndedIterator` would need some way to specify the `next_back` method.

Since this is a purely additive change, we recommend considering it at a later time after doing adqeuate exploration.
