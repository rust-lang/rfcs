- Feature Name: `generalized_type_ascription`
- Start Date: 2018-08-06
- RFC PR: _
- Rust Issue: _

# Summary
[summary]: #summary

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

Finally, we lint (warn-by-default) when a user writes
`Foo { $field: $ident : $type }`
and the compiler instead suggests:
`Foo { $field: ($ident : $type) }`.

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

1. Introducing a temporary `let` binding of form `let ident: type = expr;`,
   as a substitute for type-ascription, forces programmers to invent artificial
   variable names. Naming is hard (particularly for those who are of the
   [pointfree persuasion]). As [the saying goes][TwoHardThings]:
   > “There are only two hard things in Computer Science: cache invalidation and naming things”.

   By reducing the pressure on Rustaceans to name artificial units
   we can let programmers focus on naming where it matters more (API boundaries).

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

3. When you want to pass an expression such as `try { .. }` or
   `async { .. }` to a function which expects a generic `R: Try` or `R: Future`,
   type inference can fail. In this case,
   it is more ergonomic to type-ascribe with `try : R { .. }` instead
   of first introducing an artificial binding.

4. With type ascription, you can annotate smaller bits and subsets of what you
   previously needed to. This especially holds in pattern contexts.
   This will be made clear later on in this RFC.

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

6. Type ascription is helpful when doing *type* driven development (TDD)
   and opens up more possibilities to move in the direction of
   interactive development as is possible with [agda-mode] and [idris-mode].

[agda-mode]: http://agda.readthedocs.io/en/v2.5.2/tools/emacs-mode.html
[idris-mode]: https://github.com/idris-hackers/idris-mode

## Type ascription has already been accepted as an RFC

[RFC 803]: https://github.com/rust-lang/rfcs/blob/master/text/0803-type-ascription.md

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
function parameters, we already have the grammatic means to elide types in
implementations of traits. All that is required now is to employ the rule that
types can be omitted from parameters if and only if the types are fully determined.

However, to avoid doing too much in this RFC, we defer this to another
RFC and simply say for now, that the compiler will always require the full
signature in trait implementations. See the section on [possible future work]
for more details.

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
> As @clarcharr and @nikomatsakis [discussed above](https://github.com/rust-lang/rfcs/pull/2388#issuecomment-378750364), it's nearly always necessary to manually specify the error type for these blocks because of the `Into` conversion that `?` does. @nikomatsakis mentioned that we might even want a syntax which *requires* users to explicitly state the error type (or the full result type, for compatibility with `Option`, `Poll`, etc.). [...]

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
   `CarrierType` may come to late. Therefore, the user may have to backtrack
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
error:

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
if it so happens that you need this help.

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
that it be a hard error in Rust 2018.

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

This lint only applies when `x` is an identifier and not otherwise.

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

The operator `:` binds more tightly than `ref`, `ref mut`, `&`, and `&mut`
in pattern contexts. This is required because currently, the following compiles:

```rust
#[derive(Copy, Clone)]
struct X {}
let a = X {};

let ref b: X = a; // Note the type!
let &c : &X = b; // And here!
```

This entails for example that a Rust compiler will interpret `ref x : T` as
`ref (x : T)` instead of `(ref x) : T`. The same applies to `ref mut`, `&`,
and `&mut`.

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
$path { $ident: $ident : $ty }
```

where `$path`, `$ident`, and `$ty` are the usual meta variables, the compiler
will emit a warn-by-default lint urging the user to instead write:

```rust
$path { $ident: ($ident : $ty) }
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
--------------------------------
Δ, Γ ⊢ (x : σ) : σ
```

Since before, we have the typing rule that:

```
Δ ⊢ τ type
--------------------------------
Δ ⊢ ImplicitlyCoercible(τ, τ)
```

From these typing rules, it follows that:

```
Δ    ⊢ τ type
Δ, Γ ⊢ x : τ
--------------------------------
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
function, including patterns, quantified type variables, and `where` clauses.

The type checker is not allowed to look at the function body and neither is it
permitted to take into account the type information implied by a trait
implementation being well-formed.

The following example definitions are accepted:

```rust
// `x` is fully determined by the ascription.
// The type of `g0: fn(usize) -> ()`.
fn g0(x: usize) {}

struct Wrapping<T>(T);

// Type-variable T is determined by `x: usize`.
// The type of `g1: fn(Wrapping<usize>) -> ()`.
fn g1(Wrapping(x: usize)) {}

// Type-variable T is determined by `x: usize`.
// The type of `g3: for<'a> fn(Wrapping<&'a usize>) -> ()`.
fn g2(Wrapping(x: &usize)) {}

// Same here. Determined by `x: T`.
// The type of `g2: for<T> fn(Wrapping<T>) -> ()`.
fn g3<T>(Wrapping(x: T)) {}

// A type variable is induced by `impl Display`
// and then `typeof(x)` is that variable.
// The type of `g3: for<T: Display> fn(Wrapping<T>) -> ()`.
fn g4(Wrapping(x: impl Display)) {}

struct Foo(usize);

// `Foo` has no type variables to constrain.
// The type of `g5: fn(Foo) -> ()`.
fn g5(Foo(x)) {}

trait Trait { type Assoc; }

// `T` is fully constrained by `X::Assoc`
// which in turn is determined by `X: Trait`.
// The type of `g6: for<X: Trait> fn(Wrapping<X::Assoc>) -> ()`.
fn g6<X: Trait>(Wrapping(x: X::Assoc))
```

But the following definitions are rejected:

```rust
// The type of `x` is fully ambiguous even if we look at the body.
// The type of `b0: fn(?T) -> ()`.
fn b0(x) {}

// The compiler has to look at the body to see that `x: u8`:
// The type of `b1: fn(?T) -> ()`.
fn b1(x) {
    let y: u8 = x;
}

// There is an unconstrained unification variable `?T` from `Wrapping<?T>`.
// The type of `b2: fn(Wrapping<?T>) -> ()`
fn b2(Wrapping(x)) {}

struct X(u8);

impl From<u8> for X {
    // The compiler is not allowed to look at `From<u8>` to
    // understand that `x: u8`.
    fn from(x) -> Self { Self(x) }
}
```

Considering the rejected example function `b1`, a Rust compiler,
knowing that the `typeof(x) = u8` by looking at the body,
will emit an error message with the type identity of `x` in it.
An example error message is:

```rust
error[E0282]: type annotations needed
 --> src/main.rs:?:?
  |
1 |     fn b1(x) { .. }
  |           ^
  |           |
  |           The following patterns do not have a fully determined type.
  |           help: Write type annotations on the following patterns:
  |
  |               x: u8
  |

error: aborting due to previous error
```

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

### [RFC 1685] and deprecation schedule

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

# Unresolved questions
[unresolved]: #unresolved-questions

There are none.

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

## Elision in trait implementations

One possibility that this RFC opens up grammatically is to let the
well-formedness constraints of implementing a particular trait to inform
type inference such that you may elide type annotations, type parameters,
and `where` clauses from trait methods in the trait implementation.

However, this is a rather large, and possibly controversial,
step in and of itself and therefore such a proposal is out of scope of this RFC.
It is perfectly possible to separate these questions and therefore we do that.