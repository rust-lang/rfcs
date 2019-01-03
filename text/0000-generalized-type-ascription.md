- Feature Name: `generalized_type_ascription`
- Start Date: 2018-08-10
- RFC PR: _
- Rust Issue: _

# Summary
[summary]: #summary

[RFC 803]: https://github.com/rust-lang/rfcs/blob/master/text/0803-type-ascription.md

This RFC supersedes and subsumes [RFC 803].
We finalize a general notion of type ascription uniformly in patterns,
expressions, `let` bindings. You may now for example write:

```rust
let x = (0..10).collect() : Vec<_>;

let alpha: u8 = expr;
    ^^^^^^^^^

let [x: u8, y, z] = stuff();
    ^^^^^^^^^^^^^

if let Some(beta: u8) = expr { .. }
            ^^^^^^^^

for x: i8 in 0..100 { .. }
    ^^^^^
```

Here, the underlined bits are patterns.

Finally, when a user writes `Foo { $field: $pat : $type }`, and when
`$pat` and `$type` are syntactically α-equivalent, the compiler emits a
warn-by-default lint suggesting: `Foo { $field: ($pat : $type) }`.

# Motivation
[motivation]: #motivation

## Type ascription is useful

[RFC_803_motivation]: https://github.com/rust-lang/rfcs/blob/master/text/0803-type-ascription.md#motivation

[pointfree persuasion]: https://en.wikipedia.org/wiki/Tacit_programming

[TwoHardThings]: https://martinfowler.com/bliki/TwoHardThings.html

Type ascription is useful. A motivation for the feature is noted in the merged,
but thus far not stabilized, [RFC 803][RFC_803_motivation] which introduces
type ascription in expression contexts as `expr : T`. We reinforce that RFC
with more motivation:

1. With type ascription, you can annotate smaller bits and subsets of what you
   previously needed to. This especially holds in pattern contexts.
   This will be made clear later on in this RFC.

2. Type ascription helps retain writing flow.
   When you are writing a complex chain of methods,
   sometimes you realize that you need to add an annotation,
   either to make things compile or for the purposes of documentation.
   When you do that, it follows the flow of writing the method chain
   to not have to split things into let bindings; instead,
   you can simply add an annotation to the right of a method call in the chain
   and then continue on with the next method call.

   Similarly, type ascription also follows the reading flow well and does so
   in a non-intrusive way.

3. Introducing a temporary `let` binding of form `let ident: type = expr;`,
   as a substitute for type-ascription, forces programmers to invent artificial
   variable names. Naming is hard (particularly for those who are of the
   [pointfree persuasion]). As [the saying goes][TwoHardThings]:
   > “There are only two hard things in Computer Science: cache invalidation and naming things”.

   By reducing the pressure on Rustaceans to name artificial units
   we can let programmers focus on naming where it matters more (API boundaries).

   Another instance where temporary artificial bindings may be forced upon users
   are generic functions where the expected parameter type does not sufficiently
   constrain type inference causing it to fail. With expression type ascription
   it becomes possible to write `fun(expr: TheType)`. This avoids the artificial
   binding.

4. Turbofish is not always possible! Consider for example:

   ```rust
   fn display_all(elems: impl Iterator<Item: Display>) { ... }
   ```

   As of today (2018-08-10), it is not possible to use turbofish at all
   as you could have done had you instead written:

   ```rust
   fn display_all<I: Iterator<Item: Display>>(elems: I) { ... }
   ```

   While this may change in the future, it may also be the case that anonymous
   `arg: impl Trait`s can't ever be turbofished. In such a case, type ascription
   is our saving grace; it works independently of how `display_all` is defined
   and let's us easily constrain the type of `elems` in a syntactically
   light-weight way in any case.

   Another case of not being able to use turbofish is the `.into()` method.
   Because the `Into` trait is defined as:

   ```rust
   pub trait Into<T> {
       fn into(self) -> T;
   }
   ```

   as opposed to (and it couldn't be because the semantics would be different):

   ```rust
   pub trait Into {
       fn into<T>(self) -> T;
   }
   ```

   there is no type parameter on `into` to turbofish. Thus, you may not write:
   `thing.into::<Foo>()` but you can write `thing.into() : Foo`.

5. Type ascription is helpful when doing *type* driven development
   and opens up more possibilities to move in the direction of
   interactive development as is possible with [agda-mode] and [idris-mode].

[agda-mode]: http://agda.readthedocs.io/en/v2.5.2/tools/emacs-mode.html
[idris-mode]: https://github.com/idris-hackers/idris-mode

6. Type ascription helps with [RFC 2071] which notes that you sometimes
   have to introduce a `let` binding to please the type checker. An example:

   ```rust
   existential type Foo: Debug;

   fn add_to_foo_2(x: Foo) {
       let x: i32 = x;
       x + 1
   }
   ```

   However, this does not seem particularly ergonomic and introduces,
   relatively speaking, a lot of boilerplate.
   Instead, we can make this more ergonomic using ascription:

   ```rust
   fn add_to_foo_2(x: Foo) {
       x : i32 + 1
   }
   ```

7. As `$($pat:pat),*` is a legal pattern and the pattern grammar now accepts
   `$pat: pat : $type: ty`, it becomes possible to write macros that can
   match function signatures with arbitrary patterns for arguments.

8. Type ascription formalizes an already informal mode of communication.
   For example, Rustaceans already commonly use `x: u8` or `42: usize`
   to denote that the left hand side is of the type specified when talking
   with each other. By introducing this into the language itself,
   we align the language with how user's think.

   [issue#53572]: https://github.com/rust-lang/rust/issues/53572

   Additionally, `<pat> : <type>` is already erroneously used in error messages.
   Such an episode where a user was misled by the compiler occurred in
   [rust-lang/rust#53572][issue#53572] where the user wrote:

   ```rust
   for i in 0..1000 {
       println!("{}", i.pow(2));
   }
   ```

   which the compiler rejected, suggesting that the user should instead write:

   ```rust
   for i: i32 in 0..1000 {
   ```

   However, this is currently invalid in today's Rust.
   But this RFC would make it valid, thus making the error message correct.

## Type ascription has already been accepted as an RFC

We noted previously that [RFC 803] already accepted type ascription in
expression contexts. Thus, we have already collectively deemed to some extent
that ascription is something we want. However, the previous RFC did not apply
ascription uniformly. We believe it is beneficial to do so. We also believe
that much of the motivation for accepting RFC 803 applies to the extensions
proposed here.

## More DRY code

By introducing type ascription as a pattern, as compared to type ascription in
expression contexts or using `let` bindings, we can also get away with leaving
more inferred when pattern matching.
For example, consider:

```rust
let temporary: Option<Vec<u8>> = expr;
match temporary {
    None => logic,
    Some(vec) => logic,
}
```

as compared to:

```rust
match expr : Option<Vec<u8>> {
    None => logic,
    Some(vec) => logic,
}
```

and against:

```rust
match expr {
    None => logic,
    Some(vec: Vec<u8>) => logic,
}
```

or analogously:

```rust
if let Some(vec: Vec<u8>) = expr {
    logic
} else {
    logic
}
```

In the last two cases, the typing annotation is both *most local* and also does not
require you to annotate information that is both obvious to the reader
(who is familiar with `Option<T>`) and to the compiler
(that `expr : Option<?T>` for some `?T`).
Because the annotation is more local, we can employ more local reasoning.
This is particularly useful if the `enum` contains many variants in which
case the type ascription on `expr` may not be immediately visible.

[str_parse]: https://doc.rust-lang.org/nightly/std/primitive.str.html#method.parse

A realistic example of this scenario of this occurring is with the
[`.parse()`][str_parse] method. For example, instead of writing:

```rust
match foo.parse::<i32>() {
    Ok(x) => ...,
    Err(e) => ...
}
```

or writing:

```rust
match foo.parse() : Result<i32, _> {
    Ok(x) => ...,
    Err(e) => ...,
}
```

we can write:

```rust
match foo.parse() {
    Ok(x: i32) => ...,
    Err(e) => ...,
}
```

This annotates the important information clearly and where it matters most.

## Addressing concerns of match ergonomics

[match_concerns]: https://internals.rust-lang.org/t/lived-experiences-strange-match-ergonomics/7817

Some [concerns][match_concerns] have been noted about the match ergonomics
feature of Rust. By using type ascription in pattern contexts,
we can document and be more confident about what is and what is not a reference.
For example, given:

```rust
match &expr {
    None => logic,
    Some(vec: &Vec<u8>) => logic,
}
```

we can be sure that `vec` is a reference.
If we instead write:

```rust
let Struct { field: x: i32 } = expr;
```

we can know for certain that `x` is not a borrow.

## A more unified syntax and mental model

Given the changes in this RFC, note that when you write:

```rust
let alpha: Beta = gamma;
    ^^^^^^^^^^^
    A pattern!
```

before this RFC, it was the case that `alpha: Beta` in `let` bindings
were *a special construct*. With this RFC, it not and instead,
it is simply a part of the pattern grammar. You could also say that we already
had type ascription in "pattern context" prior to this RFC, and that the
language was just not very principled about it.

In this RFC, we try to rectify this situation and apply the grammar uniformly.
Since uniformity is our friend in constructing a language which is easy to
understand, we believe this RFC will help in learning and the teaching of Rust.
To further that end, we make sure in this RFC to use the same type ascription
syntax everywhere ascription applies. We do this both in expression and
pattern context by introducing into the grammar:

```rust
pat : pat ':' ty_sum ;
expr : expr ':' ty_sum ;
```

Notice in particular that the `':' ty_sum` is the same in both productions here.

[parser-lalr.y]: https://github.com/rust-lang/rust/blob/9b5859aea199d5f34a4d4b5ae7112c5c41f3b242/src/grammar/parser-lalr.y#L722-L827

Another thing to note is that grammar changes described in the [summary]
above replace most of the productions listed in the highlighted section and
other parts of the slightly outdated [parser-lalr.y] file with something less
complicated and smaller.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

This RFC extends type ascription in expression contexts and introduces
type ascription in pattern contexts. In the next two sections we will
go through what this means for you as a user of Rust.

## Type ascription in expressions

[implicit coercions]: https://doc.rust-lang.org/beta/reference/type-coercions.html

[RFC 803] introduced type ascription in expression contexts stating
that you may write `expr : Type` to ensure that `expr` is of a well-formed
type `Type`. This includes sub-typing and triggering [implicit coercions].
However, unlike with the `as` operator, type ascription may not trigger
explicit coercions. As an example, consider:

```rust
let mut x = 1 : u8; // OK. Implicit coercion.
let mut y = &mut x;
let _ = y : &u8; // OK. Implicit coercion &mut u8 -> &u8;
                 // Does not work on nightly yet.

let _ = 42u8 : usize; // Error! This is an explicit coercion.

let _ = 42u8 as usize; // OK. `as` permits explicit coercions (casts).
```

Type ascription in expression contexts has since been implemented and currently
available on nightly compilers. Thus, when we wish to aim to define a program like:

```rust
fn main() {
    println!("{:?}", (0..10).map(|x| x % 3 == 0).collect());

    let _ = Box::new("1234".parse().unwrap());
}
```

but get two errors of form:

```rust
error[E0283]: type annotations required: ...

error[E0282]: type annotations needed
```

you can resolve the errors by writing:

```rust
#![feature(type_ascription)]

fn main() {
    println!("{:?}", (0..10).map(|x| x % 3 == 0).collect() : Vec<_>);

    let _ = "1234".parse().unwrap(): usize.into(): Box<_>;
}
```

-----------------

*Aside:* You can also resolve the above errors by using turbofish and `Box::new`:

```rust
fn main() {
    println!("{:?}", (0..10).map(|x| x % 3 == 0).collect::<Vec<_>>());

    let x = Box::new("1234".parse::<usize>().unwrap());
}
```

### In macros

Note that the fact that `expr : Type` is a valid expression extends to macros
as well and this is implemented in a nightly compiler right now.
For example, we can make and invoke macro that ascribes an expression with a
type `Vec<$t>` with the following valid snippet:

```rust
#![feature(type_ascription)]

macro_rules! ascribe {
    ($e: expr, $t: ty) => {
        $e : Vec<$t>
    }
}

fn main() {
    let _ = ascribe!(vec![1, 2, 3], u8);
}
```

### Precedence of the operator

[prec_unresolved]: https://github.com/rust-lang/rfcs/blob/master/text/0803-type-ascription.md#unresolved-questions

[prec_ref]: https://doc.rust-lang.org/beta/reference/expressions.html

[RFC 803] proposed and implemented that `:` as an operator in expression
contexts should have the same precedence as the `as` operator
*([see the reference][prec_ref])*. However, the RFC also
[left this question unresolved][prec_unresolved] and asked:

> Is the suggested precedence correct?

We argue in this RFC that the current implementation is sub-optimal and thus
propose that the precedence should be slightly changed.

To see why, consider the example above where we wrote:

```rust
let _ = ("1234".parse().unwrap() : usize).into() : Box<_>;
```

Notice in particular here that we've had to enclose the inner ascription
in parenthesis. Consider that you are writing this snippet and reach the
`.into()`. Once you do that, you'll need to select everything on the line
until before the `=` token. This can slow down your writing flow.
Furthermore, as we chain more and more methods, the build-up of parenthesis
can increase and thus make writing and reading further impaired. An example:

```rust
let x = (((0..10)
    .map(some_computation)
    .collect() : Result<Vec<_>, _>)
    .unwrap()
    .map(other_computation) : Vec<usize>)
    .into() : Rc<[_]>;
```

We suggest instead that you should be able to write:

```rust
let x = (0..10)
    .map(some_computation)
    .collect() : Result<Vec<_>, _>
    .unwrap()
    .map(other_computation) : Vec<usize>
    .into() : Rc<[_]>;
```

To that end, `foo : bar.quux()` and `foo : bar.quux` should unambiguously be
interpreted as `(foo : bar).quux()` and `(foo : bar).quux`. 

However, this does not mean that the operator `:` should bind more tightly than
operators such as the unary operators `-`, `*`, `!`, `&`, and `&mut`.
In particular, for the latter two operators, we expect that if someone writes
`&x : Type`, it would be interpreted as `(&x) : Type` as opposed to `&(x : Type)`.

Instead, we propose that whenever type ascription is followed by a
field projection or a method call, the projections or the call should apply
to the entire ascribed expression.

Note in particular that when you write `&a:b.c`, because `&` binds more tightly
than `:` but `.` binds more tightly than `&`, the expression associates as
`&((a : b).c)`. However, when you write `&x.y:z`, it instead associates as
`(&(x.y)) : z`.

## Type ascription in patterns

With this RFC we extend the pattern syntax to allow type ascription inside of
patterns. What this means is that `MyPattern : Type` is itself a valid pattern.
For example, you may write:

```rust
match compute_stuff() {
    Ok(vec: Vec<u8>) => {
        // Logic...
    },
    Err(err: MyError<Foo>) => {
        // Logic...
    },
}
```

The following is also valid:

```rust
match do_stuff() {
    None => ...,
    // We don't recommend this way of writing but it is possible:
    Some(x): Option<u8> => ...,
}

if let Thing { field: binding: MyType } = make_thing() {
    ...
}
```

You may now also write:

```rust
for x: i8 in 0..100 {
    ...
}
```

instead of as before:

```rust
for x in 0_i8..100 {
    ...
}
```

or worse yet:

```rust
for x in 0..100 {
    // This would be more realistic if the iterator
    // couldn't use literal suffixes as with 0_i8..100.
    let x: i8 = x;

    ...
}
```

### In macros

Just as we noted before that type ascription work in expression macros so may
you use type ascription in pattern macros. For example:

```rust
macro_rules! ascribe {
    ($p: pat, $n: expr) => {
        $p : [u8, $n]
    }
}

fn main() {
    let ascribe!([x, y, z], 3) = [3, 1, 2];
}
```

It is possible to do this in a backwards compatible manner because the token `:`
is not in the follow set of `pat` fragments. This means that when you write

```rust
macro_rules! test {
    ($p:pat : u32) => {}
}
```

The compiler will complain that:

```rust
error: `$p:pat` is followed by `:`, which is not allowed for `pat` fragments
 --> src/main.rs:2:12
  |
2 |     ($p:pat : u32) => {}
  |             ^
```

### Let bindings

Before this RFC when you wrote something like:

```rust
let quux: u8 = 42;
    ^^^^
```

The underlined part was the pattern, but the typing annotation `: u8` to
the right was *not* part of the pattern. With this RFC, we unify the language
and we can now say that everything after `let` and before `=` is the pattern:

```rust
let quux: u8 = 42;
    ^^^^^^^^
    Pattern!
```

Another implication of introducing type ascription in pattern contexts is that
that you may say things like:

```rust
let [alpha: u8, beta, gamma] = [1, 2, 3];

let (alpha: u8, beta: i16, gamma: bool) = (1, -2, true);
```

## Linting ascription of named `struct` literals and patterns

Consider a struct:

```rust
struct Foo<T> {
    bar: T
}
```

When it comes to type ascribing the field `bar` in a struct literal expression
such as `Foo { bar: x : Type }` or in particular when you type ascribe
`bar` in a pattern: `Foo { bar: x : Type }` it is not always very clear from
this way of writing what is what.

We propose therefore that the compiler should provide a warn-by-default lint
that suggests that you should wrap the ascription in parenthesis like so:

```rust
let x = Foo { bar: (x : Type) }

let Foo { bar: (x: Type) } = ...;
```

This lint only applies when after giving fresh names for all identifiers inside
`x` and `Type`, their token streams match (α-equivalence).
For example, this means that if you write `let Foo { bar: x : u32 }`
or `let Foo { bar: &x : &X }` the compiler will emit a warning.
However, if you write `let Foo { bar: x : Vec<u8> }` it will not.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

## Grammar

The following alternatives are modified in the expression grammar:

```rust
ascribe: ':' ty_sum ;

expr
: ...
| expr ascribe // This is specified in RFC 803 but it is included for completeness.
;
```

Here, the precedence of `:` in the alternative `expr ascribe` is the same as
the operator `as`. However, when the parser encounters type ascription of an
expression immediately followed by a field projection or a method call,
then the parser shall interpret the projection and the call as being performed
on the ascribed expression. Thus, if a user writes `expr : type . field`
the parser associates this as `(expr : type) . field`. Similarly, if a user
writes `expr : type . method(..)` the parser associates this as
`(expr : type) . method(..)`. An implementation of this wrt. method calls
exists in [rust-lang/rust#33380](https://github.com/rust-lang/rust/pull/33380).

To the pattern grammar, the following alternative is added:

```rust
pat
: ...
| pat ascribe
;
```

The operator `:` binds more tightly than `ref` and `ref mut` but binds less
tightly than `&` and `&mut` in pattern contexts. This is required because
currently, the following compiles:

```rust
#[derive(Copy, Clone)]
struct X {}
let a = X {};

let ref b: X = a; // Note the type!
let &c : &X = b; // And here!
let d: X = c;
```

This entails for example that a Rust compiler will interpret `ref x : T` as
`ref (x : T)` instead of `(ref x) : T`. The same applies to `ref mut`.
However, `&x : T` and `&mut x : T` will be associated as `(&x) : T`
and `(&mut x) : T`.

The grammar of `let` bindings is changed from:

```rust
let : LET pat ascribe? maybe_init_expr ';'
```

to:

```rust
let : LET pat maybe_init_expr ';' ;`
```

### Lints

If and only if when the parser encounters, both in pattern and expression contexts:

```rust
$path { $ident: $pat : $ty }
```

where `$path`, `$pat`, and `$ty` are the usual meta variables,
and where `$pat` and `$ty` are α-equivalent token streams
(checkable by generating fresh names for all identifiers and
testing if they are the same, ignoring span information),
the compiler will emit a warn-by-default lint urging the user to instead write:

```rust
$path { $ident: ($pat : $ty) }
```

In pattern contexts, wrapping in parenthesis was made valid by
[rust-lang/rust#48500](https://github.com/rust-lang/rust/pull/48500).

The tool `rustfmt` will similarly prefer the latter formatting.

## Semantics and Type checking

### Expressions

The operational semantics and type checking rules for type ascription in
expression contexts is *exactly* as specified in [RFC 803].

Let `x` denote a term.

Let `τ` and `σ` denote types.

Let `Γ` denote the environment mapping names to values.

Let `Δ` denote the typing environment.

Let `Δ ⊢ ImplicitlyCoercible(τ, σ)` denote that `τ` is implicitly coercible
to `τ` in the typing environment `Δ`. Being implicitly coercible includes
sub-typing.

The type checker respects the following typing rule:

```
Δ    ⊢ τ type
Δ    ⊢ σ type
Δ    ⊢ ImplicitlyCoercible(τ, σ)
Δ, Γ ⊢ x : τ
-------------------------------- ExprTypeAscribe
Δ, Γ ⊢ (x : σ) : σ
```

Since before, we have the typing rule that:

```
Δ ⊢ τ type
-------------------------------- SelfCoercible
Δ ⊢ ImplicitlyCoercible(τ, τ)
```

From these typing rules, it follows that:

```
Δ    ⊢ τ type
--------------------------------
Δ    ⊢ ImplicitlyCoercible(τ, τ)     Δ, Γ ⊢ x : τ
-------------------------------------------------- ExprSelfTypeAscribe
Δ, Γ ⊢ (x : τ) : τ
```

N.B: See [RFC 803] for details on temporaries. Where ownership is concerned
the usual rules for `x` should apply.

### Patterns

As with type ascription in expression contexts, implicit coercions are also
permitted when matching an expression against a pattern.
From before this RFC, you could for example write:

```rust
let mut a = 1;
let b: &mut u8 = &mut a;
let c: &u8 = b; // Implicit coercion of `&mut u8` to `&u8`.
```

To stay compatible with this and avoid breaking changes, this behaviour is preserved.

When type checking an expression against a pattern where the pattern includes
a type ascription of form `pat : type`, the compiler will ensure that the
expression fragment corresponding to the ascribed pattern `pat` is implicitly
coercible (including sub-typing) to the `type` ascribed.

As for the operational semantics, if type of the expression fragment and
the ascribed-to type are an exact match, then type ascription is a no-op.
Otherwise, the semantics are those of the implicit coercion.

### Ascribing `impl Trait`

[RFC 1951]: https://github.com/rust-lang/rfcs/blob/master/text/1951-expand-impl-trait.md
[RFC 2071]: https://github.com/rust-lang/rfcs/blob/master/text/2071-impl-trait-existential-types.md#reference-impl-trait-in-let-const-and-static

Ascribing an expression or a pattern to a type `impl Trait` for some `Trait`
is permitted by the compiler. When a pattern or expression inside an `fn` body
is ascribed with a type of form `impl Trait`, the type checking rules are as
specified by [RFC 2071] with respect to `let` bindings.

# Drawbacks
[drawbacks]: #drawbacks

## Language Complexity

We believe that we've demonstrated that this RFC simplifies the language by
applying rules uniformly, and thus has a negative complexity cost on balance.
However, this view may not be shared by everybody. It is a legitimate position
to take to view this as an increase in language complexity.

## Potential conflict with named arguments

Consider the following function definition:

```rust
fn foo(alpha: u8, beta: bool) { ... }
```

Some have proposed that we introduce named function arguments into Rust.
One of the syntaxes that have been proposed are:

```rust
foo(alpha: 1, beta: true)
```

However, this syntax conflicts with type ascription in expression contexts.
For those who value named arguments over type ascription, they may want to retain
the syntax `argument: expr` because it is reminiscent of the struct literal
syntax `Struct { field: expr }`. However, we argue that it is only weakly so.
In particular, note that functions are not called with braces but that they are
called with parenthesis. Therefore, they are more syntactically kindred with
tuple structs which have positional field names. Thus, a more consistent
function call syntax would be `foo { alpha: 1, beta: true }`.

Furthermore, it is still possible to come up with other syntaxes for named arguments.
For example, you could hypothetically write `foo(alpha = 1, beta = 2)`.

### Alternative: Structural records

Another possibility is to introduce structural tuple records and then use them
to emulate named arguments in a light weight manner in that way:

```rust
fn foo(stuff: {alpha: u8, beta: bool, gamma: isize }) { .. }

foo({ alpha: 1, gamma: -42, beta: true })
```

As you can see, the syntactic overhead at the call site is quite minor.
These structural records also have other benefits such as conveying semantic
intent better than the positional style tuples.
They are a middle-ground between tuples and introducing a named struct.

### Type ascription is RFC-accepted

It should be noted that while named arguments do *not* have an accepted RFC,
type ascription in expression contexts *do* ([RFC 803]).
Also consider that named arguments have had notable opposition from
parts of the community in the past.

## Sub-optimal experience in named fields

One oft voiced criticism against the proposed syntax for type ascription
both in expression and pattern contexts is that they don't mesh well with
struct literal expressions and their corresponding patterns.
For example, when you write `Foo { bar: baz : u8 }` in a pattern context,
you *have* to introduce the binding `baz` to be able to type-ascribe `bar`.
That is, you may not do field punning in expression or pattern contexts
combined with ascription like so: `Foo { bar : Type }` because `Type`
would be ambiguous with `Foo { bar: binding }`.

In this context, the syntax is not as readable and ergonomic as we would like.
However, it is our contention that the need to use the syntax will not be that
common and that consistency is paramount. To mitigate the readability angle,
this RFC proposes to lint towards usage of parenthesis when `baz` is an identifier.

One possible way to avoid forcing the user to write `Foo { bar: (bar: u8) }`
in a pattern context might be to allow the user to ascribe the field directly
by writing `Foo { bar: : u8 }`. One could potentially write this as:
`Foo { bar :: u8 }`. One drawback in this approach is that it may confuse
readers with paths.

# Rationale and alternatives
[alternatives]: #rationale-and-alternatives

## Do nothing

We could opt to not do anything and leave type ascription in a half-baked
and inconsistent state. In that case, we would only have [RFC 803] which
gives us type ascription in expression contexts and in a mandatory way on
function parameters as well as optionally on `let` bindings.
It is also possible to unaccept [RFC 803] and have no type ascription but for
function definitions and let bindings. 

## A different syntax

We aim to design a consistent language with as syntax that is as uniform as
possible because it aids in learning and teaching Rust. Since the token `:`
is already used on `let` bindings and on function parameters to annotate,
or "ascribe", the type, it would be most consistent to use the existing syntax.
Indeed, this is a chief motivation for why [RFC 803] uses the proposed syntax
in this RFC.

However, there are also other possible syntaxes we may consider:

### `type Foo`

[internals_6666]: https://internals.rust-lang.org/t/idea-change-type-ascription-syntax-from-to-type-keyword/6666

[An internals issue][internals_6666] proposed that we instead use the following
syntax:

```rust
let foo = (0..10).collect() type Vec<_>;
```

or possibly:

```rust
let foo = (0..10).collect() : type Vec<_>;
```

We argue that this does not read well as it has the wrong tense ("type" instead
of "typed at"). As noted above it is also inconsistent and would unnecessarily
introduce two ways to do the same thing.

However, one benefit of this syntax would be to allow to have field punning
with type ascription. An example: `MyStruct { field : type Foo }`.

### Arrow, `->` syntax

[rust-lang/rust#50547]: https://github.com/rust-lang/rust/issues/50547

Another idea is to use an arrow syntax `expr -> type`. You'd then write:

```rust
let foo = (0..10).collect() -> Vec<_>;
```

[ViewPatterns]: https://ghc.haskell.org/trac/ghc/wiki/ViewPatterns

This can be read as "becomes Vec" or "leads to "Vec" which is not so bad.
However, it is as before also inconsistent syntax.
It has been noted on [the issue][rust-lang/rust#50547] that the `->` syntax
associates with callable things, which is misleading.
Finally, the syntax `->` conflicts in this case with [ViewPatterns],
which could be a useful extension to the pattern grammar.

### A macro

Another syntactic possibility is to use some sort of built-in macro solution.
For example, consider a post-fix macro:

```rust
let foo = (0..10).collect().at!(Vec<_>);
```

Beside the usual inconsistency, while this works well with method calls
and field projection, it also forces the user to wrap the type in parenthesis.

Furthermore, the method-like nature of a macro is probably sub-optimal for
ascription in pattern contexts.

### Inverted order: `$type op $expr`

One final idea for a syntax is to reverse the order of the type and the
expression in the ascription and to use a different binary operator.

For example:

```rust
let foo = Vec<_> of (0..10).collect(); // Using `of` as the operator.

let foo = usize ~ 123; // Using `~` as the operator.
```

This impetus for the reversed order comes from the observation that

```rust
do_stuff_with(try {
    if a_computation()? {
        b_computation()?
    } else {
        c_computation()?
    }
} : CarrierType);
```

does not read well. This RFC addresses this by allowing `try : C { .. }`.
The inverted order operator would handle this with `C of try { .. }` instead.

However, there are some notable problems with inverting the operator:

+ You can not use `:` as the operator. If you did, it would be confusing with
  the order in `let x: Type = ..;` and `fn foo(x: Type) ..`.

+ If a different token than `:` is used, the inverted order is still not
  consistent with let bindings and function definitions.

+ More often than not, the inverted order will cause the parser to backtrack
  because in most cases, there is not a type ascription, but the parser
  will start out assuming that there is.

+ The syntax does not work well with method chaining and field projection.
  If you consider rewriting the following chain:

  ```rust
  let _ = Box<_> of usize of "1234".parse().unwrap().into();
  ```

  There is no way for the parser to understand that it should be grouped as:

  ```rust
  let _ = Box<_> of (usize of "1234".parse().unwrap()).into();
  ```

  Furthermore, if we write a chain such as:

  ```rust
  let x = Rc<[_]> of (Vec<usize> of (Result<Vec<_>, _> of
    (0..10).map(some_computation).collect())
    .unwrap()
    .map(other_computation))
    .into();
  ```

  readability will likely suffer as the type annotation does not follow the
  flow of the reader and the annotation is not after each call.
  Even if this is formatted in the best possible way, it will not be as readable
  as with:

  ```rust
  let x = (0..10)
      .map(some_computation)
      .collect() : Result<Vec<_>, _>
      .unwrap()
      .map(other_computation) : Vec<usize>
      .into() : Rc<[_]>;
  ```

### Troubles with field punning

As we've previously noted in the [drawbacks], one disadvantage to the
currently proposed type ascription operator syntax is that it clashes
with field punning expressions and patterns. That is, if you say:
`MyStruct { field: Type }`, this is ambiguous with `MyStruct { field: binding }`.

Having said this, there are 3 chief ways to deal with this while retaining
`:` as a syntax:

1. Accept it and move on. This part of the language grammar will be somewhat
   unergonomic, but consistency and avoiding ad-hoc syntax is more important.
   This the proposed solution in this RFC.

2. Accept `MyStruct { field: Type }` where `Type` couldn't be a pattern.
   Examples of this include `MyStruct { field: Vec<Foo> }`.
   However, this is an ad-hoc syntax that is likely brittle.

3. Invent some ad-hoc disambiguation syntax. For example, we could entertain
   the syntax `MyStruct { (field: Type) }` which never parses today.
   While this could be made to work technically, it does not seem to carry
   its weight since we expect `MyStruct { field: Type }` to be somewhat rare.

## Precedence of the operator

As explained prior, we change the precedence of `:` when in an expression
context such that `x : T.foo()` is interpreted as `(x : T).foo()`.
This precedence change allow users to write readable code when they
have several method calls by using line-separation such as with:

```rust
let x = (0..10)
    .map(some_computation)
    .collect() : Foo
    .unwrap()
    .map(other_computation) : Bar
    .into() : Baz;
```

However, if you write this on a single line, or simply consider `x : T.foo()`
a user might parse this as `x : (T.foo())` instead.
While at this stage Rust does not support "type-level methods"
(meaning that this parse currently makes no sense),
a user may nonetheless make this mistake.

That said, it is still possible for the user to explicitly disambiguate with
`(x : T).foo()` wherefore this may not become a problem in practice.
The formatting tool `rustfmt` may also apply such stylings automatically.
It is important that we gain experience during the stabilization period
of this RFC and apply sensible formatting rules such that type ascription
stays readable.

Speaking of type level methods, it might,
someday be the case that we would want to permit something such as:

```rust
impl type {
    fn foo(self: type) -> type {
        match self {
            bool => usize,
            _ => Vec<usize>,
        }
    }
}
```

However, we believe this to be quite unlikely at this point.
In particular, while it may make sense to have free type level functions,
this method variant could only exist in the core library.
All in all, the prospect of adding such type level methods should not
keep us from making this precedence change.

# Prior art
[prior-art]: #prior-art

## Haskell

In Haskell it possible to type ascribe an expression like so
(here using the REPL `ghci`):

```haskell
ghci> 1 + 1 :: Int -- Type ascribing 1 + 1 to the type Int.
2

ghci> 1 + 1 :: Bool -- And to Bool, which is wrong.

<interactive>:4:1: error:
    • No instance for (Num Bool) arising from a use of ‘+’
    • In the expression: 1 + 1 :: Bool
      In an equation for ‘it’: it = 1 + 1 :: Bool
```

It should be noted that Haskell, just like Rust, allows a user to apply types
to a polymorphic function explicitly:

```haskell
{-# LANGUAGE TypeApplications #-}

id :: forall a. a -> a
id x = x

foo = id @Int 1 -- We apply Int to the type variable 'a' above.
```

This would correspond roughly to:

```rust
fn id<T>(x: T) -> T { x }

let foo = id::<i32>(1);
```

Note in particular here that the Haskell version uses the same token for
annotating the function signature and for ascribing types on expressions.

As with this RFC, you can also type ascribe inside patterns in Haskell:

```haskell
ghci> :set -XScopedTypeVariables
ghci> foo (x :: Int, y :: Bool) = if y then x + 1 else x - 1
ghci> :t foo
foo :: (Int, Bool) -> Int
```

## PureScript

[PureScript]: https://github.com/purescript/documentation/blob/master/language/Types.md#type-annotations

Being a dialect of Haskell, [PureScript] also allow users to ascribe expressions.

## Idris

Idris annotates its function definitions like so:

```idris
id : a -> a
id x = x
```

However, Idris does not have a built-in mechanism to type-ascribe expressions.
Instead, you use the library defined function `the`:

```idris
the : (a : Type) -> (value : a) -> a
the _ = id
```

You may then write `the Nat x` for the equivalent of `x : Nat`.

## Scala

[scala_annot]: https://docs.scala-lang.org/style/types.html#annotations
[scala_ascribe]: https://docs.scala-lang.org/style/types.html#ascription

Scala supports both what it calls ["type annotations"][scala_annot] and
["type ascription"][scala_ascribe].
 
For example, you may write (type annotation):

```scala
val s = "Alan": String
```

You may also write (type ascription, upcasting):

```scala
val s = s: Object
```

Note in particular that Scala does take sub-typing (of a different kind) into
account in this syntax.

## F*

[fstar]: https://www.fstar-lang.org/
[fstar_ascribe]: https://github.com/FStarLang/FStar/wiki/F*-symbols-reference

[F*][fstar] allows users to [type ascribe][fstar_ascribe] using the symbol `<:`.
For example:

```ocaml
module Ascribe

val x : string
let x = "foo" <: string
```

## Standard ML

[sml97-defn]: http://sml-family.org/sml97-defn.pdf
[intro_ml]: https://courses.cs.washington.edu/courses/cse341/04wi/lectures/02-ml-intro.html

Standard ML as defined by [its specification][sml97-defn] has the following
alternatives in its pattern and expression grammar:

```
exp : ... | exp ':' ty | ... ;

pat : ... | pat ':' ty | ... ;
```

You may therefore for [example][intro_ml] write:

```sml
val x = 3 : int
```

Note that this is exactly the same grammar as we've proposed here.

# Unresolved questions
[unresolved]: #unresolved-questions

None.

# Possible future work
[possible future work]: #possible-future-work

In previous versions of this RFCs some features were proposed including:

- Block ascription syntax; e.g. `async: Type { ... }` or `try: Type { ... }`.
- Making the syntax of function parameters into `fn name(pat0, pat1, ..)`
  rather than `fn name(pat0: type0, pat1: type1, ..)`.

These have since been removed from this particular RFC and will be proposed
separately instead.
