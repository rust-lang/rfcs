- Feature Name: `move_expressions`
- Start Date: 2026-05-14
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#155050](https://github.com/rust-lang/rust/issues/155050)

## Summary
[summary]: #summary

Add `move($expr)` syntax inside closures, async blocks, and generators. A `move($expr)` evaluates the expression at closure-creation time and captures the result by value. This gives precise control over what a closure captures and when, without needing temporary variables outside the closure.

## Motivation
[motivation]: #motivation

Closures and futures in Rust use an automatic capture mechanism that detects the places that the closure/future body uses from the surrounding environment and decides automatically how to capture them (e.g., by reference, by value). However, it is frequently useful to capture values in other ways; one common example is wishing to capture the *clone* of a value. Unfortunately Rust's current closure syntax provides no ergonomic way to manage this.

A common pattern for capturing clones, drawn from [a blog post by the Dioxus team][dioxus-blog], is to introduce "dummy variables" into the surrounding scope:

```rust
let _some_a = self.some_a.clone();
let _some_b = self.some_b.clone();
let _some_c = self.some_c.clone();
tokio::task::spawn(async move {
    do_something_else_with(_some_a, _some_b, _some_c)
});
```

[dioxus-blog]: https://dioxus.notion.site/Dioxus-Labs-High-level-Rust-5fe1f1c9c8334815ad488410d948f05e

The problems with this pattern:

- The temporary variables (`_some_a`, etc.) exist only to shuttle data into the closure. They add noise and separate the clone from its point of use.
- The `async move` captures *everything* by move, which means later code can't use `self.some_a` even though the intent was only to move the clones.
- When closures are nested or have many captures, the pre-closure variable block grows unwieldy and obscures the actual logic.

Another option is to introduce let variables into a block:

```rust
tokio::task::spawn({
    let some_a = self.some_a.clone();
    let some_b = self.some_b.clone();
    let some_c = self.some_c.clone();
    async move {
        do_something_else_with(some_a, some_b, some_c)
    }
});
```

This pattern at least avoids the need to introduce "dummy names" like `_some_a`, but it still has many of the same downsides.

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

We propose to introduce a `move($expr)` expressions that can appear in a closure or a future. The expression `$expr` will execute when the closure is created and then moved into a temporary captured by the closure; `move($expr)` is then replaced with a place expression consisting of this temporary. This can be used to capture a clone of a value (`move(vec.clone())`) but also other derived values (e.g., `move(vec.len())`).

### `move($expr)` gives explicit control over captures

Within a closure, async block, or generator, you can write `move($expr)` to evaluate an expression at closure-creation time and capture the result by value:

```rust
|| {
    move(some_vec).push(22)
}
```

This moves `some_vec` into the closure, regardless of how it's used in the body. The expression inside `move(...)` is evaluated when the closure is *created*, not when it's *called*.

Any expression is valid inside `move(...)`:

```rust
|| {
    // Move a clone
    move(data.clone()).process()
}

|| {
    // Move a reference
    let len = move(&big_vec).len();
}

|| {
    // Move the result of a function call
    move(compute_config()).apply()
}
```

### A common pattern: listing captures at the top

When you want to be fully explicit about what a closure captures, you can list all captures at the top of the body:

```rust
|| {
    let vec = move(input.vec);
    let data = move(&cx.data);
    let mut output_tx = move(output_tx);

    process(&vec, &mut output_tx, data)
}
```

### `move ||` as shorthand

The `move` keyword on a closure changes the default so that every captured variable is moved in. You can combine this with `move($expr)` for fine-grained control:

```rust
move || {
    process(&input.vec, &mut output_tx, move(&cx.data))
    //       ---------       ---------       --------
    //           |               |               |
    //       moved in by     moved in by    only a reference
    //       `move ||`       `move ||`      is moved in
}
```

Here `move(&cx.data)` creates a reference to `cx.data` at closure-creation time and captures that reference. The closure borrows `cx.data` even though it's a `move` closure.

### Async blocks and generators

`move($expr)` works the same way in async blocks and generators:

```rust
tokio::task::spawn(async {
    send_data(move(tx.clone())).await;
});
```

### Closures called multiple times (`FnMut`, `Fn`)

When a closure is called multiple times, the captured value persists across calls. A `move($expr)` captures once at creation time:

```rust
data_source_iter
    .inspect(|item| {
        inspect_item(item, move(tx.clone()).clone())
        //                      ----------  -------
        //                           |         |
        //                   clone tx once     |
        //                   at creation       |
        //                                     |
        //                             clone the captured
        //                             value on each call
    })
    .collect();

// `tx` is still usable here
```

### Common patterns with cloning

Cloning a value into a closure is one of the most common uses of `move($expr)`. The right pattern depends on how the closure uses the cloned value and how many times the closure is called.

**`move(x.clone())` — clone once, consume once (FnOnce)**

When the closure is called exactly once and consumes the value, clone directly into the capture:

```rust
let handle = tokio::spawn(async {
    channel.send(move(tx.clone())).await;
});
// tx still usable here
```

**`&move(x.clone())` — clone once, borrow on each call (Fn / FnMut)**

When the closure is called multiple times but only needs a reference to the cloned value, take a reference to the capture. This avoids consuming the captured clone on the first call:

```rust
let f: Box<dyn Fn()> = Box::new(|| {
    let data = &move(config.clone());
    data.validate();
    data.report();
});
f(); // works
f(); // works again — data is borrowed, not consumed
```

**`move(x.clone()).clone()` — clone once at creation, clone again per call (Fn / FnMut)**

When the closure is called multiple times and each call needs an owned value, clone the captured value on each invocation:

```rust
fn processed_items(
    shared_config: &Rc<SharedConfig>,
    items: Vec<Item>,
) -> impl Iterator<Item = ProcessedItem> + 'static {
    data_source_iter
        .map(|item| {
            process(item, move(shared_config.clone()).clone())
            //            --------------------------- -------
            //                      |                     |
            //            clone shared_config once        |
            //            when the closure is created     |
            //                                            |
            //                                  clone from the captured
            //                                  value on each call
        })
}
```

This is useful when you need a fresh owned copy per call but wish to have the closure own the value (in this example, the closure needs to own the `Rc<SharedConfig>` so it can satisfy the `'static` bound).

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Syntax

A `move` expression has the form:

```
move ( <expr> )
```

It is valid only inside the body of a closure, async block, or generator. Using `move(...)` outside these contexts is a compile error.

### Desugaring

A `move($expr)` desugars into a fresh temporary variable that is:
1. Bound to the value of `$expr`, evaluated at closure-creation time
2. Always captured by value (moved into the closure), regardless of how it is used in the closure body

Concretely:

```rust
|| foo(move(expr))
```

desugars to the equivalent of:

```rust
{
    let tmp = expr;
    move_capture(tmp) || foo(tmp)
}
```

where `move_capture(tmp)` means `tmp` is captured by move even if the closure body only borrows it.

The temporary is not literally introduced in surface-level Rust. It exists in the compiler's intermediate representation. Its type is inferred from `$expr`.

### Evaluation order

When a closure contains multiple `move(...)` expressions, the temporaries are evaluated in source order (left-to-right, top-to-bottom) at closure-creation time:

```rust
|| f(move(a()), move(b()))
```

desugars as:

```rust
{
    let tmp_1 = a();
    let tmp_2 = b();
    move_capture(tmp_1, tmp_2) || f(tmp_1, tmp_2)
}
```

`a()` is evaluated before `b()`.

### Interaction with `move ||` closures

In a `move ||` closure, all captured variables are moved by default. A `move($expr)` within such a closure still introduces a temporary that is captured by move, but because the expression can be anything (including a reference) it allows capturing *less* than full ownership:

```rust
move || foo(move(&x))
```

desugars as:

```rust
{
    let tmp = &x;
    move_capture(tmp) || foo(tmp)
}
```

The closure captures `tmp` (a reference) by move rather than capturing `x` by move. The closure borrows `x`.

### Nested closures

Each `move($expr)` is associated with its immediately enclosing closure. In nested closures, each `move(...)` creates a temporary for its own level:

```rust
|| {
    || {
        move(move(v.clone())).len()
    }
}
```

desugars as:

```rust
{
    let tmp_outer = v.clone();
    move_capture(tmp_outer) || {
        let tmp_inner = tmp_outer;
        move_capture(tmp_inner) || {
            tmp_inner.len()
        }
    }
}
```

The outer `move(...)` captures `v.clone()` into the outer closure. The inner `move(...)` then moves that value from the outer closure into the inner closure.

### Drop semantics

The temporary introduced by `move($expr)` is dropped when the closure is dropped. If evaluation of the expression panics, the panic propagates at closure-creation time and the closure is never created. Already-evaluated temporaries from prior `move(...)` expressions in the same closure are dropped in reverse order, following standard drop semantics.

### Type inference

The type of a `move($expr)` expression within the closure body is the type of `$expr`. Type inference works as normal. The temporary's type is inferred from the expression, and the use sites within the closure see that type.

### Closures that are never called

If a closure is created but never called, the captured temporaries are still dropped when the closure is dropped. This is the same behavior as any other captured value.

### Parsing

`move` is already a keyword in Rust. The token sequence `move(` is unambiguous: since `move` is not a valid identifier, it cannot be a function call, and since it is not followed by `||` or `|`, it is not a closure.

A move expression is an _ExpressionWithoutBlock_. Its grammar is:

> _MoveExpression_ :\
> &nbsp;&nbsp; `move` `(` _Expression_ `)`

The parser accepts `move($expr)` in any expression position. The restriction to closure, async block, and generator bodies is enforced as a semantic check, not a grammar restriction.

Because `move($expr)` uses parentheses and is an _ExpressionWithoutBlock_, there is no ambiguity with closures: `move(x) || y` always parses as the binary logical-OR of `move(x)` and `y`.

For example, the following is a semantic error because `move(x)` appears outside a closure:

```rust
fn main() {
    let y = move(x) || 22; // ERROR: `move(...)` outside of a closure
}
```

And `move($expr)` is parsed as a single expression, so it matches `$x:expr` in macros:

```rust
macro_rules! parse_as_expr {
    ($x:expr) => { 0 }
}

fn main() {
    println!("{}", parse_as_expr!(move(x))); // prints 0
}
```

### Feature gate

This feature is gated behind `#![feature(move_expressions)]`. No edition change is required since `move` is already a keyword in all editions.

## Drawbacks
[drawbacks]: #drawbacks

### Another way to spell the same thing

Rust already has `move ||` closures and the let-before-closure pattern. Adding `move($expr)` is a third mechanism for the same underlying operation. Users must learn when to use which.

### Readability of `move(...)` as an expression

`move($expr)` looks like a function call but has different evaluation semantics: the expression runs at closure-creation time, not at call time. This is a new concept for readers to internalize. However, the `move` keyword already exists and already means "take ownership," so the meaning is consistent if not immediately obvious.

### Complexity in nested cases

Nested `move(move(...))` requires understanding which closure level each `move` targets. In practice this is uncommon, but the mental model requires understanding the "each `move` targets its immediately enclosing closure" rule.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Why not fully automatic reference counting?

This RFC emerged as part of a larger discussion in how to make reference counting more ergonomic. The original goal of that effort was to address the frequest request from higher-level Rust frameworks to have "fully automatic" reference counting, where users do not need to do anything explicit to `clone` a ref-counted value. This RFC is instead targeting a more modest goal, but one which we believe is also important: making fully explicit clones more ergonomic.

The rationale is this: there are many Rust applications where explicit ref-counting is desirable (e.g., in kernel development or when developing a widely used, perf sensitive library or concurrent data structure). Moreover, even in higher-level applications, there are times when knowing exactly how many references are outstanding is important, such as when working with channels (where dropping the last `tx` handle causes a `rx` handle to return `None`, and hence can be important for correctness) or when working with large ref-counted data structures that consume a lot of memory. Fully automatic reference counting makes some Rust applications much cleaner but does nothing for these scenarios. This RFC, in contrast, helps all Rust users, even if it doesn't go as far as higher-level applications would like. Nothing in the RFC precludes adding fully automatic reference counting as future work.

### Why this design over explicit capture clauses?

[Explicit capture clauses][ecc] (e.g., `move(a.b.c.clone(), ..) || ...`) have been discussed for years. They front-load all capture decisions at the closure head, which helps in some cases but creates problems:

- The capture list grows linearly with captures, duplicating information that's often obvious from the body.
- They introduce a new syntactic position with its own grammar, including place remapping (`a.b.c = expr`) and open-ended captures (`..`).
- For short closures, the overhead of the capture list dominates the closure itself.
- They don't integrate naturally with the "clone then capture" pattern when you want the clone *at the point of use* rather than at the closure head.

Move expressions solve the same problems with less syntax: you write `move($expr)` at the point of use, which is both the declaration and the consumption.

[ecc]: https://smallcultfollowing.com/babysteps/blog/2025/10/22/explicit-capture-clauses/

### Why not RFC #3680's `.use` / `use ||`?

RFC #3680 proposed a `Use` trait and `use ||` closures that automatically "used" captures, with the goal of having a "DWIM" closure where it is not required to explicitly clone each value you use. This is a worthy goal but it is not the goal pursued by this RFC.  We chose not to pursue this goal first because there are substantial use cases that do not want it. This includes low-level Rust, but also, even in higher-level frameworks, cases like a `Sender` on a channel or a `Bytes` buffer, where knowing exactly how many clones exist can be important for avoiding deadlocks or controlling memory usage. Therefore, we are first targeting the version where all clones are explicit. This does not prevent adding a more ergonomic, implicit option later (such as `.use` or `use ||` closures) to handle the remaining cases more succinctly.

### Why not postfix syntax (`expr.move`)?

A postfix operator would look like `expr.move` or `$expr.move`. The problem is that `move` doesn't operate on the final value of the expression; it changes *when* the entire expression is evaluated. Consider:

```rust
|| process(foo(bar()).move)
```

When does `bar()` execute? It must be at closure-creation time, but the postfix position suggests it's part of the normal call-time evaluation. A prefix `move(...)` makes the scope of early evaluation explicit.

### What happens if we don't do this?

The clone-before-closure pattern remains the only option. This is verbose but functional. The cost is friction for a common pattern, particularly in async code.

### What is the fully desugaring semantics, canonical form for closures?

As a general rule, it is desirable to have a "fully desugared, most general" form of each piece of Rust syntax. For closures, the "fully general form" is to convert a closure into a move closure that only references new temporaries introduced explicitly to be desugared (determining exactly what is captured into those temporaries is non-trivial and require analyzing the body of the closure to see what places it uses and how). The canonical closure form does not change with this RFC, it is simply extended so that `move` expressions are another way to get one of those temporary values:

```rust
// Example 1:
|| foo(&vec)
// equivalent to:
{
    let tmp = &vec;
    move || foo(tmp)
}

// Example 2:
move || foo(&vec)
// equivalent to:
{
    move || foo(&vec)
}

// Example 3:
|| foo(&vec, move(bar.clone()))
// equivalent to:
{
    let tmp0 = &vec;
    let tmp1 = bar.clone();
    move || foo(tmp0, tmp1)
}
```

### Does `move($expr)` change evaluation order away from source order?

Arguably not more so than closures already do. The body of a closure is already deferred — none of it executes at the point where the closure expression appears in the source. A `move($expr)` extracts a sub-expression from that deferred body and evaluates it at the point where the closure is created, which *is* source order for the enclosing scope. This is exactly what the let-before-closure pattern does today:

```rust
// Today: `expr` evaluates here, in source order
let tmp = expr;
|| use(tmp)

// With this RFC: same semantics, just written inline
|| use(move(expr))
```

In both cases, `expr` executes at the closure-creation site. The `move($expr)` form simply avoids introducing a named temporary in the enclosing scope.

### If I write `move(x.clone())` twice, will `x` be cloned twice?

Yes. Each `move($expr)` is independently evaluated at closure-creation time:

```rust
|| {
    move(x.clone()).method1();
    move(x.clone()).method2(); // x is cloned a second time
}
```

If you want to clone once and use the result multiple times, bind it with `let`:

```rust
|| {
    let x = &move(x.clone());
    x.method1();
    x.method2();
}
```

Note the `&` — in a non-`FnOnce` closure, you typically don't want each call to take ownership of the captured clone. Taking a reference lets you use it repeatedly. Alternatively, if you need owned values on each call, you can clone the captured value:

```rust
|| {
    move(x.clone()).clone().consume();
    //   ----------  ------
    //       |          |
    //  cloned once    cloned from the
    //  at creation    captured value
    //                 on each call
}
```

### Isn't there a kind of "cliff" going from a cloned variable used once to a cloned variable used twice?

Somewhat. Under this design, if you have

```rust
|| {
    move(foobar.clone()).method();
}
```

and then you with to invoke two methods, you likely want to change to

```rust
|| {
    let foobar = move(foobar.clone());
    foobar.method();
    foobar.method2();
}
```

which means you wind up writing `foobar` a total of 4 times. We consider this acceptable if not necessarily *optimal*. It does have the upside that the execution semantics are quite clear.

## Prior art
[prior-art]: #prior-art

### C++ lambda init-captures (C++14)

C++14 added generalized lambda captures with initializers:

```cpp
auto closure = [x = std::move(some_vec)] {
    x.size();
};
```

This allows arbitrary expressions in the capture list, evaluated at lambda-creation time. The captured variable `x` is then available in the lambda body.

The design serves the same purpose as `move($expr)`: evaluating an expression at closure-creation time and binding it into the closure. The difference is that C++ places this in a capture list (front-loaded), while Rust's `move($expr)` is inline at the point of use. C++'s approach requires naming the captured variable explicitly in the capture list, which adds verbosity when the variable is used only once.

### Earlier community proposals

This idea has been discussed multiple times in the Rust community:

- [Zachary Harrold proposed it on Zulip][z1] using the `super` keyword, and created a proc-macro prototype called [soupa](https://crates.io/crates/soupa).
- [@simulacrum proposed using `move`][z2] instead of `super`, which better aligns with Rust's existing terminology.

[z1]: https://rust-lang.zulipchat.com/#narrow/channel/410673-t-lang.2Fmeetings/topic/Design.20meeting.202025-08-27.3A.20Ergonomic.20RC/near/555236763
[z2]: https://rust-lang.zulipchat.com/#narrow/channel/410673-t-lang.2Fmeetings/topic/Design.20meeting.202025-08-27.3A.20Ergonomic.20RC/near/555643180

## Unresolved questions
[unresolved-questions]: #unresolved-questions

### Should there be a syntax for moving specific variables?

The compiler's HIR desugaring represents "this variable is captured by move" directly. An explicit syntax like `move[x, y] || ...` would let users write:

```rust
move[x, y] || {
    x.process();
    y.send();
    z.borrow(); // z captured by ref
}
```

On one hand, this is a natural complement to `move($expr)`, letting you move specific variables without a full `move ||`. On the other hand, any closure can already be written using `move($expr)` at each use site, making this redundant:

```rust
|| {
    move(x).process();
    move(y).send();
    z.borrow();
}
```

The tradeoff is conciseness vs. having a single mechanism. This question can be resolved during stabilization.

## Future possibilities
[future-possibilities]: #future-possibilities

### Diagnostic suggestions

The compiler could suggest `move($expr)` when it detects the let-clone-before-move-closure pattern:

```
help: consider using a move expression
  |
- let tx_clone = tx.clone();
- tokio::spawn(async move { send(tx_clone).await });
+ tokio::spawn(async { send(move(tx.clone())).await });
```

This is left to implementation discretion rather than specified by this RFC.

### Integration with a `Share` trait

A future `Share` trait identifying types where cloning creates an alias (rather than an independent copy) would compose naturally with move expressions:

```rust
tokio::spawn(async {
    send(move(tx.share())).await;
});
```

This is out of scope for this RFC.


### `move($expr)` outside closures

A future extension could allow `move($expr)` in non-closure contexts as a no-op identity expression, making it possible to write generic code that works both inside and outside closures. Whether this is desirable is left to future discussion.
