- Feature Name: `generalized_type_ascription`
- Start Date: 2018-08-06
- RFC PR: _
- Rust Issue: _

# Summary
[summary]: #summary

[RFC 803]: https://github.com/rust-lang/rfcs/blob/master/text/0803-type-ascription.md

This RFC supersedes and subsumes [RFC 803].
We finalize a general notion of type ascription uniformly in patterns,
expressions, `let` bindings, and `fn` definitions. You may now for example write:

```rust
let x = (0..10).collect() : Vec<_>;

do_stuff(try : Option<u8> { .. });

do_stuff(async : u8 { .. });

do_stuff(unsafe : u8 { .. });

do_stuff(loop : u8 { .. });

let alpha: u8 = expr;
    ^^^^^^^^^

let [x: u8, y, z] = stuff();
    ^^^^^^^^^^^^^

if let Some(beta: u8) = expr { .. }
            ^^^^^^^^

fn foo(Wrapping(alpha: usize)) {}
       ^^^^^^^^^^^^^^^^^^^^^^
```

Here, the underlined bits are patterns.
Note however that this RFC does *not* introduce global type inference.

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

4. When you want to pass an expression such as `try { .. }` or
   `async { .. }` to a function which expects a generic `R: Try` or `R: Future`,
   type inference can fail. In this case,
   it is more ergonomic to type-ascribe with `try : R { .. }` instead
   of first introducing an artificial binding.

5. Turbofish is not always possible! Consider for example:

   ```rust
   fn display_all(elems: impl Iterator<Item: Display>) { ... }
   ```

   As of today (2018-08-06), it is not possible to use turbofish at all
   as you could have done had you instead written:

   ```rust
   fn display_all<I: Iterator<Item: Display>>(elems: I) { ... }
   ```

   While this may change in the future, it may also be the case that anonymous
   `arg: impl Trait`s can't ever be turbofished. In such a case, type ascription
   is our saving grace; it works independently of how `display_all` is defined
   and let's us easily constrain the type of `elems` in a syntactically
   light-weight way in any case.

6. Type ascription is helpful when doing *type* driven development
   and opens up more possibilities to move in the direction of
   interactive development as is possible with [agda-mode] and [idris-mode].

[agda-mode]: http://agda.readthedocs.io/en/v2.5.2/tools/emacs-mode.html
[idris-mode]: https://github.com/idris-hackers/idris-mode

## Type ascription has already been accepted as an RFC

We noted previously that [RFC 803] already accepted type ascription in
expression contexts. Thus, we have already collectively deemed to some extent
that ascription is something we want. However, the previous RFC did not apply
ascription uniformly. We believe it is beneficial to do so. We also believe
that much of the motivation for accepting RFC 803 applies to the extensions
proposed here.

## More DRY code

By introducing type ascription as a pattern, we can simplify function definitions
to permit the following:

```rust
fn take_wrapping(Wrapping(count: usize)) -> R { .. }
```

instead of the following:

```rust
fn take_wrapping(Wrapping(count: usize): Wrapping<usize>) -> R { .. }
```

Compared to type ascription in expression contexts or using `let` bindings
we can also get away with leaving more inferred when pattern matching.
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

In the last case, the typing annotation is both *most local* and also does not
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

## Uniform Syntax and Unified Mental Model

Given the changes in this RFC, note that when you write:

```rust
fn frobnicate(alpha: Beta) -> Gamma { .. }
              ^^^^^^^^^^^
              A pattern!
```

the underlined part is a pattern.
The same applies to `let` bindings. When you wrote:

```rust
let alpha: Beta = gamma;
    ^^^^^^^^^^^
    A pattern!
```

before this RFC, it was the case that `alpha: Beta` in function definitions
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
We also allow `async : T { .. }` and `try : T { .. }` which we argue is both
as a useful shorthand and feels natural with type ascription in expression
contexts.

[parser-lalr.y]: https://github.com/rust-lang/rust/blob/9b5859aea199d5f34a4d4b5ae7112c5c41f3b242/src/grammar/parser-lalr.y#L722-L827

Another thing to note is that grammar changes described in the [summary]
above replace most of the productions listed in the highlighted section and
other parts of the slightly outdated [parser-lalr.y] file with something less
complicated and smaller.

## Future proofing for DRY in trait implementations

Finally, by using the syntactic category of `pat` in the context of
function parameters, we already have the grammatical means to elide types in
implementations of traits. All that is required now is to employ the rule that
types can be omitted from parameters if and only if the types are fully determined.

However, to avoid doing too much in this RFC, we defer this to another
RFC and simply say for now, that the compiler will always require the full
signature in trait implementations. See the section on [possible future work]
for more details.

## Paving the way for better error messages

By making the fundamental unit of a function parameter be a pattern,
it becomes technically feasible to improve error messages such that
when you write `fn foo(x, y) { ... }`, the body can be analysed by the compiler.
It could then, at the compiler implementations option, give a user a help
message which provides the types of `x` and `y`.

We believe that this provides a sweet-spot between global type inference and
the absence of it. This way, the compiler will reject the code, but a structured
and easily applied fix is provided for you.

## Motivation for `async / try / ... : Type { .. }`

[RFC 2394]: https://github.com/rust-lang/rfcs/pull/2394
[RFC 2388]: https://github.com/rust-lang/rfcs/pull/2388
[RFC 243]: https://github.com/rust-lang/rfcs/pull/243
[niko_try_1]: https://github.com/rust-lang/rfcs/pull/2388#issuecomment-378750364

[RFC 2394] introduced `async { .. }` blocks and [RFC 2388] renamed the
previously, by [RFC 243], introduced `catch { .. }` blocks to `try { .. }`.

In RFC 2388, [some opined][niko_try_1] that it might be good idea to make it
mandatory to specify the type of a `try { .. }` expression.
The RFC did not end up proposing such a mandatory mechanism.
However, @cramertj then noted a concern (which was eventually resolved) that:
> As @clarcharr and @nikomatsakis
> [discussed above](https://github.com/rust-lang/rfcs/pull/2388#issuecomment-378750364),
> it's nearly always necessary to manually specify the error type for these
> blocks because of the `Into` conversion that `?` does.
> @nikomatsakis mentioned that we might even want a syntax which *requires*
> users to explicitly state the error type
> (or the full result type, for compatibility with `Option`, `Poll`, etc.). [...]

Since the language already has expression level type ascription, it is already
possible to constrain the carrier type of a `try { .. }` with `try { .. } : C`
where `C` is the carrier type (such as `Result<T, E>`).
However, consider a situation where this `try { .. }` expression is passed
to some generic function. In that case, and especially if the `try { .. }`
expression spans several lines, the type ascription at the end might not
read very well:

```rust
do_stuff_with(try {
    if a_computation()? {
        b_computation()?
    } else {
        c_computation()?
    }
} : CarrierType);
```

This example has two primary problems:

1. It does not format well.
2. The choice of `CarrierType` affects the dynamic semantics of the `try { .. }`
   block. Thus, the information that the `try { .. }` block is of type
   `CarrierType` may come too late. Therefore, the user may have to backtrack
   in reading. This in turn negatively affects the speed with which the code
   may be read.

By allowing the user to type-ascribe the carrier type up-front as in the
following snippet, we improve on both points:

```rust
do_stuff_with(try: CarrierType {
    if a_computation()? {
        b_computation()?
    } else {
        c_computation()?
    }
});
```

[async_nemo157_1]: https://github.com/rust-lang/rust/issues/50547#issuecomment-408169261

Similarly, for the `async { .. }` block due to [RFC 2394],
it was [noted][async_nemo157_1] on the tracking issue that:

> @cramertj
> food for thought: i'm often writing `Ok::<(), MyErrorType>(())` at the end of
> `async { ... }` blocks. perhaps there's something we can come up with to make constraining the error type easier?
>
> @withoutboats
> [...] possibly we want it to be consistent with [`try`]?

We agree. In fact, the same 2 problems above occur for `async { .. }` as well.
Thus, for the sake of consistency and uniform syntax as well as solving the
same set of problems we propose that `async : R { .. }` be permitted.

We also take the opportunity to note that there also exists other block forms
in Rust which adhere to the `keyword block` grammar. These forms are:

+ `unsafe { .. }`
+ `loop { .. }`

To further syntactic uniformity we thus extend these forms to optionally permit
`: Type` so that you may write:

+ `unsafe : R { .. }`
+ `loop : R { .. }`

In the future, it may be possible that we introduce new block forms such as

+ `const { .. }`

In that case, it will probably be a good idea to permit optional type ascription
in the same way.

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

    let _ = ("1234".parse().unwrap() : usize).into() : Box<_>;
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

### In `async`, `try`, ... blocks

[RFC 2388]: https://github.com/rust-lang/rfcs/pull/2388
[RFC 243]: https://github.com/rust-lang/rfcs/pull/243

[RFC 243] introduced `catch { .. }` blocks to Rust. Then [RFC 2388] came along
and renamed these blocks to `try { .. }`. In this RFC, we propose that you should
be allowed to optionally type-ascribe the block form in an ergonomic manner:

```rust
try : MyResult<T, E> {
    // The logic...
}
```

This snippet is equivalent in all respects to:

```rust
try {
    // The logic...
} : MyResult<T, E>
```

The same applies to the `unsafe` and `loop` constructs as well so we may write:

```rust
let x = loop : usize {
    break 1;
};

let x = unsafe : usize {
    // Some unsafe logic...
};
```

and the snippet is equivalent to:

```rust
let x = loop {
    break 1;
} : usize;

let x = unsafe {
    // Some unsafe logic...
} : usize;
```

[RFC 2394]: https://github.com/rust-lang/rfcs/pull/2394

Finally, in [RFC 2394] `async { .. }` blocks were introduced to the language.
You may type ascribe these blocks in the same way as above:

```rust
let future = async : io::Result<()> {
    ...
};
```

Do note however that in this case, you are annotating the inner type of the
resulting future and not the future itself. Thus, this is equivalent to:

```rust
let future = async {
    ...
} : impl Future<Output = io::Result<()>>;
```

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

### Function definitions

Another change that comes with this RFC is that the fundamental unit of a
function parameter also becomes a pattern. Thus when you write things like:

```rust
fn foo(alpha: usize, beta: bool) { ... }
       ^^^^^^^^^^^^  ^^^^^^^^^^^
       Pattern       Pattern
```

the underlined parts are the patterns while before this RFC, the patterns were:

```rust
fn foo(alpha: usize, beta: bool) { ... }
       ^^^^^         ^^^^
       Pattern       Pattern
```

Indeed, with the change to function definitions, it becomes
*syntactically valid* to write:

```rust
fn foo(alpha, beta) -> usize { .. }
```

Nevertheless, as we want to avoid introducing global type inference to the
language, the type checker will prevent this from compiling and will emit an
error which *may* look like:

```rust
error[E0282]: type annotations needed
 --> src/main.rs:1:9
  |
1 |     fn foo(alpha, beta) -> usize { .. }
  |            ^^^^^  ^^^^
  |            |      |
  |            |______|
  |            |
  |            The following patterns do not have a fully determined type.
  |            help: Write type annotations on the following patterns:
  |
  |                alpha: usize
  |                beta: bool
  |

error: aborting due to previous error
```

This also gives the compiler an opportunity to tell you what the types are
if it so happens that you need this help. Providing the user with this
type information is not mandatory and is instead up to the implementation
of the compiler.

However, in some cases, you can determine the type from the pattern alone.
Therefore, when the type is fully determined you may omit the type ascription.
Thus, the following definitions are legal:

```rust
fn foo<T>(Wrapping(value: T)) -> usize { ... }

fn bar(Wrapping(value: usize)) -> usize { ... }

struct Quux {
    field: usize
}

// There are no generics here:
fn baz(Quux { field }) -> usize { ... }

struct Wobble(bool);

// Also fully determined here:
fn baz(Wobble(x)) -> usize { ... }
```

However, the following definition is not OK:

```rust
// Unconstrained type variable ?T of Wrapping<?T>:
fn foo(Wrapping(value)) -> usize { ... }
```

By using this mechanism, we gain a measure of elision and can make writing
more ergonomic.

[RFC 1685]: https://github.com/rust-lang/rfcs/blob/master/text/1685-deprecate-anonymous-parameters.md

#### [RFC 1685] and deprecation schedule

In current Rust, the type checker will accept a method, in a trait definition,
which has a parameter that only specifies a type but not a pattern (example below).
The accepted RFC 1685 proposed that we deprecate this ability such that you
*must* provide a pattern / parameter name. However, the RFC left the deprecation
strategy and the schedule unresolved.

Since we want the ability to view function parameters uniformly as patterns
and extend them to trait definitions:

```rust
trait MyTrait {
    fn do_stuff(Wrapping(x: usize)) {
        // Provided logic...
    }
}
```

we move up on the deprecation schedule of [RFC 1685] and propose that
writing:

```rust
trait Foo {
    fn bar(MyType) -> ... { ... }
}
```

will cause the compiler to emit a warn-by-default lint in Rust 2015 and
that it be a hard error in Rust 2018. This resolves the unresolved question
in [RFC 1685].

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

Assuming the following tokens and the terminal strings they lex:

```rust
LOOP : 'loop' ;
UNSAFE : 'unsafe' ;
TRY : 'try' ;
ASYNC : 'async' ;
LET : 'let' ;
SELF : 'self' ;
DOTDOTDOT : '...' ;
```

The following alternatives are modified in the expression grammar:

```rust
ascribe: ':' ty_sum ;

expr
: ...
| expr ascribe // This is specified in RFC 803 but it is included for completeness.
| LOOP ascribe? block // This replaces the existing production for loop { .. }.
| UNSAFE ascribe? block
| TRY ascribe? block
| ASYNC ascribe? block
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

Finally, the grammar of function definitions is changed:

```rust
// As before this RFC:
fn_decl : fn_params ret_ty ; 

fn_decl_with_self : fn_params_with_self ret_ty ;

// Changed in this RFC:
fn_anon_params_with_self : '(' fn_anon_params_with_self_params? ')' ;

fn_params : '(' params? ')' ;

fn_params_with_self : '(' self_param ((',' params)* ','?)? ')' : fn_params ;

fn_anon_params_with_self_params
: self_param (',' ty_or_pat)* ','?
| ty_or_pat (',' ty_or_pat)* (',' va_tail)?
;

params : pat (',' pat)* (',' va_tail)?;

ty_or_pat
: ty   // Warning in Rust 2015. Forbidden in Rust 2018!
| pat  // Note! -- the order between `ty` and `pat` is important!
;

self_param : ('&' lifetime?)? maybe_mut SELF maybe_ty_ascription ;
va_tail : (pat ':')? DOTDOTDOT | %empty ; // Needed for RFC 2137.
```

### [RFC 1685] and deprecation schedule

The schedule of linting against writing:

```rust
trait Foo {
    fn bar(Type) { ... }
}
```

is currently to warn in Rust 2018 and then transition to a hard error the
next edition. This RFC moves up the schedule and makes it a warning in
Rust 2015 and a hard error in Rust 2018.

Thus, in Rust 2018, the production `ty_or_pat` is defined as just:

```rust
ty_or_pat : pat ;
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

In pattern contexts, wrapping in parenthsis is made valid by
[rust-lang/rust#48500](https://github.com/rust-lang/rust/pull/48500).

The tool `rustfmt` will similarly prefer the latter formatting.

## Desugaring

A Rust compiler will desugar, where `$ty` is a meta variable for a type and
where `$body` is some block body, the following:

```rust
loop : $ty { $body }

unsafe : $ty { $body }

try : $ty { $body }

async : $ty { $body }
```

into:

```rust
loop { $body } : $ty

unsafe { $body } : $ty

try { $body } : $ty

async { $body } : impl Future<Output = $ty>
```

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

To stay compatible with this and avoid breaking changes, this behavior is preserved.

When type checking an expression against a pattern where the pattern includes
a type ascription of form `pat : type`, the compiler will ensure that the
expression fragment corresponding to the ascribed pattern `pat` is implicitly
coercible (including sub-typing) to the `type` ascribed.

As for the operational semantics, if type of the expression fragment and
the ascribed-to type are an exact match, then type ascription is a no-op.
Otherwise, the semantics are those of the implicit coercion.

#### Function definitions

A type of a formal parameter of a function is allowed to be elided and inferred
if and only if the type of the formal parameter is fully determined
(has no unification variables left) by looking solely at the signature of the
function, including patterns, quantified variables, and `where` clauses.

We impose the following rules:

1. The type checker is not allowed to look at the function body and neither is
   it permitted to take into account the type information implied by a trait
   implementation being well-formed.

2. With respect to lifetime and type parameters, none will be added due
   to type inference. However, the compiler will generate distinct lifetimes
   for each lifetime position in the type portion, if such exist, of a pattern.
   This applies recursively in each parameter.

   Lifetime positions are:
   1. Anywhere `'_` is found.
   2. Anywhere `&[mut] $type` is found.

   [RFC 2115]: https://github.com/rust-lang/rfcs/blob/master/text/2115-argument-lifetimes.md#the-wildcard-lifetime

   Finally, due to backwards compatibility with Rust 2015,
   for a parameter which has a pattern that matches `$pat : $type`,
   if the type definition corresponding to `$type` has lifetimes which are not
   specified in `$type`, those are also added and a deprecation warning due
   to [RFC 2115] is emitted.

3. If the pattern of a parameter has `&[mut] pat` patterns anywhere,
   including recursively, then the full type of the top level pattern
   must be specified. In specifying the full type, lifetimes may be elided.

Given the following type definitions:

```rust
struct Foo(usize);

struct Wrapping<T>(T);

struct Product<A, B>(A, B);

trait Trait { type Assoc; }

struct Bar<'a>(&'a bool);

struct Wibble<'a>(&'a i32, Bar<'a>);

struct Quux<'a, 'b: 'a>(&'a Foo, Bar<'b>);

struct Beta<'a>(&'a Bar<'a>);
```

The following example definitions are accepted:

```rust
// 1) No lifetime positions, so none are generated.
// 2) Looking at `x: usize` we determine that `x : usize`.
// -----------------------------------------------------------------------------
// + `good_0 : fn(usize) -> ()`.
// + `x : usize`.
fn good_0(x: usize) {}

// 1) We find the in-band lifetime `'a` and add it as an input lifetime parameter.
// 2) Looking at `x: usize` we determine that `x : &'a usize`.
// -----------------------------------------------------------------------------
// + `good_1 : for<'a> fn(&'a usize) -> ()`.
// + `x : &'a usize`.
fn good_1(x: &'a usize) {}

// Nothing to infer, everything is explicitly quantified and specified.
// -----------------------------------------------------------------------------
// + `good_2 : for<'a> fn(&'a usize) -> ()`.
// + `x : &'a usize`
fn good_2<'a>(x: &'a usize) {}

// 1) We find one lifetime position, we introduce and substitute `'a`
//    for it and add it as an input lifetime parameter.
// 2) The pattern is now `&x: &'a usize`. The full type is known.
// 3) Since `&x: &'a usize`, then `x: usize`.
// -----------------------------------------------------------------------------
// + `good_3 : for<'a> fn(&'a usize) -> ()`.
// + `x : usize`.
fn good_3(&x: &usize) {}

// 1) No lifetime positions.
// 2) Looking at `ref x: usize`, the type of the parameter is `usize`.
// -----------------------------------------------------------------------------
// + `good_4 : for fn(usize) -> ()`.
// + `x : &'<tmp> usize`.
fn good_4(ref x: usize) {}

// 1) No lifetime positions.
// 2) Looking at `Foo($pat)` we know that `parameter : Foo` and that `$pat: usize`.
// 3) `$pat = x`, thus `x: usize`.
// -----------------------------------------------------------------------------
// + `good_5 : fn(Foo) -> ()`.
// + `x : usize`.
fn good_5(Foo(x)) {}

// 1) Looking at `x: Bar`, we apply the back-compat rule and find one lifetime
//    parameter `'a` which we quantify.
// 2) We emit a warning.
// -----------------------------------------------------------------------------
// + `good_6 : for<'a> fn(Bar<'a>) -> ()`.
// + `x : usize`.
fn good_6(x: Bar) {}

// 1) Looking at `Foo(x): &_` we find one input lifetime position and quantify
//    it as `'a`. We substitute the pattern for `Foo(x): &'a _`.
// 2) Seeing `Foo(x): &'a _`, we substitute `_` for a new
//    unification variable `?T`. We now have `Foo(x): &'a ?T`.
// 3) Seeing `Foo(x)` we know that `?T = Foo` and substitute to it.
//    We know that `parameter : &'a Foo`.
// 4) We know that `x : &'a usize` due to the match mode.
// -----------------------------------------------------------------------------
// + `good_7 : for<'a> fn(&'a Foo) -> ()`.
// + `x : &'a usize`.
fn good_7(Foo(x): &_) {}

// 1) We find one lifetime position, quantify it as `'a`.
//    We now have `Foo(ref): &'a _`.
// 2) Seeing that, we substitute `_` for unification variable `?T`.
//    We now have `Foo(ref): &'a ?T`.
// 3) Seeing `Foo(x)` we know that `?T = Foo`. Thus `parameter : &'a Foo`.
// 4) Seeing `ref x`, we know that `x : &'a usize`.
// -----------------------------------------------------------------------------
// + `good_8 : for<'a> fn(&'a Foo) -> ()`.
// + `x : &'a usize`.
fn good_8(Foo(ref x): &_) {}

// 1) No lifetime positions.
// 2) Looking at `Foo($pat)` we know that `parameter : Foo`
//    and that `$pat: usize`.
// 2) Looking at `$pat = ref x` we know that `ref x: usize`.
//    We know therefore that `x: &'<tmp> usize`.
// -----------------------------------------------------------------------------
// + `good_9 : fn(Foo) -> ()`.
// + `x : &'<tmp> usize`.
fn good_9(Foo(ref x)) {}

// 1) No lifetime positions.
// 2) Seeing an array with 3 elements, we know that `parameter : [?T; 3]`
//    for some `?T` type.
// 3) Seeing `a: u8`, we know that `?T = u8` and that `a: u8`.
// 4) Seeing `b`, we know that because `parameter : [u8; 3]` that `b: u8`.
// 5) Seeing `ref c`, we know that `c: &'<tmp> u8`.
// -----------------------------------------------------------------------------
// + `good_10 : fn([u8; 3]) -> ()`.
// + `a : u8`.
// + `b : u8`.
// + `c : &'<tmp> u8`.
fn good_10([a: u8, b, ref c]) {}

// 1) No lifetime positions.
// 2) Same as `good_10`.
// 3) Seeing `a: impl Debug` we quantify a type parameter `T: Debug` and
//    substitute `?T` for `T`.
//    We mark the function as not turbo-fishable.
// 4) Seeing `a` and because `parameter: [T; 3]` we know that `a: T`.
//    Repeat for `b`.
// -----------------------------------------------------------------------------
// + `good_11 : for<T: Debug> fn([T; 3]) -> ()`.
// + `a : T`.
// + `b : T`.
// + `c : T`.
fn good_11([a: impl Debug, b, c]) {}

// 1) No lifetime positions.
// 2) Seeing `Wrapping($pat)`, we know that `parameter : Wrapping<?T>`.
// 3) Seeing `$pat = usize`, we know that:
//    a) `x : usize`
//    b) `?T = usize`.
//    c) `parameter : Wrapping<usize>`.
// -----------------------------------------------------------------------------
// + `good_13 : fn(Wrapping<usize>) -> ()`.
// + `x : usize`
fn good_13(Wrapping(x: usize)) {}

// 1) No lifetime positions.
// 2) Seeing `Wrapping($pat)`, we know that `parameter : Wrapping<?T>`.
// 3) Seeing `$pat = x: T`, we know that:
//    a) `x : T`.
//    b) `parameter : Wrapping<T>`.
// -----------------------------------------------------------------------------
// + `good_14 : for<T> fn(Wrapping<T>) -> ()`.
// + `x : T`.
fn good_14<T>(Wrapping(x: T)) {}

// 1) No lifetime positions.
// 2) Seeing `Wrapping($pat)`, we know that `parameter : Wrapping<?T>`.
// 3) Seeing `$pat = x: impl Display`, we quantify a type parameter `T: Display`
//    We substitute => `$pat = x: T`.
//    We mark the function as not turbo-fishable.
// 4) Seeing `$pat = x: T`, we know that:
//    a) `x : T`.
//    b) `?T = T`.
//    c) `parameter : Wrapping<T>`.
// -----------------------------------------------------------------------------
// + `good_15 : for<T: Display> fn(Wrapping<T>) -> ()`.
// + `x : T`.
fn good_15(Wrapping(x: impl Display)) {}

// 1) No lifetime positions.
// 2) Seeing `Wrapping($pat)`, we know that `parameter : Wrapping<?T>`.
// 3) Looking at `$pat = x: <X as Trait>::Assoc`, we:
//    a) Verify that `X: Trait` and that `Trait` has an associated type `Assoc`.
//    b) Substitute `?T` for `<X as Trait>::Assoc`.
// -----------------------------------------------------------------------------
// + `good_16 : for<X: Trait> fn(Wrapping<X::Assoc>) -> ()`.
// + `x : <X as Trait>::Assoc`.
fn good_16<X: Trait>(Wrapping(x: <X as Trait>::Assoc)) {}

// 1) We find `Bar<'_>`. We substitute `'_` for a new lifetime `'a`.
// 2) Seeing `Wrapping($pat)`, we know that `parameter : Wrapping<?T>`.
// 3) Seeing `$pat = x: Bar<'a>`, we know that:
//    a) `x : Bar<'a>`.
//    b) `?T = Bar<'a>`.
//    c) `parameter : Wrapping<Bar<'a>>`.
// -----------------------------------------------------------------------------
// + `good_17 : for<'a> fn(Wrapping<Bar<'a>>) -> ()`.
// + `x : Bar<'a>`.
fn good_17(Wrapping(x: Bar<'_>)) {}

// 1) We find `&'_ i32`. We substitute `'_` for a new lifetime `'a`.
// 2) We find `Bar<'_>`. We substitute `'_` for a new lifetime `'b`.
// 3) Seeing `Product($pat_1, $pat_2)`, we know `parameter : Product<?T, ?U>`.
// 4) Seeing `$pat_1 = x: &'a i32`, we know that:
//    a) `x : &'a i32`.
//    b) `?T = &'a i32`.
// 5) Seeing `$pat_2 = y: Bar<'b>`, we know that:
//    a) `y : Bar<'b>`.
//    b) `?T = Bar<'b>`.
// -----------------------------------------------------------------------------
// + `good_18 : for<'a, 'b> fn(Product<&'a i32, Bar<'b>>) -> ()`.
// + `x : &'a i32`.
// + `y : Bar<'b>`.
fn good_18(Product(x: &i32, y: Bar<'_>)) {}

impl Foo {
    // 1) Seeing &self, we assign all lifetimes in the output the lifetime `'self`.
    // 2) We find 3 lifetime positions:
    //    a) `&'_ u8`
    //    b) `&'_ u16`
    //    c) `&'_ u32`
    //    We quantify a lifetime for each.
    // 3) Seeing `Product($pat_1, $pat_2)` we know that:
    //    a) `parameter : Product<?T, ?U>`.
    // 4) Seeing `$pat_1 = x: &'a u8`, we know that:
    //    a) `x : &'a u8`.
    //    a) `?T = &'a u8`.
    // 5) Seeing `$pat_2 = Product($pat_3, $pat_4)` we know that:
    //    a) `?U = Product<?V, ?X>`
    // 6) Repeat 4) for `$pat_3` and `$pat_4`.
    // -------------------------------------------------------------------------
    // + `good19 : for<'self, 'a, 'b, 'c>
    //     fn(
    //         &'self Foo,
    //         Product<
    //             &'a u8,
    //             Product<
    //                 &'b u16,
    //                 &'c u32
    //             >
    //         >
    //     ) -> &'self str`.
    // + `x : &'a u8`.
    // + `y : &'b u16`.
    // + `z : &'c u32`.
    fn good_19(&self, Product(x: &u8, Product(y: &u16, z: &u32))) -> &str { .. }
}

// 1) We quantify the in-band lifetime `'a`.
// 2) Looking at `Wibble($pat_1, $pat_2)`, we know that:
//    a) `parameter : Wibble<'?t>`.
//    b) `$pat_1 : &'?t i32`.
//    c) `$pat_2 : Bar<'?t>`.
// 3) Looking at `$pat_1 : &'?t i32 = x: &'a i32`, we know that:
//    b) `x : &'a i32`.
//    c) `'?t = 'a`.
// 4) Looking at `$pat_2 : Bar<'?t> = y: Bar<'a>`, we know that:
//    b) `y : Bar<'a>`.
//    c) `'?t = 'a`. This was already solved in  3).
// -----------------------------------------------------------------------------
// + `good_20 : for<'a> fn(Wibble<'a>) -> ()`.
// + `x : &'a i32`.
// + `y : Bar<'a>`.
fn good_20(Wibble(x: &'a i32, y: Bar<'a>)) {}

// 1) We quantify in-band lifetimes `'a` and `'b`.
// 2) Seeing `Quux($pat_1, $pat_2)`, we know that:
//    a) `parameter : Quux<'?t, '?u>` where `'?u: '?t`.
//    b) `$pat_1 : &'?t Foo`.
//    c) `$pat_2 : Bar<'?u>`.
// 3) Seeing `$pat_1 : &'?t Foo = x: &'a Foo`, we know that:
//    a) `x : &'a Foo`.
//    b) `'?t = 'a`.
// 4) Seeing `$pat_2 : &'?u Foo = Bar($pat_3)`, we know that:
//    a) `$pat_3 : &'?u bool`
// 5) Seeing `$pat_3 : &'?u bool = y: &'b bool`, we know that:
//    a) `y : &'b bool`.
//    a) `'?u = 'b`.
// -----------------------------------------------------------------------------
// + `good_21 : for<'a, 'b: 'a> fn(Quux<'a, 'b>) -> ()`.
fn good_21(Quux(x: &'a Foo, Bar(y: &'b bool))) {}

// 1) We find `&'_ usize`. We substitute `'_` for a new lifetime `'a`.
// 2) Looking at `Wrapping($pat)`, we know that:
//    a) `parameter : Wrapping<?T>`.
//    b) `$pat : ?T`.
// 3) Looking at `$pat : ?T = x: &'a usize`, we know that:
//    a) `x : &'a usize`.
//    b) `?T = &'a usize`.
//    c) `parameter : Wrapping<&'a usize>`.
// -----------------------------------------------------------------------------
// + `good_22 : for<'a> fn(Wrapping<&'a usize>) -> ()`.
// + `x : &'a usize'.
fn good_22(Wrapping(x: &usize)) {}

// 1) We find `&'_ usize`. We substitute `'_` for a new lifetime `'a`.
// 2) We find `&'_ _`. We substitute `'_` for a new lifetime `'b`.
// 3) Looking at `$pat_1 : &'b _`,
//    we substitute `_` for a new unification variable `?T` where `?T: 'b`.
//    We know that: `parameter = $pat_1 : &'b ?T`.
//    We enter a reference match binding mode.
// 4) Looking at `$pat_1 : &'b ?T = Wrapping($pat_2)`, we know that:
//    a) `?T = Wrapping<?U>`.
//    b) `$pat_2 : ?U`.
//    c) `?U: 'b`.
// 5) Looking at `$pat_2 : ?U = x : &'a usize`, we know that:
//    a) `x : &'a usize`, `'b: 'a`, and `?U = usize` due to match mode.
//    d) `parameter : &'b Wrapping<usize>`
// 6) because `'a: 'b`, `'b: 'a`, we substitute `'b` for `'a` and remove `'b`.
// -----------------------------------------------------------------------------
// + `good_23 : for<'a> fn(&'a Wrapping<usize>) -> ()`.
// + `x : &'a usize'.
fn good_23(Wrapping(x: &usize): &_) {}

// 1) We find `&'_ T`. We substitute `'_` for a new lifetime `'a`.
//    We also conclude `T: 'a`.
// 2) Looking at `Wrapping($pat_1)`, we know that:
//    a) `parameter : Wrapping<?T>`.
//    b) `$pat_1 : ?T`.
// 3) Looking at `$pat_1 : ?T = x: &'a T`, we know that:
//    a) `?T = &'a T`.
//    b) `x : &'a T`.
// -----------------------------------------------------------------------------
// + `good_24 : for<'a, T: 'a> fn(Wrapping<&'a T>) -> ()`.
// + `x : &'a T'.
fn good_24<T>(Wrapping(x: &T)) {}
```

But the following definitions are rejected:

```rust
// The type of `x` is fully ambiguous even if we look at the body.
// The type of `bad_0: fn(?T) -> ()`.
fn bad_0(x) {}

// The compiler has to look at the body to see that `x: u8`:
// The type of `bad_1: fn(?T) -> ()`.
fn bad_1(x) {
    let y: u8 = x;
}

// There is an unconstrained unification variable `?T` from `Wrapping<?T>`.
// The type of `bad_2: fn(Wrapping<?T>) -> ()`
fn bad_2(Wrapping(x)) {}

// No lifetime parameter added for `Quux<'?>` to unify with.
fn bad_3(Quux(x, Bar(y))) {}

// Inferred `x: &'a`, `y: &'b Bar<'b>` but `'a != 'b` which `Wibble<'?>` requires.
fn bad_4(Wibble(x: &i32, y: Bar<'_>)) {}

// Rejected because parameter has & but not fully spec type.
fn bad_6(&(x: usize)) {}

// Rejected because parameter has & but not fully spec type.
fn bad_7(&[a: u8, ref b]) {}

// Rejected because parameter has & but not fully spec type.
fn bad_8(Beta(&Bar(&x))) {}

// The two separate `impl Trait`s get each one type parameter
// T and U leading to: [x: T, y: U] which is not well formed.
fn bad_9([x: impl Trait, y: impl Trait]) {}

impl From<usize> for Foo {
    // The compiler is not allowed to look at `From<usize>` to
    // understand that `x: usize`.
    fn from(x) -> Self { Self(x) }
}
```

#### Optional: improved error messages

Considering the rejected example function `bad_1`, a Rust compiler,
knowing that the `typeof(x) = u8` by looking at the body
(if such analysis is performed),
can emit an error message with the type identity of `x` in it.
An example error message is:

```rust
error[E0282]: type annotations needed
 --> src/main.rs:?:?
  |
1 |     fn bad_1(x) { .. }
  |           ^
  |           |
  |           The following patterns do not have a fully determined type.
  |           help: Write type annotations on the following patterns:
  |
  |               x: u8
  |

error: aborting due to previous error
```

Providing this type information is optional.

### Ascribing `impl Trait`

[RFC 1951]: https://github.com/rust-lang/rfcs/blob/master/text/1951-expand-impl-trait.md
[RFC 2071]: https://github.com/rust-lang/rfcs/blob/master/text/2071-impl-trait-existential-types.md#reference-impl-trait-in-let-const-and-static

Ascribing an expression or a pattern to a type `impl Trait` for some `Trait`
is permitted by the compiler. The semantics of doing so are as follows:

1. When a pattern in the `fn` signature contains `impl Trait`,
   it has the usual `universal_impl_trait` semantics as specified by
   [RFC 1951]. This means that for each `impl Trait` in any pattern in
   the `fn` signature, an "anonymous" type parameter is added.

2. When a pattern or expression inside an `fn` body is ascribed with a type
   of form `impl Trait`, the type checking rules are as specified by
   [RFC 2071] with respect to `let` bindings.

# Drawbacks
[drawbacks]: #drawbacks

## Language Complexity

We believe that we've demonstrated that this RFC simplifies the language by
applying rules uniformly, and thus has a negative complexity cost on balance.
However, this view may not be shared by everybody. It is a legitimate position
to take to view this as an increase in language complexity.

## Readability of function definitions

One of the benefits of static typing is that it acts as machine checked
documentation. This is particularly useful on API boundaries such as function
definitions. Indeed, even though a language like Haskell features
global type inference, it is the cultural norm in the Haskell community
that functions should have explicitly annotated signatures.

This RFC does not introduce global type inference in any way, but it does allow
you to elide the exact type of each function parameter. This could potentially
lead to less obvious type signatures. However, the RFC has purposefully
defines the rules for the elision in such a way that you should fairly easily
be able to see what type a function parameter is of.

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

In this context, the syntax is not as readable and ergonomic as we would like.
However, it is our contention that the need to use the syntax will not be that
common and that consistency is paramount. To mitigate the readability angle,
this RFC proposes to lint towards usage of parenthesis when `baz` is an identifier.

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

### Arrow, `->` syntax

[rust-lang/rust#50547]: https://github.com/rust-lang/rust/issues/50547

Another idea is to use an arrow syntax `expr -> type`.
This idea was floated on [rust-lang/rust#50547] as:

```rust
async -> io::Result<()> {
    ...
}
```

To apply this consistently, you'd then write:

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

## [RFC 1685] and deprecation schedule

[rust-lang/rust#48309]: https://github.com/rust-lang/rust/pull/48309
[48309_noted]: https://github.com/rust-lang/rust/pull/48309#issuecomment-391288075

It has been [noted][48309_noted] on [rust-lang/rust#48309] that:

> @scottmcm I think if we had a concrete motivation to be stricter, we could be.

The expedited schedule of making

```rust
trait Foo {
    fn bar(Type) { ... }
}
```

a warning in 2015 and a hard error in 2018 is motivated by wanting
to emphasize the nature of function parameters as patterns uniformly.
For the purposes of improving learnability, we believe it is prudent
to avoid syntactic inconsistencies in the language.

Furthermore, it was also noted on the same PR by @nikomatsakis that:

> [..] we don't think there's much use of this in the wild,
> and we have an automated fix.

Thus, we believe it is reasonable to expedite the schedule.

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

## Type inference algorithm for lifetimes

With respect to type inference of function types, as specified in the reference
above, there are are multiple choices and restrictions that can be imposed.
In the reference we've specified a model based on not inferring and implicitly
quantifying type parameters and lifetime parameters (unless a lifetime position
is explicitly included). For type parameters we believe this is the right choice
because we believe they are crucially important to include for the reader.
However, for lifetimes, things are more subtle and there are designs possible.
We will now try to outline these choices and why the current design and
restrictions were chosen.

### Does `Constructor(x : &usize)` inhibit match ergonomics?

Given the definition of `good_23` from above:

```rust
fn good_23(Wrapping(x: &usize): &_) {}
```

we can ask what the type of `good_23` should be. In the current design we've
judged it as (1):

```rust
good_23 : for<'a> fn(&'a Wrapping<usize>) -> ()
```

However, another plausible judgement (2) is:

```rust
good_23 : for<'a, 'b: 'a> fn(&'a Wrapping<&'b usize>) -> ()
```

We can also ask whether given (A):

```rust
match &alpha {
    None => {},
    Some(x: &usize) => {},
}
```

the type of `alpha` is `&Option<usize>` or if it is `&Option<&usize>`.

These are in essence the same problem.
The question is: does type ascription inside a pattern disable default match
bindings? We argue that they should not and that judgement (1) is right.
We argue this for two reasons:

#### 1. Consistency with typing `let`

Consider if a user had today written the following valid snippet:

```rust
match &alpha {
    None => {},
    Some(x) => {
        let x: &usize = x;
    },
}
```

[verify_match_erg]: https://play.rust-lang.org/?gist=f484300d1f0435deed234faa9d1da14b&version=stable&mode=debug&edition=2015

If they did that, then match ergonomics would apply and `alpha` would be
typed at `&Option<usize>`. You can [verify][verify_match_erg] this with
the following snippet:

```rust
trait Foo { 
    fn new() -> Self;
}

impl Foo for &'static Option<usize> {
    fn new() -> Self { &Some(0) }
}

impl Foo for &'static Option<&'static usize> {
    fn new() -> Self { &Some(&1) }
}

fn main() {
    let x: &_ = Foo::new();
    if let Some(x) = x {
        let x: &usize = x;
        // Prints 0.
        println!("{}", x);
    }
}
```

However, if the user moved to snippet (A), then the example would instead print
`0` to the terminal. We believe that the ability to transition in this way to
and from the typed `let` binding is important for consistency and to make
the langauge easy to understand.

#### 2. Consistency with closures

Consider the following program:

```rust
fn main() {
    struct Foo<T>(T);
    let fun = |Foo(x): &_| { let _ : &usize = x; };
    fun(&Foo(&0usize));
}
```

It is ill typed. When the compiler sees this it will error with:

```rust
error[E0308]: mismatched types
 --> src/main.rs:4:12
  |
4 |     fun(&Foo(&0usize));
  |              ^^^^^^^
  |              |
  |              expected usize, found &usize
  |              help: consider removing the borrow: `0usize`
  |
```

This means that the compiler judged `fun` as:

```rust
fun : for<'a> impl Fn(&'a Foo<usize>) -> ()
```

as opposed to:

```rust
fun : for<'a, 'b: 'a> impl Fn(&'a Foo<&'b usize>) -> ()
```

We argue that consistency of function definitions with closures
is paramount for an understandable language.
Thus, we conclude that match ergonomics should apply.

### How much unification do we permit?

One of the rules we've imposed is that each lifetime position mentioned
in a type fragment of some pattern in a function parameter introduces
a distinct lifetime. However, this means that type inference may sometimes
introduce too many lifetimes for one type and therefore reject the definition.
This happened in the case of:

```rust
struct Bar<'a>(&'a bool);
struct Wibble<'a>(&'a i32, Bar<'a>);

// Inferred `x: &'a`, `y: &'b Bar<'b>` but `'a != 'b` which `Wibble<'?>` requires.
fn bad_4(Wibble(x: &i32, y: Bar<'_>)) {}
```

However, type inference could accept such a definition by not eagerly assigning
lifetimes and instead adding them lazily as they are needed.
For example, we could say that:

> With respect to lifetime elision, when generating input lifetime parameters,
> the most general setup of lifetimes is inferred such that:
>
> + a struct using the same lifetime on any number of fields get assigned
>   the same distinct lifetime.
>
> + patterns for each generic parameter of a struct allowed to be assigned
>   the set of distinct lifetimes each pattern itself generates.
>   This rule is applied inductively.
>
> [current rules]: https://doc.rust-lang.org/stable/reference/lifetime-elision.html
> With respect to output lifetime elision, the [current rules] apply.

If we applied such a rule, we could accept the following two previously
rejected definitions:

```rust
// 1) Looking at `Quux($pat_1, $pat_2)`, we know that:
//    a) `parameter : Quux<'?t, '?u>`.
//    b) `'?u: '?t`.
//    c) `$pat_1 : &'?t Foo`.
//    d) `$pat_2 : Bar<'?u>`.
// 2) Looking at `$pat_1 : &'?t Foo = x`, we know that:
//    a) `x : &'?t Foo`.
// 3) Looking at `$pat_2 : Bar<'?u> = Bar($pat_3)`, we know that:
//    a) `$pat_3 : &'?u bool`.
// 4) Looking at `$pat_3 : &'?u bool = y`, we know that:
//    a) `y : &'?u bool`.
// 5) Having lifetime quantification variables `'?t` and `?'u`,
//    we quantify and substitute `'a` and `'b` for these.
// -----------------------------------------------------------------------------
// + `bad_3_now_good : for<'a, 'b: 'a> fn(Quux<'a, 'b>) -> ()`.
// + `x : &'a Foo`.
// + `y : &'b bool`.
fn bad_3_now_good(Quux(x, Bar(y))) {}

// 1) Looking at `Wibble($pat_1, $pat_2)`, we know that:
//    a) `parameter : Wibble<'?t>`.
//    b) `$pat_1 : &?'t i32`.
//    c) `$pat_2 : Bar<&?'t>`.
// 2) Looking at `$pat_1 : &?'t i32 = x: &'_ i32`, we know that:
//    a) `x : &'?v i32`.
//    b) `x : &'?t i32`.
//    c) `'?v = '?t`.
// 3) Looking at `$pat_2 : Bar<&?'t> = y: Bar<'_>`, we know that:
//    a) `y : Bar<'?x>`.
//    b) `y : Bar<'?t>`.
//    c) `'?x = '?t`.
// 4) Having lifetime quantification variable `'?t`,
//    we quantify and substitute `'a` for it.
// -----------------------------------------------------------------------------
// + `bad_4_now_good : for<'a> fn(Wibble<'a>) -> ()`.
// + `x : &'a i32`.
// + `y : Bar<'a>`.
fn bad_4_now_good(Wibble(x: &i32, y: Bar<'_>)) {}
```

This lazy unification engine can also accept definitions
`bad_6`, `bad_7`, and `bad_10`. For example (with a match ergonomics twist):

```rust
// 1) Looking at `$pat_1: &'_ _`, we know that:
//    a) `parameter : &'?t ?T`
//    b) `?T: '?t`.
//    c) `$pat_1 : `?T`.
// 2) Looking at `$pat_1 : ?T = Beta($pat_2)`, we know that:
//    a) `?T = Beta<'?u>`.
//    b) `'?u: '?t`.
//    b) `$pat_2 : &'?t Bar<'?u>`.
// 3) Looking at `$pat_2 : &'?t Bar<'?u> = & $pat_3`, we know that:
//    a) `$pat_3 : Bar<'?u>`.
// 4) Looking at `$pat_3 : Bar<'?u> = Bar($pat_4)`, we know that:
//    a) `$pat_4 : &'?u bool`.
// 5) Looking at `$pat_4 : &'?u bool = & $pat_5`, we know that:
//    a) `$pat_5 : bool`.
// 6) Looking at `$pat_5 : bool = x`, we know that:
//    a) `x : bool`.
// 7) Having lifetime quantification variables `'?t`, `'?u`
//    we quantify and substitute `'a` and `'b` for them.
// -----------------------------------------------------------------------------
// + `bad_10_twist_now_good : for<'a, 'b: 'a> fn(&'a Beta<'b>) -> ()`.
// + `x : bool`.
fn bad_10_twist_now_good(Beta(&Bar(&x)): &_) {}
```

Why have we then not proposed this mechanism right now if it is so flexible?
For two reasons:

1. If you take a look at `bad_4_now_good` you see two lifetime positions.
   You might infer from this that these are two distinct lifetimes,
   but that inference would be mistaken as type of `bad_4_now_good`
   is `for<'a> fn(Wibble<'a>) -> ()`.

2. References are sometimes important to highlight.
   In particular, this unification model would accept `bad_3_now_good`
   which in no way from the patterns indicate that:

   ```rust
   x : &'a Foo
   y : &'b bool
   ```

We might eventually relax these rules.
However, we have concluded that, as a starting point, to keep things simple,
we will not extend lifetime elision to patterns,
or allow `bad_3`, `bad_4`, `bad_6`, `bad_7`, and `bad_10` to compile.

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

1. Should we permit `async : Type { .. }` and should `Type` be the inner type?

   There is an inconsistency in the desugaring of the various
   `KEYWORD : Type { .. }` forms. While ascriptions on other block forms desugar
   as `KEYWORD { .. } : Type`, the `async : Type { .. }` construct desugars
   as `async { .. } : impl Future<Output = Type>`.

   This could lead to surprises for some users. Thus, we might consider a
   different symbol just for `async` such as `async -> Type { .. }`.
   We could also consider not having this feature for the `async` block
   form at all. These are all reasonable alternatives.

   Another possibility is to change `async` and by extension also `async fn` to
   use the external type approach. This is however considerably out of this
   RFC's scope.

   Speaking of `async fn` and internal types, while it is unfortunate that we
   can't be fully consistent in the desugaring without moving to an external
   type approach, this problem is really inherent to the nature of `async fn`
   using the inner-type method itself. It is thus equally possible that
   `async : Type { .. }` desugaring as `async { .. } : impl Future<Output = R>`
   will align with what people expects this to mean because it is how
   `async` works elsewhere.

2. Should type ascription in patterns inhibit match ergonomics?

3. What exactly should be allowed wrt. type inference of function types?
   Because this is rather subtle, it is considered OK to leave this for
   and tweak it during stabilization.

# Possible future work
[possible future work]: #possible-future-work

## `self` as a pattern

To simplify the grammar further and to not make distinction between different
variants of `fn` items, one step that we could take is to make `self` a pattern.
We would then introduce the following into the pattern grammar:

```rust
pat
: ...
| ('&' lifetime?)? maybe_mut SELF
;
```

Note however that while `self` is a legal pattern *grammatically*, this
does not mean that we need to allow it anywhere but where it is allowed today
(in methods of inherent and trait `impl`s). The type checker could forbid
occurrences we don't allow today. This change would likely simplify the grammar
and improve error messages. However, it would also likely complicate the type
checker.

One reason we might want to take this step besides syntactic simplicity is to
enable extension functions ([see internals discussion]) such as:

```rust
fn sorted<T: Ord>(mut self: Vec<T>) -> Vec<T> {
    self.sort();
    self
}
```

[see internals discussion]: https://internals.rust-lang.org/t/idea-simpler-method-syntax-private-helpers/7460

in the future. However, this grammatical change is not proposed in this RFC
at the moment.

## Lifetimes in parameter patterns

In the same vein, we could introduce lifetimes in the pattern grammar such that
you can write `&'a $pat` and `&'mut $pat`. We would then restrict this usage
to function parameters but not elsewhere.
Doing this would allow users to express things such as:

```rust
/// `foo : for<'a> fn(&'a Wrapping<usize>) -> ()`.
fn foo<'a>(&'a Wrapping(x: usize)) { .. }
```

as well as:

```rust
/// `foo : for<'a> fn(Wrapping<&'a mut Foo>) -> ()`.
fn foo<'a>(Wrapping(&'a mut Foo(ref mut x))) { .. }
```

Also note that the previous section can be encoded in terms of the composition
of this section as well as `self` being a pattern making the grammar further
simplified.

However, to limit the scope of this RFC this is not proposed at this point.
We also do not propose it at this point because it is unknown how often this
would occur.

## Elision in trait implementations

One possibility that this RFC opens up grammatically is to let the
well-formedness constraints of implementing a particular trait to inform
type inference such that you may elide type annotations, type parameters,
and `where` clauses from trait methods in the trait implementation.

However, this is a rather large, and possibly controversial,
step in and of itself and therefore such a proposal is out of scope of this RFC.
It is perfectly possible to separate these questions and therefore we do that.