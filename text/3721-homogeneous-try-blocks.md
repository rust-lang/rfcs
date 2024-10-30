- Feature Name: `homogeneous_try_blocks`
- Start Date: 2024-02-22
- RFC PR: [rust-lang/rfcs#3721](https://github.com/rust-lang/rfcs/pull/3721)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

Tweak the behaviour of `?` inside `try{}` blocks to not depend on context,
in order to work better with methods and need type annotations less often.

The stable behaviour of `?` when *not* in a `try{}` block is untouched.


# Motivation
[motivation]: #motivation

> I do have some mild other concerns about try block -- in particular it is
> frequently necessary in practice to give hints as to the try of a try-block.
>
> ~ [Niko commenting on #70941](https://github.com/rust-lang/rust/issues/70941#issuecomment-612167041)

---

The desugaring of `val?` currently works as follows, per RFC #3058:

```rust
match Try::branch(val) {
    ControlFlow::Continue(v) => v,
    ControlFlow::Break(r) => return FromResidual::from_residual(r),
}
```

Importantly, that's using a trait to create the return value.
And because the argument of the associated function is a generic on the trait,
it depends on inference to determine the correct type to return.

That works great in functions, because Rust's inference trade-offs mean that
the return type of a function is always specified in full.  Thus the `return`
has complete type context, both to pick the return type as well as,
for `Result`, the exact error type into which to convert the error.

However, once things get more complicated, it stops working as well.  That's even
true before we start adding `try{}` blocks, since closures can hit them too.
(While closures behave like functions in most ways, their return types can be
left for type inference to figure out, and thus might not have full context.)

For example, consider this example of trying to use `Iterator::try_for_each` to
read the `Result`s from the `BufRead::lines` iterator:

```rust
use std::io::{self, BufRead};
pub fn concat_lines(reader: impl BufRead) -> io::Result<String> {
    let mut out = String::new();
    reader.lines().try_for_each(|line| {
        let line = line?; // <-- question mark
        out.push_str(&line);
        Ok(())
    })?; // <-- question mark
    Ok(out)
}
```

<!-- https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=c8171a07eb256392a6e392e0f940b171 -->

Though it looks reasonable, it doesn't compile:

```text
error[E0282]: type annotations needed
 --> src/lib.rs:7:9
  |
7 |         Ok(())
  |         ^^ cannot infer type for type parameter `E` declared on the enum `Result`
  |

error[E0283]: type annotations needed
 --> src/lib.rs:8:7
  |
8 |     })?; // <-- question mark
  |       ^ cannot infer type for type parameter `E`
  |
```

The core of the problem is that there's nothing to constrain the intermediate type
that occurs *between* the two `?`s.  We'd be happy for it to just be the same
`io::Result<_>` as in the other places, but there's nothing saying it *must* be that.
To the compiler, we might want some completely different error type that happens
to support conversion to and from `io::Error`.

The easiest fix here is the annotate the return type of the closure, as follows:

```rust
use std::io::{self, BufRead};
pub fn concat_lines(reader: impl BufRead) -> io::Result<String> {
    let mut out = String::new();
    reader.lines().try_for_each(|line| -> io::Result<()> { // <-- return type
        let line = line?;
        out.push_str(&line);
        Ok(())
    })?;
    Ok(out)
}
```

<!-- https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=a3870699c7df0df06eb720c036d94f0e -->

But it would be nice to have a way to request that "the obvious thing" should happen.

This same kind of problem happens with `try{}` blocks as they were implemented
in nightly at the time of writing of this RFC.  The desugaring of `?` in a `try{}`
block was essentially the same as in a function or closure, differing only in that
it "returns" the value from the block instead of from the enclosing function.

For example, this works great as it the type context available from the return type:

```rust
pub fn adding_a(x: Option<i32>, y: Option<i32>, z: Option<i32>) -> Option<i32> {
    Some(x?.checked_add(y?)?.checked_add(z?)?)
}
```

<!-- https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=0e383fab261dc7a39adf61975a307af3 -->

Suppose, however, that you wanted to do more in the method after the additions,
and thus added a `try{}` block around it:

```rust
#![feature(try_blocks)]
pub fn adding_b(x: Option<i32>, y: Option<i32>, z: Option<i32>) -> i32 {
    try { // pre-RFC version
        x?.checked_add(y?)?.checked_add(z?)?
    }
    .unwrap_or(0)
}
```

<!-- https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=e53dda532b44eecd6d1725143d04f900 -->

That doesn't compile, since a (non-trait) method call required the type be determined:

```text
error[E0282]: type annotations needed
 --> src/lib.rs:3:5
  |
3 | /     try { // pre-RFC version
4 | |         x?.checked_add(y?)?.checked_add(z?)?
5 | |     }
  | |_____^ cannot infer type
  |
  = note: type must be known at this point
```

This is, in a way, more annoying than the `Result` case.  Since at least there,
there's the possibility that one wants the `io::Error` converted into some
`my_special::Error`.  But for `Option`, there's no conversion for `None`.
While it's possible that there's some other type that accepts its residual,
the normal case is definitely that it just stays a `None`.

This RFC proposes using the unannotated `try { ... }` block as the marker to
request a slightly-different `?` desugaring that stays in the same family.

With that, the `adding_b` example just works.  And the earlier `concat_lines`
problem can be solved simply as

```rust
use std::io::{self, BufRead};
pub fn concat_lines(reader: impl BufRead) -> io::Result<String> {
    let mut out = String::new();
    reader.lines().try_for_each(|line| try { // <-- new version of `try`
        let line = line?;
        out.push_str(&line);
    })?;
    Ok(out)
}
```

(Note that this version also removes an `Ok(())`, as was decided in
[#70941](https://github.com/rust-lang/rust/issues/70941).)


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

*Assuming this would go some time after [9.2](https://doc.rust-lang.org/stable/book/ch09-02-recoverable-errors-with-result.html)
in the book, which introduces `Result` and `?` for error handling.*

<!-- https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=24e0f7446f4980fcb1afa0c4b1ab08bf -->

So far all the places we've used `?` it's been fine to just return from the function on an error.  Sometimes, however,
it's nice to do a bunch of fallible operations, but still handle the errors from all of them before leaving the function.

One way to do that is to make a closure an immediately call it (an *IIFE*,
immediately-invoked function expression, to borrow a name from JavaScript):

```rust,edition2021,compile_fail
let pair_result = (||{
    let a = std::fs::read_to_string("hello")?;
    let b = std::fs::read_to_string("world")?;
    Ok((a, b))
})();
```

That's somewhat symbol soup, however.  And even worse, it doesn't actually compile
because it doesn't know what error type to use:
```text
error[E0282]: type annotations needed for `Result<(String, String), E>`
  --> src/lib.rs:28:9
   |
   |     let pair_result = (||{
   |         ----------- consider giving `pair_result` the explicit type `Result<(_, _), E>`, where the type parameter `E` is specified
...
   |         Ok((a, b))
   |         ^^ cannot infer type for type parameter `E` declared on the enum `Result`
```

Why haven't we had this problem before?  Well, when we're writing *functions*
we have to write the return type of the function down explicitly.  The `?` operator
in a function uses that to know to which error type it should convert any error is gets.
But in the closure, the return type is left to be inferred, and there are many possible answers,
so it errors because of the ambiguity.

This can be fixed by using a *try block* instead:

```rust,edition2021
let pair_result = try {
    let a = std::fs::read_to_string("hello")?;
    let b = std::fs::read_to_string("world")?;
    (a, b)
};
```

Here the `?` operator still does essentially the same thing -- either gives the value
from the `Ok` or short-circuits the error from the `Err` -- but with slightly
different details:

- Rather than returning the error from the function, it returns it from the `try` block.
  And thus in this case an error from either `read_to_string` ends up in the `pair_result` local.

- Rather than using the function's return type to decide the error type,
  it keeps using the same family as the type to which the `?` was applied.
  And thus in this case, since `read_to_string` returns `io::Result<String>`,
  it knows to return `io::Result<_>`, which ends up being `io::Result<(String, String)>`.

The trailing expression of the `try` block is automatically wrapped in `Ok(...)`,
so we get to remove that call too.  (Note to RFC readers: this decision is not part of this RFC.
It was previously decided in [#70941](https://github.com/rust-lang/rust/issues/70941).)

This behaviour is what you want in the vast majority of simple cases.  In particular,
it always works for things with just one `?`, so simple things like `try { a? + 1 }`
will do the right thing with minimal syntactic overhead.  It's also common to want
to group a bunch of things with the same error type.  Perhaps it's a bunch of calls
to one library, which all use that library's error type.  Or you want to do
[a bunch of `io` operations](https://github.com/rust-lang/rust/blob/d6f3a4ecb48ead838638e902f2fa4e5f3059779b/compiler/rustc_borrowck/src/nll.rs#L355-L367) which all use `io::Result`.  Additionally, `try` blocks work with
`?`-on-`Option` as well, where error-conversion is never needed, since there is only `None`.

It will fail to compile, however, if not everything shares the same error type.
Suppose we add some formatting operation to the previous example:

```rust,edition2021,compile_fail
let pair_result = try {
    let a = std::fs::read_to_string("hello")?;
    let b = std::fs::read_to_string("world")?;
    let c: i32 = b.parse()?;
    (a, c)
};
```

The compiler won't let us do that:

```text
error[E0308]: mismatched types
  --> src/lib.rs:14:32
   |
   |     let c: i32 = b.parse()?;
   |                           ^ expected struct `std::io::Error`, found struct `ParseIntError`
   = note: expected enum `Result<_, std::io::Error>`
              found enum `Result<_, ParseIntError>`
note: return type inferred to be `Result<_, std::io::Error>` here
  --> src/lib.rs:14:32
   |
   |     let a = std::fs::read_to_string("hello")?;
   |                                             ^
```

For now, the best solution for that mixed-error case is the same as before: to refactor it to a function.

### Common `Option` Patterns

Various languages with `null` have a *null-conditional* operator `?.` that short-circuits if the value to the left is `null`.

Rust, of course, doesn't have `null`, but `None` often serves a similar role.
`try` blocks plus `?` combine to give Rust a `?.` without needing to add it as a special operator.

Suppose you have some types like this:

```rust
struct Foo {
    foo: Option<Bar>,
}

struct Bar {
    bar: Option<i32>,
}
```

where you have an `x: Foo` and want to add one to the innermost number, getting an `Option`.

There's various ways you could do that, such as

```rust
x.foo.and_then(|a| a.bar).map(|b| b + 1)
```

or

```rust
if let Foo { foo: Some(Bar { bar: Some(b) }) } = x {
    Some(b + 1)
} else {
    None
}
```

but with `try` blocks, you simplify that down to

```rust
try { x.foo?.bar? + 1 }
```

<!-- https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=144eed270eba8cfe76a192d362634115 -->

You can also use this for things that don't have dedicated methods on `Option`.

For example, there's an `Option::zip` for going from `Option<A>` and `Option<B>` to `Option<(A, B)>`.
But there's no *three*-argument version of this.

That's ok, though, since you can do that with `try` blocks easily:

```rust
try { (x?, y?, z?) }
```


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

> ⚠️ This section describes a possible implementation that works with today's type system. ⚠️
>
> The core of the RFC is the homogeneity of `try` blocks.  As the author of the RFC,
> I'd be happy with other implementations that maintain the properties of this one.
> If it ended up happening with custom typing rules instead, or something, that would be fine.
> But it's worth emphasizing that it's doable entirely via a desugaring, no new solver features.

## Grammar

No change to the grammar; it stays just

*TryBlockExpression*: `try` *BlockExpression*

## Desugaring

Today on nightly, `x?` *inside a `try` block* desugars as follows, after [RFC 3058]:

[RFC 3058]: https://rust-lang.github.io/rfcs/3058-try-trait-v2.html

```rust
match Try::branch(x) {
    ControlFlow::Continue(v) => v,
    ControlFlow::Break(r) => break 'try FromResidual::from_residual(r),
}
```

Where `'try` means the synthetic label added to the innermost enclosing `try` block.
(The actual label is not something that can be mentioned from user code,
but it's using the same [label-break-value] mechanism that stabilized in 1.65.)

[label-break-value]: https://blog.rust-lang.org/2022/11/03/Rust-1.65.0.html#break-from-labeled-blocks

This RFC changes that desugaring to

```rust
// This is an internal convenience function for the desugar, not something public
fn make_try_type<T, R: Residual<T>>(r: R) -> <R as Residual<T>>::TryType {
    FromResidual::from_residual(r)
}

match Try::branch(x) {
    ControlFlow::Continue(v) => v,
    ControlFlow::Break(r) => break 'try make_try_type(r),
}
```

This still uses `FromResidual::from_residual` to actually create the value,
but determines the type to return from the argument via the `Residual` trait
rather than depending on having sufficient context to infer it.

## The `Residual` trait

This trait [already exists as unstable](https://doc.rust-lang.org/1.82.0/std/ops/trait.Residual.html),
so feel free to read its rustdoc instead of here, if you prefer.  It was added to support APIs like
[`Iterator::try_find`](https://doc.rust-lang.org/1.82.0/std/iter/trait.Iterator.html#method.try_find)
which also need this "I want a `Try` type from the same 'family', but with a different `Output` type" behaviour.

> ⚠️ As the author of this RFC, the details of this trait are not the important part of this RFC. ⚠️
> I propose that, like was done for [RFC 3058], the exact details here be left as an unresolved question
> to be finalized after nightly experimentation.
> In particular, it appears that the [naming and structure related to `try_trait_v2`
> is likely to change](https://github.com/rust-lang/rust/issues/84277#issuecomment-1066120333),
> and thus the `Residual` trait will likely change as part of that.  But for now
> this RFC is written following the names used in the previous RFC.

```rust
pub trait Residual<V> {
    type TryType: ops::Try<Output = V, Residual = Self>;
}
```

### Implementations

```rust
impl<T, E> ops::Residual<T> for Result<convert::Infallible, E> {
    type TryType = Result<T, E>;
}

impl<T> ops::Residual<T> for Option<convert::Infallible> {
    type TryType = Option<T>;
}

impl<B, C> ops::Residual<C> for ControlFlow<B, convert::Infallible> {
    type TryType = ControlFlow<B, C>;
}
```


# Drawbacks
[drawbacks]: #drawbacks

This adds extra nuance to the `?` operator, so one might argue that the extra convenience of homogeneity
is not worth the complexity and that adding type annotations instead is fine.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

## Supporting methods

Today on nightly, with potentially-heterogeneous `try` blocks, this code doesn't work

<!-- https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=6c58af71873885c51e7dc6572f4c96ce -->

```rust
try { slice.get(i)? + slice.get(j)? }.unwrap_or(-1)
```

because method invocation requires that it knows the type, but with a contextual return type from the `try` block that's not available

```
error[E0282]: type annotations needed
 --> src/lib.rs:4:5
  |
4 |     try { slice.get(i)? + slice.get(j)? }.unwrap_or(-1)
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ cannot infer type
```

With the homogeneous `try` blocks in this RFC, however, that works because the type flows "out" from the try block,
rather than "in" from how the block is used.

## Supporting generics

Essentially the same as the previous section, but this doesn't work on nightly either:

<!-- https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=fead74266baf2900be2bf69a61fbe618 -->

```rust
let x = try { slice.get(i)? + slice.get(j)? };
dbg!(x);
```

because `dbg!` accepts any `Debug`gable type and thus here it also doesn't know what type you want

```
error[E0282]: type annotations needed
 --> src/lib.rs:4:9
  |
4 |     let x = try { slice.get(i)? + slice.get(j)? };
  |         ^
5 |     dbg!(x);
  |          - type must be known at this point
  |
help: consider giving `x` an explicit type
  |
4 |     let x: /* Type */ = try { slice.get(i)? + slice.get(j)? };
  |          ++++++++++++
```

Homogeneous `try` fixes this as well.

## The simple case deserves the simple syntax

We could add a new `try homogeneous { ... }` block with this behaviour, and leave `try { ... }` as heterogeneous.

That feels backwards, because heterogeneous try blocks are the ones that most commonly need a *type* annotation of some sort.

If there's `?`s on multiple `Result`s with incompatible error types, we need to tell it *somehow* which type to use.
Maybe we want an `anyhow::Result<_>`, maybe we want our own `Result<_, crate::CustomError>`, whatever.

Thus if they commonly need a type annotation anyway, we can consider in the future (see below for more)
an annotated version of `try` blocks that allow heterogeneity, while leaving the short thing for the simple case.

## Manual error conversion is always possible

Even inside a homogeneous `try` block, you could always *manually* add a call to convert an error.

For example, you could do something like
```rust
try {
    foo()?;
    bar().map_err(Into::into)?;
    qux()?;
}
```

if you need to convert the error type from `bar` to the one used by `foo` and `qux`.

We could always add a specific method to express that intent, though this RFC does not propose one.
Spelling it as `.map_err(into)` might be pretty good already, which would be possible with [RFC#3591].

[RFC#3591]: https://github.com/rust-lang/rfcs/pull/3591

## Other merging approaches

There's a variety of other things we could do if the `?`s don't all match.

- Maybe we try to convert everything to the first one
- Maybe we try to convert everything to the last one
- Maybe we fold them through some type function that attempts to merge residuals

But these are all much less local.

A nice property of the homogeneous `try` block is that you don't have to think about all this stuff.
When you see `try {`, you know that they're all the same.  You can thus reorder them without worrying.
So long as you know what family one of them is from, you know the rest are the same.

## This case really is common

The rust compiler uses `try` blocks in a bunch of places already.  Last I checked, they were *all* homogeneous.
(Though of course it's possible that some have been added since then.)

Let's look at a couple of examples.

This one is single-`?` on `Option`, basically a `map`, and thus is homogeneous:

```rust
let before = try {
    let span = self.span.trim_end(hole_span)?;
    Self { span, ..*self }
};
```

This one is homogeneous on the same visitor type, but on nightly ends up needing
the type annotation because it's the method-call case discussed above:

```rust
let result: ControlFlow<()> = try {
    self.visit(typeck_results.node_type(id))?;
    self.visit(typeck_results.node_args(id))?;
    if let Some(adjustments) = typeck_results.adjustments().get(id) {
        adjustments.iter().try_for_each(|adjustment| self.visit(adjustment.target))?;
    }
};
result.is_break()
```

This one is homogeneous because both are `io::Result<_>`s:

```rust
let r = with_no_trimmed_paths!(dot::render_opts(&graphviz, &mut buf, &render_opts));

let lhs = try {
    r?;
    file.write_all(&buf)?;
};
```

This one is homogeneous because both `?`s are on `Option`s:

```rust
let insertable: Option<_> = try {
    if generics.has_impl_trait() {
        None?
    }
    let args = self.node_args_opt(expr.hir_id)?;
    let span = tcx.hir().span(segment.hir_id);
    let insert_span = segment.ident.span.shrink_to_hi().with_hi(span.hi());
    InsertableGenericArgs {
        insert_span,
        args,
        generics_def_id: def_id,
        def_id,
        have_turbofish: false,
    }
};
return Box::new(insertable.into_iter());
```

These are again all `io::Result`s, where the annotation might not be needed because
that failure class wants `io::Error` specifically, but that's be clearer with this RFC:

```rust
fn export_symbols(&mut self, tmpdir: &Path, _crate_type: CrateType, symbols: &[String]) {
    let path = tmpdir.join("symbols");
    let res: io::Result<()> = try {
        let mut f = File::create_buffered(&path)?;
        for sym in symbols {
            writeln!(f, "{sym}")?;
        }
    };
    if let Err(error) = res {
        self.sess.dcx().emit_fatal(errors::SymbolFileWriteFailure { error });
    } else {
        self.link_arg("--export-symbols").link_arg(&path);
    }
}
```

Another place where everything is `io::Result<_>` already, so homogeneous would be fine
and would allow removing the `let` & type annotation:

```rust
if tcx.sess.opts.unstable_opts.dump_mir_graphviz {
    let _: io::Result<()> = try {
        let mut file = create_dump_file(tcx, "dot", pass_num, pass_name, disambiguator, body)?;
        write_mir_fn_graphviz(tcx, body, false, &mut file)?;
    };
}
```


# Prior art
[prior-art]: #prior-art

Languages with traditional exceptions don't return a value from `try` blocks, so don't have this problem.
Even checked exceptions are still always the `Exception` type.

## Scoping of nullability checks

In C#, the `?.` operator is scoped without a visible lexical block.
We could try to special-case `?.`, maybe over an edition change, to do something similar instead of needing the `try { ... }` at all.

The invisible scope can be trouble, however.  Take this program:

<!-- https://dotnetfiddle.net/HWGfKf -->

```cs
using System;
using FluentAssertions;

public class Foo {
	public string val;
}

public class Program
{
	private Foo? foo;

	public static void Main()
	{
		var program = new Program();
		program.foo?.val.Should().NotBeNull(); // Check 1
		Console.WriteLine("FirstOnePassed");
		(program.foo?.val).Should().NotBeNull(); // Check 2
	}
}
```

The first check never actually runs, because the `?.` skips it, as it's scoped to the statement.
The second check fails, because the `?.` got scoped to the parens.

Translating the two to Rust, they'd be
```rust
try { program.foo?.val.should().not_be_null() };
```
vs
```rust
try { program.foo?.val }.should().not_be_null();
```
where having the lexical scope visible emphasizes what happens if the `?` does short-circuit.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

Questions to be resolved in nightly:
- [ ] How exactly should the trait for this be named and structured?


# Future possibilities
[future-possibilities]: #future-possibilities

## Annotated heterogeneous `try` blocks

We could have `try ☃️ anyhow::Result<_> { ... }` blocks that use the old `?` desugaring.
(Insert your favourite token in place of ☃️, but please don't discuss that in *this* RFC.)

The extra token is negligible compared to the type annotation, unlike it would be in the homogeneous case.

That could be done at any point, as it's not a breaking change, thanks to `try` being a keyword.

The [flavour] conversation might find a version of this that could go well with `async` blocks too.

There are also other possible versions of this taking more advantage of the residual type to avoid needing
to write the `_` in more cases.  Spitballing, you could have things like `try ☃️ Option` or `try ☃️ anyhow::Result`,
say, where that isn't a type but is instead a 1-parameter type *constructor*.

[flavour]: https://github.com/rust-lang/rfcs/pull/3710

## Integration with `yeet`

This RFC has no conflict with [`yeet`], though it does open up some new questions.

[`yeet`]: https://github.com/rust-lang/rust/issues/96373

In many ways, the discussion here is similar to an open question about `yeet`
around what conversions, if any, it can do.

For example, if I'm in a `-> io::Result<()>` function, can I do
```rust
yeet ErrorKind::NotFound;
```
or would it need to be
```rust
yeet ErrorKind::NotFound.into();
```
or even require full specificity?
```rust
yeet io::Error::from(ErrorKind::NotFound)
```

One potentially-interesting version of that would be to keep `yeet` as
*heterogeneous* inside the *homogeneous* `try` blocks.

That would mean that it would still be the `?`s that would pick the return type,
but you'd be able to `yeet` more-specific types that would get translated.

For example, that could allow something like
```rust
let r = try {
    let f = File::open_buffered(path)?;
    let mut magic = [0; 4];
    f.read_exact(&mut magic)?;
    if (magic == [0; 4]) {
        yeet ErrorKind::InvalidData;
    }
};
```
where the `?`s are still homogeneous, picking `io::Result<()>` as the return type
for the block, but still allowing error-conversion in the `yeet` so you can `yeet`
the "more specific" type and still have the compiler figure it out.

