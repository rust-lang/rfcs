- Feature Name: `try_blocks_heterogeneous`
- Start Date: 2026-04-12
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

## Summary
[summary]: #summary

[RFC 3721](https://github.com/rust-lang/rfcs/pull/3721) implemented default support for homogeneous `try {...}` blocks, where all `?`s return the same error type. This RFC aims to provide support for explicit annotation of the returned error type from a `try {...}` block.

## Motivation
[motivation]: #motivation

> I'm a bit concerned about this change. Applications and libraries often use crates like `thiserror` to automatically group errors. For example, I often write something like
>
> ```rust
> #[derive(Error)]
> enum MyError {
>     #[error("Failed to parse config: {0}")]
>     InvalidConfig(#[from] serde::Error),
>     #[error("Failed to connect to server: {0}")]
>     ServerConnectionFailed(#[from] io::Error),
>     ...
> }
> ```
>
> which I then use as
>
> ```rust
> fn example() -> Result<(), MyError> {
>     let config = parse_config()?; // ? promotes serde::Error to MyError
>     let server = connect_to_server(server.url)?; // ? promotes io::Error to MyError
>     // ...
> }
> ```
>
> With this change, this approach would stop working in `try` blocks.
>
> ~ [purplesyringa commenting on #3721](https://github.com/rust-lang/rfcs/pull/3721#issuecomment-2466852085)

Currently there is no way to get the following example to compile, as the compiler is unable to safely determine the correct types returned from the try blocks, and no notation is available for the user to specify the type:

```rust
#![feature(try_blocks)]

use std::num::ParseIntError;

#[derive(Debug)]
struct Error1;

#[derive(Debug)]
struct Error2;

impl From<ParseIntError> for Error1 {
    fn from(_: ParseIntError) -> Self {
        Self
    }
}

impl From<ParseIntError> for Error2 {
    fn from(_: ParseIntError) -> Self {
        Self
    }
}

impl From<Error1> for Error2 {
    fn from(_: Error2) -> Self {
        Self
    }
}

impl From<Error2> for Error1 {
    fn from(_: Error2) -> Self {
        Self
    }
}

fn err1(s: &str) -> Result<i32, Error1> {
    Ok(s.parse()?)
}

fn err2(s: &str) -> Result<i32, Error2> {
    Ok(s.parse()?)
}

fn heterogeneous_into_exists() {
    let x = try { err1("1")? + err2("2")? };
    let y = try { err2("1")? + err1("2")? };
    assert_eq!(x.unwrap(), y.unwrap());
}
```

The initial experimental approach to provide a proof-of-concept introduced the (**deliberate placeholder**) syntax `try bikeshed ... {...}` in [PR #149489](https://github.com/rust-lang/rust/pull/149489).

>_For the remainder of this RFC we will continue with `bikeshed` to allow for examples which work on current nightly with `#![feature(try_blocks_heterogeneous)]`._
>
>_See open questions and [try bikeshed: What should the syntax be?](https://github.com/rust-lang/rust/issues/154128) for consideration of possible target syntax._

This would allow the above example to become:

```rust
fn heterogeneous_into_exists() {
    let x = try bikeshed Result<_, Error1> { err1("1")? + err2("2")? };
    let y = try bikeshed Result<_, Error1> { err2("1")? + err1("2")? };
    assert_eq!(x.unwrap(), y.unwrap());
}
```

and for cases where no direct `Into` relationship exists, or is needed, via a common third error type:

```rust
use std::{error::Error, fmt::Display};
impl Error for Error1 {}
impl Display for Error1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error1")?;
        Ok(())
    }
}

impl Error for Error2 {}
impl Display for Error2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error2")?;
        Ok(())
    }
}

fn heterogeneous_into_anyhow() {
    let x = try bikeshed anyhow::Result<_> { err1("1")? + err2("2")? };
    let y = try bikeshed anyhow::Result<_> { err2("1")? + err1("2")? };
    assert_eq!(x.unwrap(), y.unwrap());
}
```

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

_Assuming the explanation for try blocks is implemented as per RFC 3712, which contains:_

> This behaviour is what you want in the vast majority of simple cases.  In particular,
> it always works for things with just one `?`, so simple things like `try { a? + 1 }`
> will do the right thing with minimal syntactic overhead.  It's also common to want
> to group a bunch of things with the same error type.  Perhaps it's a bunch of calls
> to one library, which all use that library's error type.  Or you want to do
> [a bunch of `io` operations](<https://github.com/rust-lang/rust/blob/d6f3a4ecb48ead838638e902f2fa4e5f3059779b/>> > compiler/rustc_borrowck/src/nll.rs#L355-L367) which all use `io::Result`.  Additionally, `try` blocks work with
> `?`-on-`Option` as well, where error-conversion is never needed, since there is only `None`.
>
> It will fail to compile, however, if not everything shares the same error type.
> Suppose we add some formatting operation to the previous example:
>
> ```rust,edition2021,compile_fail
> let pair_result = try {
>     let a = std::fs::read_to_string("hello")?;
>     let b = std::fs::read_to_string("world")?;
>     let c: i32 = b.parse()?;
>     (a, c)
> };
> ```
>
> The compiler won't let us do that:
>
> ```text
> error[E0308]: mismatched types
>   --> src/lib.rs:14:32
>    |
>    |     let c: i32 = b.parse()?;
>    |                           ^ expected struct `std::io::Error`, found struct `ParseIntError`
>    = note: expected enum `Result<_, std::io::Error>`
>               found enum `Result<_, ParseIntError>`
> note: return type inferred to be `Result<_, std::io::Error>` here
>   --> src/lib.rs:14:32
>    |
>    |     let a = std::fs::read_to_string("hello")?;
>    |                                             ^
> ```
>
> ~~For now, the best solution for that mixed-error case is the same as before: to refactor it to a function.~~

_replace the final sentence with ..._

> While it may be obvious, or even irrelevant, to you which error type `pair_result` could potentially have, the compiler has no way to know this.
>
> Just like in other situations where the compiler cannot safely infer the exact type to use, you must annotate the block with a valid error type. We've already mentioned that `Result` automatically converts between error types where a suitable implementation of `Into` exists and you can leverage this and write:
>
> ```rust
> let pair_result = try bikeshed Result<_, PairError> {
>   let a = std::fs::read_to_string("hello")?;
>   let b = std::fs::read_to_string("world")?;
>   let c: i32 = b.parse()?;
>   (a, c)
> };
> ```
>
> As long as you have defined a suitable error:
>
> ```rust
> enum PairError {
>     IoError(Box<io::Error>),
>     ParseError(Box<num::ParseIntError>),
> }
> 
> impl From<io::Error> for PairError {
>     fn from(e: io::Error) -> Self {
>         Self::IoError(Box::new(e))
>     }
> }
> 
> impl From<num::ParseIntError> for PairError {
>     fn from(e: num::ParseIntError) -> Self {
>         Self::ParseError(Box::new(e))
>     }
> }
> ```
>
> Of course, there are crates available to simplify this if you do not want or need to create your own custom error type.

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This described the experimental implementation, currently in place in nightly, as implemented by [scottmcm in PR 149489](https://github.com/rust-lang/rust/pull/149489) with occasional additional comments by the RFC author.

### Compiler changes

#### Extend `ast::ExprKind::TryBlock` to store optional return type

```rust
pub enum ExprKind {
  ...
  // previously: TryBlock(Box<Block>),
  TryBlock(Box<Block>, Option<Box<Ty>>),
  ...
}
```

with associated adjustments to `visit::walk_fn`, `assert!`

#### Parse & pretty-print syntax `try bikeshed T {...}`

1. Add (temporary) `bikeshed` keyword (see [unresolved-questions]) to available Tokens

    ```rust
    enum TokenType {
    ...
        SymBikeshed,
    ...
    }
    ```

    ```rust
    macro_rules! exp {
    ...
        (Bikeshed) => { exp!(@sym, bikeshed, SymBikeshed) };
    ...
    }
    ```

2. Add `bikeshed` & `try_blocks_heterogeneous` spanned symbols

    ```rust
    symbols! {
    ...
        Symbols {
        ...
            bikeshed,
        ...
            try_blocks_heterogeneous,
        }
    ...
    }
    ```

3. Parse `try` blocks with optional `bikeshed` keyword

    ```rust
    /// Parses a `try {...}` or `try bikeshed Ty {...}` expression (`try` token already eaten).
    fn parse_try_block(&mut self, span_lo: Span) -> PResult<'a, Box<Expr>> {
        // ADD: do we have an annotated type?
        let annotation =
            if self.eat_keyword(exp!(Bikeshed)) { Some(self.parse_ty()?) } else { None };

        let (attrs, body) = self.parse_inner_attrs_and_block(None)?;
        if self.eat_keyword(exp!(Catch)) {
            Err(self.dcx().create_err(errors::CatchAfterTry { span: self.prev_token.span }))
        } else {
            let span = span_lo.to(body.span);
            //ADD: homogeneous & heterogeneous try blocks are behind separate feature gates
            let gate_sym =
                if annotation.is_none() { sym::try_blocks } else { sym::try_blocks_heterogeneous };
            
            self.psess.gated_spans.gate(gate_sym, span);
            Ok(self.mk_expr_with_attrs(span, ExprKind::TryBlock(body, annotation), attrs))
        }
    }
    ```

    ```rust
    fn is_try_block(&self) -> bool {
        self.token.is_keyword(kw::Try)
            && self.look_ahead(1, |t| {
                *t == token::OpenBrace
                    || t.is_metavar_block()
                    // ADD: optional `bikeshed` following `try`
                    || t.kind == TokenKind::Ident(sym::bikeshed, IdentIsRaw::No)
            })
            && self.token_uninterpolated_span().at_least_rust_2018()
    }
    ```

4. Correctly pretty print `bikeshed` annotated `try` blocks

    ```rust
    ast::ExprKind::TryBlock(blk, opt_ty) => {
        let cb = self.cbox(0);
        let ib = self.ibox(0);
        self.word_nbsp("try");
        // ADD: if there if a type annotation, prefix with `bikeshed`
        if let Some(ty) = opt_ty {
            self.word_nbsp("bikeshed");
            self.print_type(ty);
            self.space();
        }
        self.print_block_with_attrs(blk, attrs, cb, ib)
    }
    ```

#### Desugaring

1. Introduce `TryBlockScope` enum

    ```rust
    // The originating scope for an `Expr` when desugaring `?`
    enum TryBlockScope {
        /// There isn't a `try` block, so a `?` will use `return`.
        Function,
        /// We're inside a `try { … }` block, so a `?` will block-break
        /// from that block using a type depending only on the argument.
        Homogeneous(HirId),
        /// We're inside a `try bikeshed _ { … }` block, so a `?` will block-break
        /// from that block using the type specified.
        Heterogeneous(HirId),
    }
    ```

2. Update desugaring `try` blocks at definition site

    ```rust
    /// Desugar `try { <stmts>; <expr> }` into `{ <stmts>; ::std::ops::Try::from_output(<expr>) }`,
    /// `try { <stmts>; }` into `{ <stmts>; ::std::ops::Try::from_output(()) }`
    /// and save the block id to use it as a break target for desugaring of the `?` operator.
    fn lower_expr_try_block(&mut self, body: &Block, opt_ty: Option<&Ty>) -> hir::ExprKind<'hir> {
        let body_hir_id = self.lower_node_id(body.id);
        
        // ADD differentiation
        let new_scope = if opt_ty.is_some() {
            TryBlockScope::Heterogeneous(body_hir_id)
        } else {
            TryBlockScope::Homogeneous(body_hir_id)
        };
        let whole_block = self.with_try_block_scope(new_scope, |this| {
            let mut block = this.lower_block_noalloc(body_hir_id, body, true);
    ...
            this.arena.alloc(block)
        });

        // ADD identification of `try bikeshed` as typed blocks
        if let Some(ty) = opt_ty {
            let ty = self.lower_ty(ty, ImplTraitContext::Disallowed(ImplTraitPosition::Path));
            let block_expr = self.arena.alloc(self.expr_block(whole_block));
            hir::ExprKind::Type(block_expr, ty)
        } else {
            hir::ExprKind::Block(whole_block, None)
        }
    }
    ```

3. Update desugaring `?`, specifically in the construction of the `ControlFlow::Break` arm and the final return value

    ```rust
    /// Desugar `ExprKind::Try` from: `<expr>?` into:
    /// ```ignore (pseudo-rust)
    /// match Try::branch(<expr>) {
    ///     ControlFlow::Continue(val) => #[allow(unreachable_code)] val,,
    ///     ControlFlow::Break(residual) =>
    ///         #[allow(unreachable_code)]
    ///         // If there is an enclosing `try {...}`:
    ///         break 'catch_target Residual::into_try_type(residual),
    ///         // Otherwise:
    ///         return Try::from_residual(residual),
    /// }
    /// ```
    fn lower_expr_try(&mut self, span: Span, sub_expr: &Expr) -> hir::ExprKind<'hir> {
    ...
        // `ControlFlow::Break(residual) =>
        //     #[allow(unreachable_code)]
        //     return Try::from_residual(residual),`
        let break_arm = {
        ...

            //  (hir::LangItem, Result<HirId, LoopIdError>)
            let (constructor_item, target_id) = match self.try_block_scope {
                TryBlockScope::Function => {
                    (hir::LangItem::TryTraitFromResidual, Err(hir::LoopIdError::OutsideLoopScope))
                }
                TryBlockScope::Homogeneous(block_id) => {
                    (hir::LangItem::ResidualIntoTryType, Ok(block_id))
                }
                // `try bikeshed` treated like a function for construction of residual expression,
                // but with available Hirid for the source block
                TryBlockScope::Heterogeneous(block_id) => {
                    (hir::LangItem::TryTraitFromResidual, Ok(block_id))
                }
            };
            let from_residual_expr = self.wrap_in_try_constructor(
                // replace previous inline `if let Some() else` to differentiate try block vs function
                constructor_item,
                try_span,
                self.arena.alloc(residual_expr),
                unstable_span,
            );
            // replace `if let Some() else` with `if .is_ok() else` to differentiate try block vs function
            let ret_expr = if target_id.is_ok() {
    ...
        // originating scope: try blocks vs function
        match self.try_block_scope {
            TryBlockScope::Homogeneous(block_id) | TryBlockScope::Heterogeneous(block_id) => {
                hir::ExprKind::Break(
                    hir::Destination { label: None, target_id: Ok(block_id) },
                    Some(from_yeet_expr),
                )
            }
            TryBlockScope::Function => self.checked_return(Some(from_yeet_expr)),
        }
    }
    ```

## Drawbacks
[drawbacks]: #drawbacks

This adds further syntax complexity to the language with another, slightly different, way in which types must be annotated. The open question on the correct syntax shows that whatever is chosen it will not be immediately obvious to users.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

### Homogenous try-blocks with manual error conversion

Only support homogeneous `try` blocks and force manual conversion.

For example, you could do something like

```rust
try {
        err1("1").map_err(Into::<anyhow::Error>::into)?
            + err2("2").map_err(Into::<anyhow::Error>::into)?
    };
```

1. This leads much more verbose code where multiple error types are involved.
1. In cases where the final `Residual` is not any of the `Residuals` inside the `try` block (likely a very common situation with `anyhow`) this creates further verbosity by forcing turbofish annotation in at least one place.
1. Changing the block `Residual` requires multiple adjustments.
1. This breaks for cases where the `Try` type in question is not `Result`/`Option` unless it implements an equivalent of `map_err()`.
1. It is not immediately obvious to the user reading the block definition what the resulting `Residual` will be, the information is inside the block, not at the start of the definition. When you see `try bikeshed Foo {` you know the type without analysing the block.

### Fold through some type function that attempts to merge residuals

This is much less local, complex to implement and removes control from the user.

### Could we evolve this in future?

Once the correct keyword / syntax is identified the remainder is an early desugaring to existing features, this is the easiest kind of thing to change over editions.

Therefore, if in the future we get new type system features that would allow improved "fallback hinting" or inference of unannotated
`try` blocks, we could relax the restrictions on homogeneous `try` blocks while still maintaining the ability to annotate for explicit
clarity or where inferance is not possible. This would be easy to achieve over an edition change, but we don't need to wait for an unknown to ship something now; we can switch how it works later easily enough.

## Prior art
[prior-art]: #prior-art

Languages with traditional exceptions don't return a value from `try` blocks, so don't have this problem.
Even checked exceptions are still always the `Exception` type.

In C#, the `?.` operator is scoped without a visible lexical block.

### Related RFCs & experimental features

- [RFC #2388 `try-expr`](https://rust-lang.github.io/rfcs/2388-try-expr.html)
- [RFC #3721 `homogeneous_try_blocks`](https://rust-lang.github.io/rfcs/3721-homogeneous-try-blocks.html)
- [RFC #3058 `try-trait-v2`](https://rust-lang.github.io/rfcs/3058-try-trait-v2.html)
- [Tracking issue #91285 `try_trait_v2_residual`](https://github.com/rust-lang/rust/issues/91285)
- [Tracking issue #63178 `Iterator::try_find`](https://github.com/rust-lang/rust/issues/63178)
- [Tracking issue #79711 `array::try_map`](https://github.com/rust-lang/rust/issues/79711)
- [Tracking issue #89379 `try_array_from_fn`](https://github.com/rust-lang/rust/issues/89379)

## Unresolved questions
[unresolved-questions]: #unresolved-questions

- [ ] What should the syntax be? See [Issue #154128](https://github.com/rust-lang/rust/issues/154128) for discussion of alternatives (`:`, `->`, `as`, _nothing_ just `try T {}`, or even `try ☃️ {...}` as in RFC3721)
- [ ] What type should be annotated? This should probably be the full type, with optional inference, as currently implemented for `bikeshed`, but see [Issue #154127](https://github.com/rust-lang/rust/issues/154127) for discussion.

## Future possibilities
[future-possibilities]: #future-possibilities

### Allow inference via function return type or variable binding

For cases such as

```rust
fn heterogeneous_via_return_type() -> Result<(), Error1> {
    let x = try { err1("1")? + err2("2")? }?;
    let y = try { err2("1")? + err1("2")? };
    let _: Result<_, Error2> = try { err2("1")? + err1("2")? };
    assert_eq!(x, y?);
    Ok(())
}
```

where the errors involved all implement `Into<Error1>`

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.
