- Feature Name: `partial_types`
- Start Date: 2023-04-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

Partial types proposal is a generalization on "partial borrowing"-like proposals (more correct name is "partial not borrowing" or "qualified borrowing" since Rust allows partial borrowing already).

This proposal is a universal roadmap "how to do partial not consumption (including partial not borrowing) right", and not under the hood of the Rust compiler.

Partial Types is a **minimal** and full extension to the Rust Type System, which allows to safe control easily partial parameters and all kinds of partial not consumption.

Advantages: maximum type safety, maximum type control guarantee, no ambiguities, flexibility, usability and universality.


# Motivation
[motivation]: #motivation

Safe, Flexible controllable partial parameters for functions and partial not consumption (including partial not borrowing) are highly needed and this feature unlock huge amount of possibilities.

Partial borrowing is already possible in Rust, as partial referencing and partial moves.

But partial parameters are forbidden now, as qualified consumption: partial not borrowing, partial not referencing, partial not moving and partial initializing.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

_**Note**: I didn't comment type explanations (as is needed for compiler) for colorizing purposes only._

_**Note**: I use symbol `~` in code as a synonym of "equivalent" word only. It is not a Rust operator!_


## Partial types by type Integrity

```rust
// case (A1)
let foo : i16 = 0;
```

What is a **full type** of `foo`? Is it the `i16`? No, the `i16` is a sub-full_type in Rust type-system.
```rust
// case (A2)
<variable>  : <full_type>;
<variable>  : <type_clarification> <type>;
<variable>  : <sharing> <'lifetime> <mutability> <type>;
```

If we explicitly write **full type** (using _unused_ Rust names) we get:
```rust
// case (A3)
// FROM case (A1)
foo  : value 'static const i16;
```

That mean, that variable `foo` has the `i16` (sub-)type with next type-clarification: `const`(not `mut`) mutability, `value`(not `&`) sharing and `'static` lifetime.

I propose to extend type system by adding type integrity to clarification sub-type. So, our variable will have next full type:
```rust
// case (A4)
<variable>  : <sharing> <'lifetime> <mutability> <%integrity> <type>;

// case (A5)
// FROM case (A1)
foo  : value 'b const %a i16;
foo  : value 'static const %full i16;
foo  : %full i16;
foo  : i16;
```

Lifetime variants are `'static` (for static lifetime), `'_`(don't care lifetime) and any other `'b`(some "b" lifetime).

By the same analogy, integrity has similar names and meanings: `%full`(full integrity, soft keyword), `%_`(don't care how partial integrity is, soft keyword), `%empty` or `%!` (no integrity, soft keyword) and any other `%a`(some "a" integrity).

Symbol `%` percent mean percent or part of the whole thing (variable in our case).

_Note_: It is highly recommended to deprecate operator `%` as a remainder function (it is still no ambiguities to write "`\s+%\s+`"), and replace it with another operator (for example: `%mod` / `%rem` / `mod` / `rem`) to not to be confused by type integrity. 


## Traits with Integrity variants

We could already write Traits with **safe** virtual functions, that consumes virtual partial types having only variants of type integrity in Trait declaration
```rust
// case (B1)
pub trait Upd {
    type UpdType;

    fn summarize<%a>(&self: & %a Self) -> String;

    fn update_value<%a>(&mut self : &mut %a Self, newvalue: UpdType);

    fn update_sametype<%a, %b>(&mut self : &mut %a Self, &another: & %b Self);
}
```

Unfortunately, having variants of type integrity is not enough to write **safe** implementations or other non-virtual function declarations.


## Detailed Integrity

We need detailed integrity to write non-virtual specific typed parameters in function, including trait implementation.

An abstractions is added for integrity detailing, we assume that **every** variable is a `struct`-like objects (even if it is not).

We need for this some new quasi-fields and some field integrity (which should be soft keywords).

_Note: I do not comment types (as is needed for compiler) for colorizing purposes._

_Note: I use symbol `~` as a synonym of "equivalent" word only. It is not a Rust operator._
```rust
// case (C1)
let foo = 0i16;
    foo : i16  ~  %full i16;
    foo : %full i16  ~  %{self.*} i16;
    foo : %{self.*} i16  ~  %{self.self} i16;
```

Where :
 - `self` is an "access" to variable itself
 - `.*` is an "every field" quasi-field
 - `.self` quasi-field for primitive types, since `i16` is not a `struct` type

What's about structures? Almost the same:
```rust
struct Point {
    x: f64,
    y: f64,
    z: f64,
    t: f64,
    w: f64,
}

// case (C2)
let &mut p1 : &mut Point = Point {x:1.0, y:2.0, z:3.0, t:4.0, w:5.0};
    p1 : &mut Point  ~  &mut %full Point;
    p1 : &mut %{self.*} Point;
    p1 : &mut %{self.x, self.y, self.z, self.t, self.w} Point;
    p1 : &mut %{self.{x, y, z, w}} Point;
```

Where :
 - `.{<fld1>, <fld2>, }` is an field-set quasi-field

We assume, that each field could be in one of 2 specific field-integrity - `%fit` and `%deny`. 

We also must reserve as a keyword a `%miss` field-integrity for future ReExtendeded Partial Types, which allows to create **safe** self-referential types.

`%fit` is default field-integrity and it means we have an access to this field and could use it as we wish. But if we try to access `%deny` field it cause a compiler error.

```rust
// case (C3)
let foo = 0i16;
    foo : %{self.*} i16  ~  %{self.self} i16;
    foo : %{%fit self.*} i16  ~  %{%fit self.self} i16;
    foo : %{%fit self.*, %deny self._} i16;


// case (C4)
// FROM case (C2)
let &mut p1 : &mut Point = Point {x:1.0, y:2.0, z:3.0, t:4.0, w:5.0};
    p1 : &mut %{%fit self.*} Point  ~  &mut  %{%fit self.*, %deny self._};
    p1 : &mut %{%fit self.{x, y, z, w}} Point;
```

Where :
 - `%fit` integrity
 - `%deny` integrity
 - `._` is a "rest of fields" quasi-field

As we see, 
 - `%empty : %{%deny self.*}` or `%empty : %{}` integrity
 - `%full  : %{%fit  self.*}` or `%full  : %{self.*}` integrity


## Partial parameters

We add enough integrity, and could write partial parameters for non-virtual function declarations:
```rust
// case (D1)
fn re_ref_t (& p : & %{self.t, %ignore self._} Point) -> &f64 {
   &p.t
}

// case (D2)
fn refmut_w (&mut p : &mut %{self.w, %ignore self._} Point) -> &mut f64 {
   &mut p.w
}
```

Where :
 - `%ignore` is a "don't care which exactly" quasi filed-integrity (`%_` is a whole type integrity and it is unclear if we could use it in both contents)

But `%ignore self._` quasi-filed-integrity of quasi-field looks annoying, so we simplify a bit adding `%any : %ignore self._`.

```rust
// case (D3)
// FROM case (D1)
fn re_ref_t (& p : & %{self.t, %any} Point) -> &f64 {
   &p.t
}

// case (D4)
// FROM case (D2)
fn refmut_w (&mut p : &mut %{self.w, %any} Point) -> &mut f64 {
   &mut p.w
}

// case (D5)
struct PointExtra {
    x: f64,
    y: f64,
    saved_x: f64,
    saved_y: f64,
}

fn x_store(&mut p1 : &mut %{self.saved_x, %any} PointExtra, & p2 : & %{self.x, %any} PointExtra) {
    *p1.saved_x = *p2.x
}

fn x_restore(&mut p1 : &mut %{self.x, %any} PointExtra, & p2 : & %{self.saved_x, %any} PointExtra) {
    *p1.x = *p2.saved_x;
}
```

or use `where` clause if integrity is extra verbose:
```rust
// case (D6)
// FROM case (D5)

fn x_store(&mut p1 : &mut %fit_sv_x PointExtra, & p2 : & %fit_x PointExtra) 
    where %fit_sv_x : %{self.saved_x, %any},
          %fit_x : %{self.x, %any}
{
    *p1.saved_x = *p2.x
}

fn x_restore(&mut p1 : &mut %fit_x PointExtra, & p2 : & %fit_sv_x PointExtra) 
    where %fit_sv_x : %{self.saved_x, %any},
          %fit_x : %{self.x, %any}
{
    *p1.x = *p2.saved_x;
}
```

Implementation parameters are mostly same:
```rust
// case (D7)
impl Point {
    pub fn x_refmut(&mut self : &mut %{self.x, %any} Self) -> &mut f64 {
        &mut self.x
    }

    pub fn y_refmut(&mut self : &mut %{self.y, %any} Self) -> &mut f64 {
        &mut self.y
    }
}
```

We could also use multiple sub-parameters of same parameter
```rust
// case (D8)
    pub fn xy_swich(&mut self : &mut %{self.{x, y}, %any} Self) {
        let tmp = *self.x;
        *self.x = *self.y;
        *self.y = tmp;
    }
```

Now type integrity guarantee to compiler, that only some fields has an access inside function, but not the rest of them.
So, no extra lock on `self` is needed, only for `%fit` fields.

Now compiler can catch "out of scope parameter" errors
```rust
// case (D9)
    pub fn xt_refmut(&self : &mut %{self.xt, %any} Self) -> &mut f64 {
        //                               ^~~~~~
        // error: no field 'self.xt' on type `self`
        &mut self.xt
    }
```

Since using `%ignore` filed is **unsafe**, trying to use ignoring field is a compile error:
```rust
// case (D10)
    pub fn t_refmut(&self : &mut %{self.t, %any} Self) -> &mut f64 {
        &mut self.x
        //   ^~~~~~
        // error: cannot find value 'self.x' in this scope
    }
```

Compile could catch more dead code warnings
```rust
// case (D11)
    pub fn x_refmut(&self : &mut %{self.x, self.y, %any} Self) -> &mut f64 {
        //                                   ^~~~~~
        // warning: '#[warn(dead_code)]' field is never read: `self.y`
        &mut self.x
    }
```

## Several selfs

If we want to include `x_store` and `x_restore` from case (D5) for implementation we find something weird: we need **several** selfs!

Sure, they must be a keywords. It could be either `self1, self2, ..` or `self-1, self-2, ..` or `self#1, self#2` or `self_ref, self_refmut` or any other.

```rust
// case (E1)
trait St {

    fn x_store<%a, %b>(&mut self1: &mut %a Self, &self2: & %b Self);

    fn x_restore<%a, %b>(&mut self1: &mut %a Self, &self2: & %b Self);
}

// case (E2)
    pub fn x_store(&mut self1 : &mut %{self.x, %any} Self, &self2 : & %{self.saved_x, %any} Self) 
    {
        *self1.saved_x = *self2.x
    }

    pub fn x_restore(&mut self1 : &mut %{self.saved_x, %any} Self, &self2 : & %{self.x, %any} Self) {
        *self1.x = *self2.saved_x;
    }
```

Sure, if we use several `self`s, their fit fileds integrity cannot overlap!

```rust
// case (E3)
    pub fn x2_store(&mut self1 : &mut %{self.x, %any} Self, &self2 : & %{self.x, %any} Self) {
        //                                 ^~~~~~                         ^~~~~
        // error: cannot overlap fit-field 'self.x' on self1 and self2
        *self1.x = *self2.x;
    }
```

Fortunately, these additions is enough to write **any safe** function declarations.


## Partial not consumption

We wrote function declaration. Could we already partially not consume variables in arguments?

Fortunately, we could qualified consume implicit `self` arguments.

Unfortunately, implicit `self` argument is the only qualified consumed argument.

Exists 5 "pseudo-function" consumption for variables in expressions:
 - `&mut` - (mutable-)borrowing consumption
 - `&` - referential (immutable borrowing) consumption
 - `<_nothing>` - move consumption
 - `<StrucName>` initialized consumption
 - `.` access to the filed

Partial access to the field is already granted (exept arrays). 

Rust consumer use same names for action and for type clarifications, so we follow this style.

We need to add integrity filter to them, ignoring it mean `%full` filter (Ok, it is a bit unclear which is a default filter - `%full` or `%max`)!

`%full` means consumer consume all fields.

```rust
struct A { f1: String, f2: String, f3: String }
let mut x: A;

// case (F1)
let a: &mut String = &mut x.f1; // x.f1 borrowed mutably
let b: &String = &x.f2;         // x.f2 borrowed immutably
let c: &String = &x.f2;
// error:Can borrow again
let d: String = x.f3;           // Move out of x.f3

// case (F2)
// FROM case (F1)
let a: &mut String = &mut %full x.f1;
let b: &String = & %full x.f2;
let d: String =  %full x.f3;
```

Trying to consume `%deny` field is a compile error! The consumer DO NOT consume `%deny` EVER.

Resulted field integrity is the following:

| ↓filter / →integrity | `%fit`  | `%deny`   | `%hidden` |
|----------------------|---------|-----------|-----------|
| `%fit`               | `%fit`  | !ERROR    | !ERROR    |
| `%deny`              | `%deny` | `%deny`   | !ERROR    |
| `%hidden`            | !ERROR  | !ERROR    | `%hidden` |

```rust
struct S5 { f1: String, f2: String, f3: String, f4: String, f5: String }
let mut x: S5;

// case (F3)
let ref1: &mut String = &mut x.f1;
//
let ref_x23 = & %{self.f2, self.f3, %deny self._} x;
    //
    ref_x23 : & %{%fit self.{f2, f3}, %deny self.{f1, f4, f5}} S5;
    //
let move_x45 = %{self.{f4, f5}, %cut} x;
    //
    move_x45 : %{%fit self.{f4, f5}, %deny self.{f1, f2, f3}} S5;
```

But `%deny self._` quasi-filed-integrity of quasi-field looks annoying, so we simplify a bit adding `%cut : %deny self._`.

What to do if we wish to create a reference to `ref_x23`. Do we need to write explicitly an integrity or exists implicit way?

No, we could use `%max`(or `%id`) - qualified safe filter with maximum fit-fields, but technically is an `id` filter to variable integrity:

| var integrity   | `%max`   |
|-----------------|----------|
| `%fit`          | `%fit`   |
| `%deny`         | `%deny`  |
| `%hidden`       | `%hidden`|

Having this we could write next implicitly
```rust
// FROM case (F1)
    ref_x23: & %{%fit self.{f2, f3}, %deny self.{f1, f4, f5}} S5;

// case (F4)
let refref_x23 = & %max ref_x23;
//
    refref_x23: && %{%fit self.{f2, f3}, %deny self.{f1, f4, f5}} S5;
```

For function argument we add another filter `%min` - qualified safe filter with minimum fit-fields, but it refers not to variable integrity, but to parameter integrity, so we could use it in arguments consumption only! It is an compile error if `%min` is written outside of contents!

| param integrity  | `%min`   |
|------------------|----------|
| `%fit`           | `%fit`   |
| `%deny`          | `%deny`  | 
| `%ignore`        | `%deny`  | 
| `%hidden`        | `%hidden`| 

Implementations always consumes `self` by `%min` filter!

```rust
// FROM case (D3)
fn re_ref_t (& p : & %{self.t, %any} Point) -> &f64 {
   &p.t
}
let mut p1 : mut Point = Point {x:1.0, y:2.0, z:3.0, t:4.0, w:5.0};

// case (F5)
let reft = re_ref_t(& %min p1);


// case (F6)
    fn update_sametype<%a, %b>(&mut self : &mut %a Self, & another: & %b Self);
//
p1.update_sametype(& %min p2);


// case (F7)
    fn update_another<%a, %b>(& self : & %a Self, & mut another: & %b Self);
p3.update_sametype(&mut %min p2);
```


## Partially Initialized Variables

We must have an ability to create partially initilized variables. So we need to add a filter-integrity to a constructor

```rust
struct Point {
    x: f64,
    y: f64,
    z: f64,
    t: f64,
    w: f64,
}

// case (G1)
let p1_full = Point {x:1.0, y:2.0, z:3.0, t:4.0, w:5.0};
    //
    p1_full : Point  ~  %full Point;

// case (G2)
let p_x = %{self.x, %cut} Point {x:1.0};
    //
    p_x : %{%fit self.x, %deny self._} Point;
    //

let p_yz = %{self.{y,z}, %cut} Point {y:1.0, z: 2.0};
    //
    p_yz : %{%fit self.{y,z}, %deny self._} Point;
    //
```

Also it could be nice if constructor allows several filler variables (which do not overlap fit-fields)
```rust
// case (G3)
let p_xyz = %max Point {..p_x, ..p_yz};
    //
    p_xyz : %{%fit self.{x,y,z}, %deny self.{t,w}};

// case (G4)
let p2_full = Point {t:1.0, w:2.0, ..p_xyz};
    //
    p1_full : Point  ~  %full Point;
    //
```

A bit unclear how to fill unused fields, so we write unused values to a fill the type for tuple constructor

```rust
// case (G5)
let t4_02 = %{self.{0,2}, %cut} ("str", 1i32, &0u16, 0.0f32);
    //
    t4_02 : %{%fit self.{0,2}, %deny self.{1,3}} (&str, i32, &u16, f32);
```

Integrity filter could help to deconstruct types for matching:

```rust
// case (G6)
let opt_t4_1 = Some (%{self.1, %cut} ("str", 1i32, &0u16, 0.0f32));
    //
    opt_t4_1 : Option<%{%fit self.{1}, %deny self.{1,3}} (&str, i32, &u16, f32)>;
    //
    let Some (%{self.1, %cut} (_, ref y, _, _)) = opt_t4_1;
```

## Private fields

And finally, what to do with private fields?

If variable has private field, it is an  always `%hidden self.private` quasi-field.
```rust
pub struct HiddenPoint {
    pub x: f64,
    pub y: f64,
    z: f64,
    t: f64,
    w: f64,
}

// case (H1)
let p1 : HiddenPoint;
    p1 : %full HiddenPoint;
    p1 : %{%fit self.pub, %private} HiddenPoint;
    p1 : %{%fit self.{x, y}, %private} HiddenPoint;
    p1 : %{%fit self.{x, y}, %hidden<%full> self.private} HiddenPoint;
```

Where :
 - `.pub` is a "all public fields" quasi-field
 - `.private` is a "all private fields" quasi-field
 - `%hidden<%a>` - it is some specific `%a` quasi field integrity, but we have no access to specify it
 - `%private` is a shortcut for `%hidden<%full> self.private`

So, more fully we could write for struct witj private fields:
 - `%empty : %{%deny self.pub, %hidden<%empty> self.private}` integrity
 - `%full  : %{%fit  self.pub, %hidden<%full>  self.private}` integrity


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation


# Drawbacks
[drawbacks]: #drawbacks

- it is definitely not a minor change
- type system became much more complicated


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

(A) A lot of proposals that are alternatives to Partial Types in a whole:
 - Partial borrowing [issue#1215](https://github.com/rust-lang/rfcs/issues/1215)
 - View patterns [internals#16879](https://internals.rust-lang.org/t/view-types-based-on-pattern-matching/16879)
 - Permissions [#3380](https://github.com/rust-lang/rfcs/pull/3380)
 - Field projection [#3318](https://github.com/rust-lang/rfcs/pull/3318)
 - Fields in Traits [#1546](https://github.com/rust-lang/rfcs/pull/1546)
 - ImplFields [issue#3269](https://github.com/rust-lang/rfcs/issues/3269)

(B) Alternative for another names or corrections for Partial Types.
 - `%empty` or `%!` name
 - `self1, self2, ..` or `self-1, self-2, ..` or `self#1, self#2`. Or add only 2 specific selfs: `self_ref, self_refmut`


# Prior art
[prior-art]: #prior-art

Most languages don't have such strict rules for references and links as Rust, so this feature is almost unnecessary for them.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

Default qualified consumption is `%full` on Stage 1. It fully backward compatible and allow to switch cost-less to `%max` default!
But maybe it is not a good choice. As default argument consumption is `%full`, but not `%min`.

# Future possibilities
[future-possibilities]: #future-possibilities

We could add additional ReExtendeded Partial Types for **safe** Self-Referential Types. 

Theory of types do not forbid extension of Partial Type, but internal Rust representation of variables gives significant limitations on such action.

It is need the `%miss`(aka `%deny` but extendible) field integrity to initialized constructor consumption only. And additional "extender" `%%=`.

Pertly self-referential types example:
```rust
struct SR <T>{
    val : T,
    lnk : & T, // reference to self.val
}

// case (FP1)
let x = %{%miss self.lnk, %fit self._} SR {val : 5i32 };
    //
    x : %{%miss self.lnk, %fit self.val} SR<i32>
    //
x.lnk %%= & x.val;
    //
    x : SR<i32>  ~  %full SR<i32>
```
And even AlmostFully self-referential types:
And another shortcut `%unfill : %miss self._`

```rust
struct FSR <T>{
    val : T,
    lnk : & %{%deny self.lnk, %fit self.val} FSR<T>, 
    // reference to almost self!
}

// case (FP2)
let x = %{self.val, %unfill} FSR {val : 5i32 };
    //
    x : %{%miss self.lnk, %fit self.val} FSR<i32>
    //
x.lnk %%= & %max  x;
    //
    x : FSR<i32>  ~  %full FSR<i32>
```

First difficulty - `%max` is no longer `id`,  `%max(on %miss) ~ %deny`. Both `filter-%fit on %miss` and `filter-%ignore on %miss` must cause a compiler error for 3 main consumers.

Second and most difficult, that `return` consumption (yes, 6th type of consumers) from function could preserve `%miss`, so also we need filter `%max_miss`, where `%max_miss(on %miss) ~ %miss`!


```rust
// case (FP3)
// FROM case (FP2)
fn create_var()-> %{%miss self.lnk, %fit self._} FSR {
    let x = %{self.val, %unfill} FSR {val : 5i32 };
        //
        x : %{%miss self.lnk, %fit self.val} FSR<i32>
        //
    %max_miss return x; 
    // filter integrity before 'return' to not to confused with `move` consumer!
}

let y = create_var();
y.lnk %%= & %max  y;
    //
    y : FSR<i32>  ~  %full FSR<i32>
```
