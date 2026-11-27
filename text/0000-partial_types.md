- Feature Name: `partial_types`
- Start Date: 2024-12-06
- RFC PR: [rust-lang/rfcs#3736](https://github.com/rust-lang/rfcs/pull/3736)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

This proposal is universal flexible tool to work **safe** and **zero cost binary** with partial Structs and Tuples in parameters, arguments, references and  borrows.

Advantages: maximum type safety, maximum type control guarantee, no ambiguities, zero-cost-binary, flexibility, usability and universality.


# Motivation
[motivation]: #motivation

A lot of rust code where I need a struct mutable borrowed and stored paralelly to other borrows of the same struct but different fields. And partial borrowing is a good solution for these problems and they are highly needed.

Partial Types proposal is a generalization on "partial borrowing"-like proposals. Safe, Flexible controllable partial parameters for functions and partial consumption (including partial borrowing) are highly needed.

Partial Types extension gives to Product Types (`PT = T1 and T2 and T3 and ..`), Structs and Tuples first of all, a good **mathematical guarantee** to borrow-checker that borrowing the whole variable with partial type and pretending to borrow just permitted fields is **fully safe** (without using `unsafe`).
```rust
struct StructABC { a: u32, b: i64, c: f32, }

// function with partial parameter
fn ref_a (s : & StructABC.{a}) -> &u32 {
    &s.a
}

let s = StructABC {a: 4, b: 7, c: 0.0};

// partial expression, partial reference and partial argument
let sa = ref_a(& s.{a});
```

And since it is a guarantee by **type**, not by **values**, it has _zero cost_ in binary! Any type error is a compiler error, so no errors in the runtime.

This extension is not only fully backward-compatible, but is fully forward-compatible! Forward-compatibility is an ability to use updated functions old way.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Partiality of type (or partial type access) is written as `Path.{fld1, fld2, fld3}` after Path (Type name), where `fld1`, `fld2`, .. are only permitted to read (and to write if variable is `mut`) fields of this type regardless of visibility, the rest of fields are forbidden to read and write and unset.

Inverse partiality of type (or partial type access) is written as `Path.{off fld1, fld2, fld3}` after Path (Type name), where `off` is a new keyword and `fld1`, `fld2`, .. are only forbidden to read fields of this type regardless of visibility, the rest of fields are permitted to read (and maybe write) and unset.

But the same time fields that are forbidden to read and write it is totally Ok to borrow, re-borrow, move, re-move, without any consequences - because the Compiler guarantee that in safe mode it is impossible to use such fields. It is a compile error if someone try to access it.

## Partial Structs and Tuples

For Product Types `PT = T1 and T2 and T3 and ..`), for structs, tuples we need not only partiality of a type, but also **"partial access" expression**: `Expr .{fld1, fld2, fld3}`, where `fld1`, `fld2`, .. are permitted fields of this type regardless of visibility, the rest of fields are forbidden.

Alternative expression is `Expr .{off fld1, fld2, fld3}`, where `fld1`, `fld2`, .. are forbidden fields of this type regardless of visibility, the rest of fields are permitted. Alternative syntax is useful to avoid of using private fields and if a list of permitted fields is much longer then list of forbidden fields.

Advantages of using inverse partiality: for better ergonomics and avoid using private fields names directly
```rust
//  // For better ergonomics
let t1 = s10.{fld1, fld2, fld3, fld4, fld5, fld6, fld7, fld8};
//  just "allowed" fields
//  t1 : S10.{fld1, fld2, fld3, fld4, fld5, fld6, fld7, fld8}

let t2 = s10.{off fld9, fld10};
//  just "forbidden" fields
//  t2 : S10.{fld1, fld2, fld3, fld4, fld5, fld6, fld7, fld8}


//  // For avoid using private fields names directly
let fpubs  = &foo.{pubfld1, pubfld2, pubfld3,};
//  just "allowed" fields
//  fpubs : Foo.{pubfld1, pubfld2, pubfld3,}

let fprivs = &foo.{off pubfld1, pubfld2, pubfld3,};
//  just "forbidden" fields
//  fprivs : Foo.{privfld1, privfld2, privfld3, privfld4, privfld5,}
```

One step to partial borrows Structs and Tuples.
```rust
struct Point {
    x: f64,
    y: f64, 
    was_x: f64, 
    was_y: f64,
    state : f64,
}
let mut p1 = Point {x:1.0, y:2.0, was_x: 4.0, was_y: 5.0, state: 12.0};
    // p1 : Point

let ref_p1was = &mut p1.{wax_x, was_y};
    // ref_p1was : &mut Point.{was_x, was_y}

let ref_p1now = &mut p1.{x, y};
    // ref_p1now : &mut Point.{x, y}
```
It is simple and will be possible. 

It is easy to write functions, which consume partial parameters:
```rust
impl Point {
    fn ref_x (self : & Self.{x}) -> &f64 {
        &self.x
    }

    fn refmut_y (self : &mut Self.{y}) -> &mut f64 {
        &mut self.y
    }
}
let ref_p1x = p1.ref_x();
let refmut_p1y = p1.refmut_y();
```
Here, since the methods' `self` types are partial references, only the needed fields are borrowed, so the call to `refmut_y` doesn't invalidate `ref_p1x`.


It is expected, that `self` is **always** cut partiality of argument by same partiality as self-parameter by partial expression before use (even if implicit rules are off)!

Pseudo-rust:
```rust
fn ref_xy (self : & Self.{ x, y}) -> &f64 {
    /*  */
}

p1.ref_xy();
// which "desugar"
Point::ref_xy(& p1.{x, y});
```

Product-Typed argument type must match with function parameter type or argument type could has **more** permitted partiality then parameter type.
```rust
// Struct ~ Product Type
struct S4 {a : i32, b : i32, c : i32, d : i32}

fn do_sab(s : S4.{a, b}) { /* .. */ }

let s = S4 {a: 6, b: 7, c: 8, d: 9};

do_sab(s.{a});       // s.{a} - error
do_sab(s.{b});       // s.{b} - error
do_sab(s.{a, b});    // s.{a, b} - Ok
do_sab(s.{a, b, c}); // s.{a, b, c} - Ok
do_sab(s);           // s.{*} - Ok
```


# Reference-level explanation

The core Idea of this proposal is "Proxy Borrowing" - we borrow the whole variable, but borrow-checker pretends it borrow just permitted/allowed fields.

Automatically Type-checker gives a mathematical guarantee, because all denied/forbidden fields remain intact! 

And this mean, that Proxy Borrowing borrowing is fully **safe** and _zero cost_ in binary.

## Proxy Borrowing

Borrowing rules for partial types:

`PermittedField` field borrowing rules are ordinary Rust rules. New variable borrows the whole variable (with partial type), but checker pretends it borrows just permitted fields of this variable.

Not-`PermittedField` filed is always is ready to borrow regardless if origin field is denied(by move, by reference, by borrow).

When we write a code for full or partial borrow, the link of object itself returns, but borrow-checker checks to borrow of permitted fields only.

This new mechanism of is simple and universal.

```rust
struct S4 {a : i32, b : i32, c : i32, d : i32}
let s = S4 {a : 5, b: 6, c: 7, d: 8};
    // s : S4

let r_sd = & s.{d};
    // r_sd : & S4.{d}
    //
    // borrow-checker check just for &s.d

let mut mr_sabc = &mut s.{a, b, c};
    // mr_sabc : &mut S4.{a, b, c}
    //
    // borrow-checkercheck just for &mut s.a, &mut s.b, &mut s.c

let rr_sbc = & mr_sabc.{b, c};
    // rr_sbc : && S4.{b, c}
    //
    // borrow-checker check just for &mr_sabc.b, &mr_sabc.c

let mut mrr_sa = &mut mr_sabc.{a};
    // mrr_sa : &&mut S4.{a}
    //
    // borrow-checker check just for &mut mr_sabc.a
```

## Syntax

Second, but still important - syntax.

### Partiality Syntax

Minimal Partiality we could write:
```
Partiality:      .{ PartialFields* }
PartialFields:   PartialField1 (, PartialField )* ,?
PartialField1:   off? PartialField
PartialField:    PermittedField
PermittedField:  IDENTIFIER | TUPLE_INDEX
```

If we wish to describe nested partial structs, we must have a bit more complex Partiality:

```
PartialField:    PermittedField Partiality?
```

Example of using nested partiality:
```rust
struct Foo { a: i32, bar: Bar, }

struct Bar { b: f32, c: String, }

impl Foo {
    fn baz(&self.{a, bar.{c}}) {
    }
}
```
`off` (or we could choose another name) is a local keyword for syntax of inverse partiality.

### Partial Struct syntax

Syntax is needed to Struct Type - is update `TypePath`
```
TypePath:   ::? TypePathSegment (:: TypePathSegment)* Partiality?
```

### Partial Tuple syntax

For Tuple Type we need to update `TupleType`
```
TupleType:  ( ) | ( ( Type , )+ Type? ) Partiality?
```

### Partial Expression syntax

For Expression we need create new kind of Expression:
```
PartialExpression:   Expression Partiality
```

and include it into `ExpressionWithoutBdeny`:
```
ExpressionWithoutBdeny:   ... | FieldExpression | PartialExpression | ...
```


## Logic Scheme

Third, but still important - Logic Scheme.

For pseudo-rust we suppose, partiality is a `HashSet` of permitted field-names.

Common rules:
```rust
fn bar(v : SomeType.{'type_prtlty}) 
{ /* .. */ }

let v : SomeType.{'var_prtlty}; 
```
Then:

(1) If `SomeType` is not supported type (neither Struct nor Tuple) then Error.

(2) If partiality has no extra field-names `type_prtlty.is_subset(full_prtlty)` it compiles, otherwise Error.

(3) If `var_prtlty.is_subset(full_prtlty)` it compiles, otherwise Error.

(4) If `type_prtlty.is_empty()` or `var_prtlty.is_empty()` (if they are explicitly written as '`.{}`') then Error

Maybe (4) is too strong limitation and it is handy to check just the address for comparison and wasn't allowed to read/write any fields.
Then, (4) is a part of "Unresolved questions"

### Partial Struct and Tuples Logic Scheme


Let we have (pseudo-rust) and `st_param_prtlty` and `st_arg_prtlty` are `HashSet` of permitted field-names: 
```rust
fn bar(s : SomeStructOrTuple.{'st_param_prtlty}) 
{ /* .. */ }

let s : SomeStructOrTuple.{'st_arg_prtlty}; 
bar(s);

let rsp = & s.{'expr_prtlty};

impl SomeStructOrTuple.{'st_impl_prtlty} {
    fn foo(self : Self.{'st_slf_prtlty}) 
	{ /* .. */ }
}

s.foo();
// (4) desugars into:
SomeStructOrTuple.{'st_impl_prtlty}::foo(s.{'st_slf_prtlty});
```
Then:

(1) If `st_arg_prtlty.is_superset(st_param_prtlty)` it compiles, otherwise Error.

(2) If `expr_prtlty.is_subset(st_arg_prtlty)` it compiles, otherwise Error.

(3) If `st_slf_prtlty.is_subset(st_impl_prtlty)` it compiles, otherwise Error.

(4) Updating desugaring for `self` (and `Rhs`) variables.

Desugaring `s.foo()` into `SomeStructOrTuple.{'st_impl_prtlty}::foo(s.{'st_slf_prtlty})` .

(5) It has **no sense** to have several implementation of same product-type and different partiality. 

(6) Anyway let we have several implementations for same type, but different partiality. And `all_st_impl_prtlty` is an `array` of each `st_impl_prtlty`.

If `all_st_impl_prtlty.iter().any(|&sip| st_arg_prtlty.is_subset(sip))` it compiles, otherwise Error.

(8) If `1 == all_st_impl_prtlty.iter().fold(0, |acc, &sip| if st_arg_prtlty.is_subset(sip) {acc+1} else {acc})` it compiles, otherwise ?Error.

We expect that just one "implementation" partiality is match and we choose it for calling a method.


# Drawbacks
[drawbacks]: #drawbacks

- it is definitely not a minor change
- type system became much more complicated


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

 A lot of proposals that are alternatives to Partial Product Types in a whole:
 - Partial Types (v2) [#3426](https://github.com/rust-lang/rfcs/pull/3426)
 - Partial Mutability [#3428](https://github.com/rust-lang/rfcs/pull/3428)
 - Partial Types [#3420](https://github.com/rust-lang/rfcs/pull/3420)
 - Partial borrowing [issue#1215](https://github.com/rust-lang/rfcs/issues/1215)
 - View patterns [internals#16879](https://internals.rust-lang.org/t/view-types-based-on-pattern-matching/16879)
 - Permissions [#3380](https://github.com/rust-lang/rfcs/pull/3380)
 - Fields in Traits [#1546](https://github.com/rust-lang/rfcs/pull/1546)
 - ImplFields [issue#3269](https://github.com/rust-lang/rfcs/issues/3269)


# Prior art
[prior-art]: #prior-art

Most languages don't have such strict rules for references and links as Rust, so this feature is almost unnecessary for them.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

It would be wonderfull to have some pseudo-field, which meant "all not public(private) fields". Maybe `!pub` is Ok.
```rust
let fprivs = &foo.{!pub};
//  fprivs : Foo.{privfld1, privfld2, privfld3, privfld4, privfld5,}
```

# Future possibilities
[future-possibilities]: #future-possibilities

Adding "partiality" opens wide variety of future possibilities.


## Partial Mutability

*partly independent sub-proposal*.

For full flexibility of using partial borrowing partial mutability is needed!

For Product Partial Types (structs, tuples) we use "partial mutability" expression: `mut .{fld1, fld2, ..}`, where `fld1`, `fld2`, .. are mutable fields of this type, the rest of fields are immutable(constant).
 
Partly mutable variables become possible for Product Partial Types:
```rust
struct S4 {a : i32, b : i32, c : i32, d : i32}

let mut.{a}       s_ma   = S4 {a: 6, b: 7, c: 8, d: 9};
let mut.{b, c}    s_mbc  = S4 {a: 6, b: 7, c: 8, d: 9};
let mut.{a, c, d} s_macd = S4 {a: 6, b: 7, c: 8, d: 9};
```

It is also possible to make partial-mutable references.

Not-`PermittedField` filed is always is ready to mutable and immutable borrow regardless if origin field is denied(by move, by reference, by borrow), is visible, is mutable:
```rust
   fn mab_s(s : &mut.{a,b} S4) 
   { /* ... */ }
   
   mab_s(&mut.{a,b} s_macd);
```
It is expected, that `&mut.{..}` is a third type of borrowing!

Example with full flexibility of using partial borrowing together with partial mutability

```rust
impl Point {
   pub fn mx_rstate(self : &mut.{x} Self.{x, state}) 
   { /* ... */ }

   pub fn my_rstate(self : &mut.{y} Self.{y, state}) 
   { /* ... */ }

   pub fn mxy_rstate(self : &mut.{x,y} Self.{x, y, state}) { 
    /* ... */
    self.{x, state}.mx_rstate(); // explicit
    self.mx_rstate(); // same implicit
    /* ... */
    self.{y, state}.my_rstate(); // explicit
    self.my_rstate(); // same implicit
    /* ... */
   }
}
```


## Explicit Off Fields

This extension is not a mandatory. Tuple type has "naked" structure, so it would be handy have more pretty visuals, instead of mark all permitted fields in "partiality", write `off` before denied field.
```rust
let t :: (i32, &u64, f64, u8).{1,3};
// same as
let t :: (off i32, &u64, off f64, u8);
```

This extension is not just pretty, but useful for Tuples.


## Partial Types to Sum Types (Enums)

Partial Types extension gives to Sum Types (`ST = T1 or T2 or T3 or ..`), Enums first of all, a good tool for "partial functions".
```rust
enum EnumABC { A(u32), B(i64), C(f32), }

// function with partial parameter Enum
fn print_A(a: EnumABC.{A}) {
    println!("a is {}", a.0);
}

let ea = EnumABC::A(7);
//  ea : EnumABC.{A} inferred

print_A(ea);
```
