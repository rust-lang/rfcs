- Feature Name: `stuctural_records`
- Start Date: 2018-11-02
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

**Introduce _structural records_** of the form `{ foo: 1u8, bar: true }` of
type `{ foo: u8, bar: bool }` into the language. Another way to understand
these sorts of objects is to think of them as *"tuples with named fields"*,
*"unnamed structs"*, or *"anonymous structs"*.

# Motivation
[motivation]: #motivation

There are four motivations to introduce structural records,
two major and two minor.

## Major: Improving ergonomics, readability, and maintainability

Sometimes, you just need a one-off struct that you will use a few times for
some public API or in particular for some internal API. For example, you'd like
to send over a few values to a function, but you don't want to have too many
function arguments. In such a scenario, defining a new nominal `struct` which
contains all the fields you want to pass is not a particularly ergonomic solution.
Instead, it is rather heavy weight:

```rust
struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

fn caller() {
    ...
    let color = Color { red: 255, green: 0, blue: 0 };
    ...
    do_stuff_with(color);
    ...
}

fn do_stuff_with(color: Color) { // Only time we use `Color`!
    some_stuff(color.red);
    ...
    other_stuff(color.green);
    ...
    yet_more_stuff(color.blue);
}
```

To remedy the ergonomics problem, you may instead opt for using a positional
tuple to contain the values you want to pass. However, now you have likely
created a regression for readers of your code since the fields of tuples are
accessed with their positional index, which does not carry clear semantic intent:

```rust
fn caller() {
    ...
    let color = (255, 0, 0);
    ...
    do_stuff_with(color); // Unclear what each position means...
    ...
}

fn do_stuff_with(color: (u8, u8, u8)) { // More ergonomic!
    some_stuff(color.0); // But less readable... :(
    ...
    other_stuff(color.1);
    ...
    yet_more_stuff(color.2);
}
```

Using structural records, we can have our cake and eat it too:

```rust
fn caller() {
    ...
    let color = { red: 255, green: 0, blue: 0 };
    ...
    do_stuff_with(color); // ...but here it is clear.
    ...
}

fn do_stuff_with(color: { red: u8, green: u8, blue: u8 }) { // More ergonomic!
    some_stuff(color.red); // *And* readable! :)
    ...
    other_stuff(color.green);
    ...
    yet_more_stuff(color.blue);
}
```

In the above snippet, the semantic intent of the fields is clear both when
reading the body of the function, as well as when reading the documentation
when `do_stuff_with` is exposed as a public API.

[@eternaleye]: https://internals.rust-lang.org/t/pre-rfc-unnamed-struct-types/3872/58
[@kardeiz]: https://internals.rust-lang.org/t/pre-rfc-unnamed-struct-types/3872/65

Another example of reducing boilerplate was given by [@eternaleye]:

```rust
struct RectangleClassic {
    width: u64,
    height u64,
    red: u8,
    green: u8,
    blue: u8,
}

struct RectangleTidy {
    dimensions: {
        width: u64,
        height: u64,
    },
    color: {
        red: u8,
        green: u8,
        blue: u8,
    },
}
```

In the second type `RectangleTidy`, we keep boilerplate to a minimum and we can
also treat `rect.color` and `rect.dimensions` as separate objects and move them
out of `rect : RectangleTidy` as units, which we cannot do with `RectangleClassic`.
If we wanted to do that, we would have to invent two new types `Dimensions` and
`Color` and then `#[derive(..)]` the bits and pieces that we need.
As noted by [@kardeiz], this ability may also be useful for serializing one-off
structures with `serde`.

## Major: Better rapid prototyping and refactoring

Let's assume we opted for using the type `{ red: u8, green: u8, blue: u8 }`
from above. This gave us the ability to prototype our application rapidly.
However, as time passes, we might have more uses for RGB colours and so we
decide to make a nominal type out of it and give it some operations.
Because the structural record we've used above has named fields, we can easily
refactor it into the nominal type `Color`.

Indeed, an IDE should be able to use the information that exists and provide
the refactoring for you automatically. If we had instead used a positional tuple,
the information would simply be unavailable. Thus, the refactoring could not be
made automatically and if you had to do it manually, you would need to spend
time understanding the code to deduce what proper field names would be.

## Minor: Emulating named function arguments

Structural records could be considered to lessen the need for named
function arguments by writing in the following style:

```rust
fn foo(bar: { baz: u8, quux: u8 }) -> u8 {
    bar.baz + bar.quux
}

fn main() {
    assert_eq!(3, foo({ baz: 1, quux: 2 }));
}
```

While this is a possible use case, in this RFC, we do not see named function
arguments as a *major* motivation for structural records as they do not cover
aspects of named arguments that people sometimes want. In particular:

1. With structural records, you cannot construct them positionally.
   In other words, you may not call `foo` from above with `foo(1, 2)` because
   these records do not have a defined order that users can make use of.

[`<*const T>::copy_to_nonoverlapping`]: 
https://doc.rust-lang.org/std/primitive.pointer.html#method.copy_to_nonoverlapping

2. You cannot adapt existing standard library functions to use named arguments.
   Consider for example the function [`<*const T>::copy_to_nonoverlapping`].
   It has the following signature:

   ```rust
   pub unsafe fn copy_to_nonoverlapping(self, dest: *mut T, count: usize);
   ```

   Because this is an `unsafe` function, we might want to call this as:

   ```rust
   ptr.copy_to_nonoverlapping({ dest: <the_destination>, count: <the_count> })
   ```

   However, because we can write:

   ```rust
   const X: unsafe fn(*const u8, *mut u8, usize)
          = <*const u8>::copy_to_nonoverlapping;
   ```

   it would be a breaking change to redefine the function to take a structural
   record instead of two arguments.

Having noted these two possible deficiencies of structural records as a way to
emulate named function arguments, this emulation can still work well in many
cases. Thus, while the motivation is not major here, we still consider it to
be a minor motivation.

## Minor: Smoothing out the language

The current situation in the language with respect to product types can be
described with the following table:

|                  | Nominal                         | Structural        |
|------------------|---------------------------------|-------------------|
| **Unit**         | Yes, `struct T;`                | Yes, `()`         |
| **Positional**   | Yes, `struct T(A, B);`          | Yes, `(A, B)`     |
| **Named fields** | Yes, `struct T { a: A, b: B }`  | **No**, this RFC  |

As we can see, the current situation is inconsistent.
While the language provides for unit types and positional product types
of both the nominal and structural flavour, the structural variant of
structs with named fields is missing while the nominal type exists.
A consistent programming language is a beautiful language, but it's not an
end in itself. Instead, the main benefit is to reduce surprises for learners.

[@withoutboats]: https://internals.rust-lang.org/t/pre-rfc-unnamed-struct-types/3872/23
[@regexident]: https://internals.rust-lang.org/t/pre-rfc-catching-functions/6505/188

Indeed, [@withoutboats] noted:

> To me this seems consistent and fine - the kind of feature a user could infer to exist from knowing the other features of Rust - but I’m not thrilled by the idea of trying to use this to implement named arguments.

and [@regexident] noted:

> An unnamed type can have zero, n indexed or n named members:
> + `()`
> + `(T, U, …)`
> + **`{ t: T, u: U }`** [*editor's note:* this is **_not_** supported *yet*.]

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Vocabulary

[structural typing]: https://en.wikipedia.org/wiki/Structural_type_system
[nominal typing]: https://en.wikipedia.org/wiki/Nominal_type_system
[product type]: https://en.wikipedia.org/wiki/Product_type
[record type]: https://en.wikipedia.org/wiki/Record_(computer_science)

- With *[structural typing]*, the equivalence of two types is determined by
  looking at their structure. For example, given the type `(A, B)` and `(C, D)`
  where `A`, `B`, `C`, and `D` are types, if `A == C` and `B == D`,
  then `(A, B) == (C, D)`. In Rust, the only case of structural typing is
  the positional tuples we've already seen.

  The main benefit of structural typing is that it becomes ergonomic and
  quite expressive to produce new types *"out of thin air"* without having
  to predefine each type.

- With *[nominal typing]*, types are instead equivalent if their declared
  names are the same. For example, assuming that the types `Foo` and `Bar`
  are defined as:

  ```rust
  struct Foo(u8, bool);
  struct Bar(u8, bool);
  ```

  even though they have the same structure, `Foo` is not the same type as `Bar`
  and so the type system will not let you use a `Foo` where a `Bar` is expected.

  The main benefit to nominal typing is maintainability and robustness.
  In Rust, we take advantage of nominal typing coupled with privacy to
  enforce API invariants. This is particularly important for code that
  involves use of `unsafe`.

- A *[product type]* is a type which is made up of a sequence of other types.
  For example, the type `(A, B, C)` consists of the types `A`, `B`, *and* `C`.
  They are called product types because the number of possible values that the
  type can take is the *product* of the number of values that each component /
  factor / operand type can take. For example, if we consider `(A, B, C)`,
  the number of values is `values(A) * values(B) * values(C)`.

- A *[record type]* is a special case of a product type in which each component
  type is *labeled*. In Rust, record types are `struct`s with named fields.
  For example, you can write:

  ```rust
  struct Foo {
      bar: u8,
      baz: bool,
  }
  ```

  The only case of a record type in Rust uses nominal typing.
  There is currently no structural variant of record types.
  This brings us to the proposal in this RFC...

## Proposal

As we've seen, Rust currently lacks a structurally typed variant of record types.
In this RFC, we propose to change this by introducing record types that are
also structural, or in other words: *"structural records"*.

### Construction

To create a structural record with the field `alpha` with value `42u8`,
`beta` with value `true`, and `gamma` with value `"Dancing Ferris"`,
you can simply write:

```rust
let my_record = {
    alpha: 42u8,
    beta: true,
    gamma: "Dancing Ferris"
};
```

Note how this is the same syntax as used for creating normal structs but without
the name of the struct. So we have taken:

```rust
struct MyRecordType {
    alpha: u8,
    beta: bool,
    gamma: &'static str,
}

let my_record_nominal = MyRecordType {
    alpha: 42u8,
    beta: true,
    gamma: "Dancing Ferris"
};
```

and removed `MyRecordType`. Note that because we are using structural typing,
we did not have to define a type `MyRecordType` ahead of time.

If you already had variables named `alpha` and `beta` in scope,
just as you could have with `MyRecordType`, you could have also written:

```rust
let alpha = 42u8;
let beta = true;

let my_record = {
    alpha,
    beta,
    gamma: "Dancing Ferris"
};
```

### Pattern matching

Once you have produced a structural record, you can also pattern match on the
expression. To do so, you can write:

```rust
match my_record {
    { alpha, beta, gamma } => println!("{}, {}, {}", alpha, beta, gamma),
}
```

This pattern is *irrefutable* so you can also just write:

```rust
let { alpha, beta, gamma } = my_record;
println!("{}, {}, {}", alpha, beta, gamma);
```

This is not particular to `match` and `let`. This also works for `if let`,
`while let`, `for` loops, and function arguments.

If we had used `MyRecordType`, you would have instead written:

```rust
let MyRecordType { alpha, beta, gamma } = my_record_nominal;
println!("{}, {}, {}", alpha, beta, gamma);
```

When pattern matching on a structural record, it is also possible to give
the binding you've created a different name. To do so, write:

```rust
let { alpha: new_alpha, beta, gamma } = my_record;
println!("{}, {}, {}", new_alpha, beta, gamma);
```

In this snippet, we've bound the field `alpha` to the binding `new_alpha`.
This is not limited to one field, you can do this will all of them.

As with nominal structs, it's possible to ignore some or all fields when
pattern matching on structural records. Examples include:

```rust
let { alpha, .. } = my_record;
println!("{}", alpha);

let { .. } = my_record;
```

### Field access

Given the binding `my_record_nominal` of type `MyRecordType`, you can access its
fields with the usual `my_record_nomina.alpha` syntax. This also applies to
structural records. It is perfectly valid to move or copy:

```rust
println!("The answer to life... is: {}", my_record.alpha);
```

or to borrow a field:

```rust
fn borrow(x: &bool) { .. }

borrow(&my_record.beta);
```

including mutably:

```rust
fn mutably_borrow(x: &mut bool) { .. }

let mut my_record = { alpha: 42u8, beta: true, gamma: "Dancing Ferris" };

mutably_borrow(&mut my_record.beta);
```

### One-field records

Positional tuples with a single element use the syntax `(x,)` both for types
and for values to distinguish between the 1-element tuple and wrapping a value
in parenthesis. To avoid ambiguities with the `{ x }` syntax that is already
allowed, we impose the same rule for one-field structural records. To create
such a record, you must write `{ x, }` or `{ x: <value>, }`. This difference
is also imposed to maintain a sense of consistency between the positional
and the non-positional structural variant.

### Struct update syntax

Nominal structs support what is referred to as the *"struct update syntax"*,
otherwise known as functional record update (FRU). For example, you can write:

```rust
struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

let yellow = Color { red: 0, green: 255, blue: 255 };

let white = Color { red: 255, ..yellow };
```

This also works for structural records, so you can write:

```rust
let yellow = { red: 0, green: 255, blue: 255 };

let white = { red: 255, ..yellow };
```

To match the behaviour of FRU for nominal structs, we impose a restriction
that the fields mentioned before `..yellow` must all exist in `yellow` and
have the same types. This means that we cannot write:

```rust
let white_50_opacity = { alpha: 0.5, red: 255, ..yellow };
```

However, there is no fundamental reason why we could not allow this,
but to start off more conservative and to be uniform in behaviour,
we don't allow this for the time being.

### The type of a structural record

You might be wondering what the type of `my_record` that we've been using
thus far is. Because this is structural typing, the fields are significant,
so the type of the record is simply:

```rust
type TheRecord = { alpha: u8, beta: bool, gamma: &'static str };

let my_record: TheRecord = { alpha: 42u8, beta: true, gamma: "Dancing Ferris" };
```

Notice how this matches the way we defined `MyRecordType` is we remove
the prefix `struct MyRecordType`.

The order in which we've put `alpha`, `beta`, and `gamma` here does not matter.
We could have also written:

```rust
type TheRecord = { beta: bool, alpha: u8, gamma: &'static str };
```

As long as the type is a *permutation* of each pair of field name and field type,
the type is the same. This also means that we can write: 

```rust
let my_record: TheRecord = { alpha: 42u8, gamma: "Dancing Ferris", beta: true };
```

### Implemented traits

With respect to trait implementations, because the type is structural,
and because there may be an unbound number of fields that can all be arbitrary
identifiers, there's no way to define implementations for the usual traits
in the language itself.

[tuples]: https://doc.rust-lang.org/std/primitive.tuple.html

Instead, the compiler will automatically provide trait implementations for the
standard traits that are implemented for [tuples]. These traits are: `Clone`,
`Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Debug`, `Default`, and `Hash`.
Each of these traits will only be implemented if all the field types of a struct
implements the trait.

For all of the aforementioned standard traits, the semantics of the
implementations are similar to that of `#[derive(Trait)]` for named-field structs.

+ For cloning `{ alpha: A, beta: B, gamma: C }` the logic is simply:

  ```rust
  {
      alpha: self.alpha.clone(),
      beta: self.beta.clone(),
      gamma: self.gamma.clone(),
  }
  ```

+ For `Default`, you would get:

  ```rust
  {
      alpha: Default::default(),
      beta: Default::default(),
      gamma: Default::default(),
  }
  ```

+ For `PartialEq`, each field is compared with same field in `other: Self`.

+ For `ParialOrd` and `Ord`, lexicographic ordering is used based on
  the name of the fields and not the order given because structural records
  don't respect the order in which the fields are put when constructing or
  giving the type of the record.

+ For `Debug` the same lexicographic ordering for `Ord` is used.
  As an example, when printing out `my_record` as with:

  ```rust
  let my_record = { beta: true, alpha: 42u8, gamma: "Dancing Ferris" };
  println!("{:#?}", my_record);
  ```
  
  the following would appear:
  
  ```rust
  {
      alpha: 42,
      beta: true,
      gamma: "Dancing Ferris"
  }
  ```

+ For `Hash`, the same ordering of the fields as before is used and then
  `self.the_field.hash(state)` is called on each field in that order.
  For example:

  ```rust
  self.alpha.hash(state);
  self.beta.hash(state);
  self.gamma.hash(state);
  ```

For auto traits (e.g. `Send`, `Sync`, `Unpin`), if all field types implement
the auto trait, then the structural record does as well.
For example, if `A` and `B` are `Send`, then so is `{ x: A, y: B }`.

A structural record is `Sized` if all field types are.
If the lexicographically last field of a structural record is `!Sized`,
then so is the structural record. If any other field but the last is
`!Sized`, then the type of the structural record is not well-formed.

### Implementations and orphans

[RFC 2451]: https://github.com/rust-lang/rfcs/pull/2451

It is possible to define your own implementations for a structural record.
The orphan rules that apply here are those of [RFC 2451] by viewing a structural
record as a positional tuple after sorting the elements lexicographically.

For example, if a `trait` is crate-local, we may implement it for a record:

```rust
trait Foo {}

impl Foo for { alpha: bool, beta: u8 } {}
```

Under 2451, we can also write:

```rust
struct Local<T>(T);

impl From<()> for Local<{ alpha: bool, beta: u8 }> { ... }
```

This is valid because `Local` is considered a local type.

However, a structural record itself isn't a local type, so you cannot write:

```rust
struct A;

impl From<()> for { alpha: A, beta: u8 } { ... }
```

This is the case even though `A` is a type local to the crate.

The behaviour for inherent implementations is also akin to tuples.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

### Grammar
[grammar]: #grammar

Given:

```rust
field ::= ident | LIT_INTEGER ;
```

We extend the expression grammar with:

```rust
expr ::= ... | expr_srec ;

expr_srec ::= "{" field_init "," field_inits_tail? "}" ;
field_inits_tail ::= field_inits ","? | (field_inits ",")? DOTDOT expr ;
field_inits ::= field_init ("," field_init)* ;
field_init ::= ident | field ":" expr ;
```

Note that this grammar permits `{ 0: x, 1: y }`.
We do so to increase reuse in parsing, to improve consistency,
and to simplify for macro users.
This is also applied in the grammars below.

We extend the pattern grammar with:

```rust
pat ::= ... | pat_srec ;

pat_srec ::= "{" pat_srec_start "}" ;
pat_srec_start ::= DOTDOT | pat_field "," pat_srec_tail
pat_srec_tail ::= pat_fields ","? | (pat_fields ",")? DOTDOT? ;
pat_fields ::= pat_field ("," pat_field)* ;
pat_field ::= BOX? binding_mode? ident | field ":" pat ;
```

Finally, we extend the type grammar with:

```rust
ty ::= ... | ty_srec ;

ty_srec ::= "{" (ty_field ",")+ (ty_field ","?)? "}" ;
ty_field ::= field ":" ty ;
```

Post macro expansion, we check that positional and named fields are not mixed.
We also check that there are no gaps in the positional fields when sorted.

### Static semantics: Type checking and inference

The static semantics of structural records are more or less equivalent to those
of positional tuples with minor differences, particularly that structural records
have named fields and tuples don't. Below, we give a partial (because otherwise
we'd be replicating every aspect of tuples...) overview of the rules.

*Note:* While the syntax in the [grammar] section was the actual concrete
surface syntax, the syntax in this section is *always* abstract syntax.

Let:

+ `n` range over the natural numbers.
+ `i` denote an index over some set or list.
+ `f` range over valid identifiers.
+ `Γ` be a well formed (typing) environment.
+ `σ` range over types.
+ `τ` range over terms.
+ `p` range over patterns.
+ `?x` denote a *fresh* unification variable named `x`.
+ `Γ ⊢ x : σ` denote a judgement that `x` is of type `σ` in the environment `Γ`.
  You can read this as *"`Γ` entails that `x` is of type `σ`"*.
+ `[x]n` denote `n` amount of `x`s. We write `x_i` for the `i`th `x`.

#### Type identity

Given a structural record type `{ [f_i: τ_i]n }`,
we consider all permutations of the field-type pairs to be the same type.
For example, `{ foo: u8, bar: bool }` and `{ bar: bool, foo: u8 }` are
the same type, but `{ foo: bool, bar: u8 }` is not.
While this can be seen as a form of subtyping in the type system,
here we merely see it as part of the concrete syntax which can be done
away with by sorting on the field names before type checking.
However, care must be taken to provide error messages that show the types
as the user wrote them and not as how the type system seems them.

Furthermore, for a structural record to be a well-formed type,
the component types must each be well-formed and each field
name may only occur once.

#### Term / Expression typing

We have the typing rule:

```rust
∀ i ∈ 0..n. Γ ⊢ τ_i : σ_i
------------------------------------- TmSRecCtor
Γ ⊢ { [f_i: τ_i]n } : { [f_i: σ_i]n }
```

That is, given a set of terms `τ_i` each typed at `σ_i`,
the structural record construction expression `{ f_i: τ_i }`
is typed at `{ f_i: σ_i }`.

If all expressions `τ_i` are promotable, then so is `{ [f_i: τ_i]n }`.

We have the typing rule:

```rust
Γ ⊢ τ : { [f_i: σ_i]n }
j ∈ 0..n
----------------------- TmSRecFieldProj
Γ ⊢ τ.f_j : σ_j
```

That is, given a term `τ : { [f_i: σ_i]n }`, if `f_j` is a field in
`{ [f_i: σ_i]n }`, then `τ.f_j` is typed at `σ_j`.

All other usual rules and behaviours such as borrowing, ownership,
what is and isn't a place expression, that apply to tuples apply to
structural records as well except that chaining field projections,
i.e. `rec.foo.bar`, is legal for structural records.

#### Pattern typing

Given a structural record pattern field of form:

```rust
srec_pat_field ::= f | f : p ;
```

We have the typing rules:

```rust
Γ ⊢ srec_pat_field = f
------------------------ PatSRecFieldPun
Γ ⊢ srec_pat_field : ?σ
```

1. That is, the field `f` receives a fresh unification variable `?σ`.

```rust
Γ ⊢ srec_pat_field = f: p   Γ ⊢ p : σ
-------------------------------------- PatSRecFieldPat
Γ ⊢ srec_pat_field : σ_i
```

2. That is, given `f: p`, if the pattern `p` is judged to be of type `σ`, 
   then `f: p` is typed at `σ`.

Given a structural record pattern of form:

```rust
srec_pat ::= { srec_pat_field* ..? } ;
```

We have the typing rules:

```rust
∀ i ∈ 0..n. Γ ⊢ srec_pat_field_i : σ_i
--------------------------------------------- PatSRec
Γ ⊢ { [srec_pat_field_i]n } : { [f_i: σ_i]n }
```

3. That is, given a structural record pattern with `n` fields,
   if each constituent `srec_pat_field_i` is typed at `σ_i`,
   the structural record pattern `{ [srec_pat_field_i]n }` is
   typed at `{ [f_i: σ_i]n }`. For example, the pattern `{ foo, bar }`
   is typed at `{ foo: ?A, bar: ?B }`.

```rust
∀ i ∈ 0..n. Γ ⊢ srec_pat_field_i : σ_i
----------------------------------------------------------------- PatSRecDots
Γ ⊢ { [srec_pat_field_i]n, .. } : { [f_i: σ_i]n, [?f_j: ?σ_j]?m }
```

4. That is, given a structural record pattern with `n` fields and with
   a tail `..` "pattern" denoting the rest of the structural record fields
   that are ignored, if each `srec_pat_field_i` is typed at `σ_i`,
   the structural record pattern `{ [srec_pat_field_i]n, .. }` is typed at
   `{ [f_i: σ_i]n, [?f_j: ?σ_j]?m }` where `[?f_j: ?σ_j]?m` is the type
   fragment for the tail of the pattern corresponding to `..`.

   A field unification variable `?f_j` as well as a type unification variable
   `?σ_j` corresponding to it is introduced for each field in the tail.
   A length unification variable `?m` is introduced to stand for the unknown
   length the tail can have.

   For example, the pattern `{ foo, bar, .. }` is typed at:
   `{ foo: ?A, bar: ?B, [?f_j: ?σ_j]?m }`.

```rust
------------------------------ PatSRecOnlyDots
Γ ⊢ { .. } : { [?f_i: ?σ_i]?n
```

5. That is, given the structural record pattern `{ .. }`, it is assigned
   the type `{ [?f_i: ?σ_i]?n` where `?f_i` and `?σ_i` are unification
   variables for the field names and their corresponding types up to
   the length `?n` which is also a unification variable.

The rules in 3. - 5. are the analogue of the rules for tuples,
in particular, it is the same behaviour as with:

```rust
let (x, y) = (1, 2);
let (x, y, ..) = (1, 2, 3, 4); // or any number of elements...
let (..) = (1, 2, 3);
```

The usual rules with respect to default match bindings, mutable patterns,
and so on apply to structural records as well.

#### Coherence and orphan rules

A structural record has the same behaviour that a positional tuple has with
respect to the orphan rules. In particular, a structural record is not a
local type in any crate irrespective of whether all field types are.

With respect to checking overlap, if two structural record types have an
intersection of field names which is non-empty, the records cannot overlap.
If instead the intersection is empty, and there is some field in both records
for which the field type is not more specific than the other, there is either
overlap or we cannot decide if there is.

#### Auto-provided implementations

For structural records, a set of implementations for some standard library
traits is provided automatically. This is detailed in the guide.

For the traits for which implementations are provided automatically,
such as `#[derive(Default)]`, if we consider a type such as:

```rust
#[derive(Default)]
struct RectangleTidy {
    dimensions: {
        width: u64,
        height: u64,
    },
    color: {
        red: u8,
        green: u8,
        blue: u8,
    },
}
```

The generated code would be akin to:

```rust
impl Default for RectangleTidy {
    fn default() -> Self {
        RectangleTidy {
            dimensions: Default::default(),
            color: Default::default()
        }
    }
}
```

as opposed to:

```rust
impl Default for RectangleTidy {
    fn default() -> Self {
        RectangleTidy {
            dimensions: {
               width: Default::default(),
               height: Default::default(),
            },
            color: {
                red: Default::default(),
                green: Default::default(),
                blue: Default::default(),
            }
        }
    }
}
```

This works, including when nested, because each layer of structural records will
have implementations provided given that they satisfy the conditions aforementioned.

### Dynamic Semantics: Layout

The layout of a structural record is the same of a positional tuple
after the fields have been lexicographically field-name ordered.
Since the layout of tuples are unspecified, this is also the case
for structural records.

# Drawbacks
[drawbacks]: #drawbacks

## Overuse?

Nominal typing is a great thing. It offers robustness and encapsulation with
which quantities that are semantically different but which have the same type
can be distinguished statically. With privacy, you can also build APIs that
make use of `unsafe` that you couldn't do with tuples or structural records.

If structural records are overused, this may reduce the overall robustness
of code in the ecosystem. However, we argue that structural records are
more robust than positional tuples are and allow you to more naturally
transition towards nominally typed records so the loss of robustness
may be made up for by reduced usage of positional tuples.

## Backtracking

In the [grammar] we have proposed for structural records there is a possibility
of some minor backtracking in the grammar (but no ambiguity).
Namely, when type ascription for expressions is present in the same language,
then backtracking may required for the first field.
That is, when having parsed:

```rust
{ $ident:
```

we cannot know whether it will become a block with a type ascription,
i.e. `{ $ident: $type }`, or if it will become a structural record
`{ $ident: $expr, }` (the comma makes this unambiguous).

[RFC 2544]: https://github.com/rust-lang/rfcs/pull/2544

This changes the formal parsing complexity of Rust such that it is no
longer LL(k). While this is a nice property to retain, this is quite a
localized instance of backtracking that a parser combinator library
should be able to deal with easily. See [RFC 2544] for a discussion.

There are also other places where allowing backtracking could be
useful such as with [RFC 2544]. Furthermore, since `{ $ident: $type }`
is not likely to occur frequently in the wild even if type ascription
is stabilized, performance is unlikely to suffer notably.

Another instance where backtracking / lookahead may be required is when parsing:

```rust
fn f() where $constraint_1, { $ident:
```

In this case, there is no ambiguity, but `{ $ident: ...` could either be the
start of a structural record type in the second constraint or belonging to the
function body (type ascription on an identifier). The second case is likely
pathological, and so it is unlikely to occur much in practice.

## Hard to implement crate-external traits

As with positional tuples, because a structural record is never crate local,
this presents users with a problem when they need to implement a trait they
don't own for a structural record comprising of crate-local types.

For example, say that you have the crate-local types `Foo` and `Bar`.
Both of these types implement `serde::Serialize`.
Now you'd like to serialize `type T = { foo: Foo, bar: Bar };`.
However, because neither `serde::Serialize` nor `T` is crate-local,
you cannot `impl serde::Serialize for T { ... }`.

This inability is both a good and a bad thing. The good part of it is that it
might prevent overuse of structural records and provide some pressure towards
nominal typing that might be good for robustness. The bad part is that these
sort of one-off structures are a good reason to have structural records in the
first place. With some combined quantification of field labels (possibly via
const generics), and with tuple-variadic generics, it should be possible (for
`serde`, if there is a will, to offer implementations of `Serialize` for all
structural records.

Note that while `impl serde::Serialize for T { ... }` may not be possible
without extensions, the following would be:

```rust
#[derive(Serialize)]
struct RectangleTidy {
    dimensions: {
        width: u64,
        height: u64,
    },
    color: {
        red: u8,
        green: u8,
        blue: u8,
    },
}
```

## "Auto-implementing traits is a magical hack"

Indeed, we would much prefer to use a less magical approach,
but providing these traits without compiler magic would require
significantly more complexity such as polymorphism over field names
as well as variadic generics. Therefore, to make structural records
usable in practice, providing a small set of traits with compiler
magic is a considerably less complex approach. In the future, perhaps
the magic could be removed.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

The syntax we've used is chosen to be maximally consistent
with positional tuples and with nominally typed braced structs.
In particular, `Foo(x, y)` for tuple structs becomes `(x, y)`
for the structural variant, `Foo { a: x, b: y }` becomes
`{ a: x, b: y }`, `(x,)` becomes `{ a: x }`, and the pattern
`(x, y, ..)` becomes `{ a: x, b: y, .. }`.

We could require the prefix `struct` as in `struct { a: x }`.
This would do away with the backtracking and also permit
one-field structural records without having a disambiguating
comma in the tail. However, this would be both less ergonomic
as well as less consistent with the rest of the language.

As for not doing this, this feature is not so critical that
the absence of it would be a crisis, but it does smooth out
the language, provide more consistency, and better readability
when compared with the structurally typed positional tuples
that we already have.

## We once removed structural records, why re-add them?

[@pcwalton]: https://internals.rust-lang.org/t/why-were-structural-records-removed/1553/2

In 2015, [@pcwalton] noted that we once removed structural records from the
language because:

> They had three usability problems:
>
> 1. The fields had to be in the exact same order every time you constructed an
> instance of the record. This is because the compiler needed a canonical ordering.
>
> 2. They could not be recursive without the compiler doing fancy typechecking
> (coinductive IIRC), which was felt to be not worth the effort.
>
> 3. Field-level privacy can’t work with them.

Point 1. does not apply to this proposal. You can use any order you like when
writing out the type, an expression, or pattern, as long as all the fields are
there and of the same type once the fields have been sorted lexicographically. 

Point 2. and 3. still hold, but this is and has always been true of positional
tuples so it is nothing new. Note that both the type and the fields of a
structural record are public, just as with tuples.

Furthermore, while all types were akin to `#[repr(C)]` at the time,
the structural records proposed in this RFC are `#[repr(Rust)]`.

# Prior art
[prior-art]: #prior-art

## frunk

[`frunk`]: https://docs.rs/frunk/0.2.2/frunk/index.html

Arguably, the [`frunk`] crate presents itself as both an alternative and as
prior art in the domain of structural records. For example, we could write:

```rust
hlist![
    field!(name, "joe"),
    field!(age, 3),
    field!(is_admin, true)
];
```

to represent `{ name: "joe", age: 3, is_admin: true }`.

However, this requires some runtime overhead and first doing:

```rust
#[allow(non_camel_case_types)]
type name = (n, a, m, e);
#[allow(non_camel_case_types)]
type age = (a, g, e);
#[allow(non_camel_case_types)]
type is_admin = (i, s, __, a, d, m, i, n);
```

Pulling `frunk` in as a dependency just to this would also a be a non-starter
for many.

## Elm

[Elm]: https://elm-lang.org/docs/records

The functional language [Elm], which is known for emphasising learnability,
provides structural records with the syntax:

```elm
{ x = 3, y = 4 }
```

which you can type at:

```elm
{ x : Float, y : Float }
```

Elm also provides for polymorphic and extensible records (row polymorphism).

The FRU syntax is also supported and the usual "pattern matching follows
the expression syntax" also applies.

## Haskell

Haskell does not support structural records.
However, a number of libraries, similar in spirit to `frunk` do exist
which emulate these kinds of records. Examples include:

[SuperRecord]: https://www.athiemann.net/2017/07/02/superrecord.html

+ [SuperRecord]:

  ```haskell
  person =
    #name := "Alex"
    & #age := 23
    & rnil
  ```

+ [vinyl](https://hackage.haskell.org/package/vinyl)

# Unresolved questions
[unresolved-questions]: #unresolved-questions

1. Should we semantically reject `{ 0: expr, 1: expr }`?
   While it does simplify the parsing to accept it in the grammar and
   provides unification in the grammar, we might still want to reject
   the code post parsing because the style might not be useful in code,
   including in macros.

   This question can be deferred to stabilization since it is relatively minor.

2. Should `Ord` and `PartialOrd` be implemented for structural records?

   We could avoid doing so and instead treat structural records as having a set
   of unordered fields. Then, if anyone wants to use `Ord` or `PartialOrd`,
   they can use a nominal `struct` or positional tuple instead where the
   ordering is clearer.

   With respect to `(Partial)Eq`, `Hash`, and `Debug`, there is not really a
   problem because `Eq` does not rely on ordering, `Hash` only needs to uphold
   `x == y => hash(x) == hash(y)`, and for `Debug`, lexicographical ordering
   makes sense. Meanwhile, a user may rightfully be surprised if they write:

   ```rust
   let first = { foo: 1, bar: 2 };
   let second = { foo: 2, bar: 1};
   assert!(first < second);
   ```

   expecting `assert!(...)` to hold while in actuality it would fail.

   As this question is major, it should be resolved prior to accepting this RFC.

# Future possibilities
[future-possibilities]: #future-possibilities

There are a number of things we could consider in the future.
To keep the scope of this RFC limited to achieving feature parity with
tuples and named-field structs, we leave these ideas out of the RFC.
Not all of these ideas are of equal worth,
some are more worthwhile and some less.

## `repr(C)`

[@retep998]: https://internals.rust-lang.org/t/pre-rfc-unnamed-struct-types/3872/25
[@retep998_2]: https://internals.rust-lang.org/t/pre-rfc-unnamed-struct-types/3872/46

The structural records we've proposed in this RFC do not respect the order
they are written in and do not have a specified type layout according to that
non-existent order. This means that they would not be particularly useful for
FFI purposes. In 2016, [@retep998] noted that they would like to use such
records to *"closely match what the headers are doing"*. @retep988 later
noted this in a later [comment][@retep998_2].

In the future however, to support that use case, we could allow attributes
on types which would make `#[repr(C)] { x: A, y: B }` legal to write.
In that case, the order would be respected and so it could be used for FFI.
This would entail that `#[repr(C)] { x: A, y: B }` would be a different
type than `{ x: A, y: B }`.

## `HasField<"the_field">`

Using const generics, the compiler could automatically implement a trait
`Get<"the_field">` or `HasField<"the_field">` and the corresponding setter
for any type which has that field. This would offer a way to be polymorphic
over fields. The `HasField` trait could be defined as:

```rust
#[lang_item = "has_field"]
trait HasField<const Field: &'static str> {
    type Field;

    fn get(self) -> Self::Field;

    fn get_ref<'a>(&'a self) -> &'a Self::Field;

    fn get_mut<'a>(&'a mut self) -> &'a mut Self::Field;
}
```

Other designs may involve generic associated types (GATs) or some
form of polymorphism over type constructors (i.e. higher kinded types).

[RFC 2529]: https://github.com/rust-lang/rfcs/pull/2529

To take privacy into account, hidden implementations could be used as
in [RFC 2529].

[OverloadedRecordFields]: https://github.com/ghc-proposals/ghc-proposals/pull/6

This idea mainly comes from Haskell and is seen in the [SuperRecord]
package aforementioned. See [OverloadedRecordFields] for a deeper
discussion.

## Supercharging FRU

The current FRU (struct update) syntax is rather limited.
For example, you cannot write `MyType { ..x, ..y }` to get some fields
from both `x` and `y`. This is understandable, after all, since both `x`
and `y` are fully formed `MyType`s, this would just take all the values
from `x` without taking anything from `y`.

However, for structural records, this wouldn't be the case.
You could imagine merging two structural records together with:

```rust
let alpha = { a: 1, b: 2 };
let beta  = { c: 3, d: 4 };
let gamma = { ..alpha, ..beta };

assert_eq!(gamma, { a: 1, d: 2, c: 3, d: 4 });
```

## Coercions from smaller to larger records

The syntax above allowed us to grow structural records with more fields.
We could also offer a way to shrink them by allowing coercions from larger
to smaller structs. This would partially move (or copy) all the bits you
used. For example:

```rust
type Smaller = { foo: u8, bar: bool };
type Bigger = { foo: u8, bar: bool, baz: char };

let b: Bigger = { foo: 1, bar: true, baz: 'a' };
let s: Smaller = b; // OK! `b` has all the fields `Smaller` needs.
```

## Coercions to and from nominally typed structs

We could allow the creation of nominal structs with structural record
expressions. For example:

```rust
struct Foo { x: u8, y: u8 }

let bar = { x: 1, y: 2 };

let foo: Foo = bar;
```

and in the other direction:

```rust
struct Foo { x: u8, y: u8 }

let foo = Foo { x: 1, y: 2 };

let bar: { x: u8, y: u8 } = foo;
```

In both of these cases, privacy must be respected.

While this could be ergonomic, it could also reduce robustness.
