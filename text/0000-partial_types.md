- Feature Name: `partial_types`
- Start Date: 2023-04-18
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

Partial types proposal is a generalization on "partial borrowing"-like proposals (more correct name is "partial not borrowing" or "qualified borrowing" since Rust allows partial borrowing already).

This proposal is a universal road-map "how to do partial not consumption (including partial not borrowing) right", and not under the hood of the Rust compiler.

Partial Types is a **minimal** and full extension to the Rust Type System, which allows to safe control easily partial parameters and all kinds of partial not consumption.

Advantages: maximum type safety, maximum type control guarantee, no ambiguities, flexibility, usability and universality.


# Motivation
[motivation]: #motivation

Safe, Flexible controllable partial parameters for functions and partial not consumption (including partial not borrowing) are highly needed and this feature unlock huge amount of possibilities.

Partial borrowing is already possible in Rust, as partial referencing and partial moves.

But partial parameters are forbidden now, as qualified consumption: partial not borrowing, partial not referencing, partial not moving and partial initializing.

This proposal
1) It is full backward-compatible.
2) It adds some **safe** flexibility to **safe** code by **safe** methods.
3) It has simplicity in binary - Type access just say by type to compiler, that some fields are forbidden to use _for everyone ever_. And that allows to use ordinary references as "partial" and ordinal variables as "partial". No extra actions with variables or pointers are needed.
4) Any type error is a compiler error, all types are erased after type-check, so no extra-cost in binary is needed.
5) It has universal rule - that mean minimal special cases on implementation.
6) It is minimal universal-extension - all other proposals propose less than this with  more or same cost

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

## Partial types by type access

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

I propose to extend type system by adding type access to sub-type. So, our variable will have next full type:

```rust
// case (A4)
<variable>  : <type_clarification> <%access> <type>;
<variable>  : <sharing> <'lifetime> <mutability> <%access> <type>;

// case (A5)
// FROM case (A1)
// foo  : %full i16;
// foo  : i16;
```

Lifetime variants are `'static` (for static lifetime), `'_`(don't care lifetime) and any other `'b`(some "b" lifetime).

By the same analogy, access has similar names and meanings: `%full`(full access, soft keyword), `%_`(don't care how partial access is, soft keyword), `%empty` or `%!` (no access, soft keyword) and any other `%a`(some "a" access).

Symbol `%` percent mean percent or part of the whole thing (variable in our case).

_Note: It is highly recommended to deprecate operator `%` as a remainder function (it is still no ambiguities to write "`\s+%\s+`"), and replace it with another operator (for example: `%mod` / `%rem` / `mod` / `rem`) to not to be confused by type access._


## Traits with access variants

We could already write Traits with **safe** abstract functions (with no body), that consumes partial types having only variants of type access in Trait declaration
```rust
// case (B1)
pub trait Upd {
    type UpdType;

    fn summarize<%a>(&self: & %a Self) -> String;

    fn update_value<%a>(&mut self : &mut %a Self, newvalue: UpdType);

    fn update_sametype<%a, %b>(&mut self : &mut %a Self, &another: & %b Self);
}
```

Unfortunately, having variants of type access is not enough to write **safe** implementations or other non-abstract function declarations.

## Detailed access
We need detailed access to write non-abstract specific typed parameters in function, including trait implementation.

We need for this some new quasi-fields and some field access (which should be soft keywords).

### Detailed Struct Type

What's about structures?
```rust
struct Point {
    x: f64,
    y: f64,
    z: f64,
    t: f64,
    w: f64,
}

// case (C1)
let &mut p1 : &mut Point = Point {x:1.0, y:2.0, z:3.0, t:4.0, w:5.0};
    //
    // p1 : &mut Point;
    // p1 : &mut %full Point;
    // p1 : &mut %{Self::*} Point;
    // p1 : &mut %{Self::x, Self::y, Self::z, Self::t, Self::w} Point;
    // p1 : &mut %{Self::{x, y, z, w}} Point;
```

Where :
 - `Self` is an "link" to variable type itself
 - `::*` is an "every field" quasi-field
 - `::{<fld1>, <fld2>, }` is an field-set quasi-field

We assume, that each field could be in one of two specific field-accesss - `%permit` and `%deny`.

We also must reserve as a keyword a `%miss` field-access for future ReExtendeded Partial Types, which allows to create **safe** self-referential types.

`%permit` is default field-access and it means we have an access to this field and could use it as we wish. But if we try to access to `%deny` field it cause a compiler error.

```rust
// case (C2)
// FROM case (C1)
let &mut p1 : &mut Point = Point {x:1.0, y:2.0, z:3.0, t:4.0, w:5.0};
    //
    // p1 : &mut %{%permit Self::*} Point;
    // p1 : &mut %{%permit Self::*, %deny Self::_};
    // p1 : &mut %{%permit Self::{x, y, z, w}} Point;
```

Where :
 - `%permit` access
 - `%deny` access
 - `::_` is a "rest of fields" quasi-field

As we see, 
 - `%empty : %{%deny Self::*}` or `%empty : %{}` access
 - `%full  : %{%permit Self::*}` or `%full  : %{Self::*}` access

### Detailed Primitive Types

Primitive types (numbers, units) do not have internal structures. Their access is always `%full`

For Primitive Partial Types we assume that **every** variable is a `struct`-like objects (even if it is not) and has a single quasi-field - `::self`.

It is a compile error if we try to `%deny` a `::self` field!

```rust
// case (C3)
let foo = 0i16;
    //
    // foo : i16  
    // foo : %full i16;
    // foo : %{Self::*} i16;
    // foo : %{Self::self} i16;
    // foo : %{%permit Self::self} i16;
```

### Detailed Tuples

For Tuples we assume that **every** variable is a `struct`-like objects (even if it is not) and has unnamed numbered fields.

It is a compile error if we try to `%deny` a `::self` field!

```rust
// case (C4)
let bar = (0i16, &5i32, "some_string");
    //
    // bar : (i16, &i32, &str);
    // bar : %full (i16, &i32, &str);
    // bar : %{Self::*} (i16, &i32, &str);
    // bar : %{%permit Self::{0,1,2}} (i16, &i32, &str);
```

### Detailed Arrays

For Arrays we assume that **every** variable is a `tuple`-like objects (even if it is not) and has unnamed numbered fields.

Unfortunately, Arrays are a bit magical, so it is _unclear_ if we could represent it access like a tuple access.

### Detailed Enum Type

What's about Enums?

It is more complicated, then `struct` types, because we grant some **type** access, not **value** access!

So, all possible constructors are permitted! But, we could deny sub-fields!
```rust
enum WebEvent {
    PageLoad,
    PageUnload,
    
    KeyPress(char),
    Paste(String),
    
    Click { x: i64, y: i64 },
}

// case (C5)
let a = WebEvent::PageLoad;
    //
    // a : WebEvent;
    // a : %full WebEvent;
    // a : %{Self::*::*} WebEvent;
    // a : %{%permit Self::{PageLoad, PageUnload}::self, %permit Self::{KeyPress, Paste}::0, %permit Self::Click::{x, y}} WebEvent;
```
where
 - `::self` quasi-field for unit types, since `PageLoad`/`PageUnload` is not a `struct`
 - `::0` is like mono-tuple field for `KeyPress(char)` and `Paste(String)`
 
 It is a compile error if we try to `%deny` a `::self` field!
 
## Partial parameters

We add enough access, and could write partial parameters for non-abstract function declarations:
```rust
// case (D1)
fn re_ref_t (& p : & %{Self::t, %ignore Self::_} Point) -> &f64 {
   &p.t
}

// case (D2)
fn refmut_w (&mut p : &mut %{Self::w, %ignore Self::_} Point) -> &mut f64 {
   &mut p.w
}
```

Where :
 - `%ignore` is a "don't care which exactly" quasi filed-access (`%_` is a whole type access and it is unclear if we could use it in both contents)

But `%ignore Self::_` quasi-filed-access of quasi-field looks annoying, so we simplify a bit adding `%any : %ignore Self::_`.

```rust
// case (D3)
// FROM case (D1)
fn re_ref_t (& p : & %{Self::t, %any} Point) -> &f64 {
   &p.t
}

// case (D4)
// FROM case (D2)
fn refmut_w (&mut p : &mut %{Self::w, %any} Point) -> &mut f64 {
   &mut p.w
}

// case (D5)
struct PointExtra {
    x: f64,
    y: f64,
    saved_x: f64,
    saved_y: f64,
}

fn x_store(&mut p1 : &mut %{Self::saved_x, %any} PointExtra, & p2 : & %{Self::x, %any} PointExtra) {
    *p1.saved_x = *p2.x
}

fn x_restore(&mut p1 : &mut %{Self::x, %any} PointExtra, & p2 : & %{Self::saved_x, %any} PointExtra) {
    *p1.x = *p2.saved_x;
}
```

or use `where` clause if access is extra verbose:
```rust
// case (D6)
// FROM case (D5)

fn x_store(&mut p1 : &mut %permit_sv_x PointExtra, & p2 : & %permit_x PointExtra) 
    where %permit_sv_x : %{Self::saved_x, %any},
          %permit_x : %{Self::x, %any}
{
    *p1.saved_x = *p2.x
}

fn x_restore(&mut p1 : &mut %permit_x PointExtra, & p2 : & %permit_sv_x PointExtra) 
    where %permit_sv_x : %{Self::saved_x, %any},
          %permit_x : %{Self::x, %any}
{
    *p1.x = *p2.saved_x;
}
```

Implementation parameters are mostly same:
```rust
// case (D7)
impl Point {
    pub fn x_refmut(&mut self : &mut %{Self::x, %any} Self) -> &mut f64 {
        &mut self.x
    }

    pub fn y_refmut(&mut self : &mut %{Self::y, %any} Self) -> &mut f64 {
        &mut self.y
    }
}
```

We could also use multiple sub-parameters of same parameter
```rust
// case (D8)
    pub fn xy_swich(&mut self : &mut %{Self::{x, y}, %any} Self) {
        let tmp = *self.x;
        *self.x = *self.y;
        *self.y = tmp;
    }
```

Now type access guarantee to compiler, that only some fields has an access inside function, but not the rest of them.
So, no extra lock on `self` is needed, only for `%permit` fields.

Now compiler can catch "out of scope parameter" errors
```rust
// case (D9)
    pub fn xt_refmut(&self : &mut %{Self::xt, %any} Self) -> &mut f64 {
        //                               ^~~~~~
        // error: no field 'Self::xt' on type `Self`
        &mut self.xt
    }
```

Since using `%ignore` filed is **unsafe by type** (we have no guarantee, that some field is permitted), trying to use ignoring field is a compile error:
```rust
// case (D10)
    pub fn t_refmut(&self : &mut %{Self::t, %any} Self) -> &mut f64 {
        &mut self.x
        //   ^~~~~~
        // error: cannot find value 'Self::x' in this scope
    }
```

Compile could catch more dead code warnings
```rust
// case (D11)
    pub fn x_refmut(&self : &mut %{Self::x, Self::y, %any} Self) -> &mut f64 {
        //                                   ^~~~~~
        // warning: '#[warn(dead_code)]' field is never read: `Self::y`
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
    pub fn x_store(&mut self1 : &mut %{Self::x, %any} Self, &self2 : & %{Self::saved_x, %any} Self) 
    {
        *self1.saved_x = *self2.x
    }

    pub fn x_restore(&mut self1 : &mut %{Self::saved_x, %any} Self, &self2 : & %{Self::x, %any} Self) {
        *self1.x = *self2.saved_x;
    }
```

Sure, if we use several `self`s, their fit fileds access cannot overlap!

```rust
// case (E3)
    pub fn x2_store(&mut self1 : &mut %{Self::x, %any} Self, &self2 : & %{Self::x, %any} Self) {
        //                                 ^~~~~~                         ^~~~~
        // error: cannot overlap fit-field 'Self::x' on self1 and self2
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

We need to add access filter to them, ignoring it mean `%full` filter (Ok, it is a bit unclear which is a default filter - `%full` or `%max`)!

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

Resulted field access is the following:

| ↓filter / →access | `%permit` | `%deny`   | `%hidden` |
|-------------------|-----------|-----------|-----------|
| `%permit`         | `%permit` | !ERROR    | !ERROR    |
| `%deny`           | `%deny`   | `%deny`   | !ERROR    |
| `%hidden`         | !ERROR    | !ERROR    | `%hidden` |

```rust
struct S5 { f1: String, f2: String, f3: String, f4: String, f5: String }
let mut x: S5;

// case (F3)
let ref1: &mut String = &mut x.f1;
//
let ref_x23 = & %{Self::f2, Self::f3, %deny Self::_} x;
    //
    // ref_x23 : & %{%permit Self::{f2, f3}, %deny Self::{f1, f4, f5}} S5;
    //
let move_x45 = %{Self::{f4, f5}, %cut} x;
    //
    // move_x45 : %{%permit Self::{f4, f5}, %deny Self::{f1, f2, f3}} S5;
```

But `%deny Self::_` quasi-filed-access of quasi-field looks annoying, so we simplify a bit adding `%cut : %deny Self::_`.

What to do if we wish to create a reference to `ref_x23`. Do we need to write explicitly an access or exists implicit way?

No, we could use `%max`(or `%id`) - qualified safe filter with maximum profit-fields, but technically is an `id` filter to variable access:

| var access   | `%max`    |
|--------------|-----------|
| `%permit`    | `%permit` |
| `%deny`      | `%deny`   |
| `%hidden`    | `%hidden` |

Having this we could write next implicitly
```rust
// FROM case (F1)
    // ref_x23: & %{%permit Self::{f2, f3}, %deny Self::{f1, f4, f5}} S5;

// case (F4)
let refref_x23 = & %max ref_x23;
//
    // refref_x23: && %{%permit Self::{f2, f3}, %deny Self::{f1, f4, f5}} S5;
```

For function argument we add another filter `%min` - qualified safe filter with minimum profit-fields, but it refers not to variable access, but to parameter access, so we could use it in arguments consumption only! It is an compile error if `%min` is written outside of contents!

| param access  | `%min`    |
|---------------|-----------|
| `%permit`     | `%permit` |
| `%deny`       | `%deny`   | 
| `%ignore`     | `%deny`   | 
| `%hidden`     | `%hidden` | 

Implementations always consumes `self` by `%min` filter!

```rust
// FROM case (D3)
fn re_ref_t (& p : & %{Self::t, %any} Point) -> &f64 {
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

We must have an ability to create partially initilized variables. So we need to add a filter-access to a constructor

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
    // p1_full : Point;
    // p1_full : %full Point;

// case (G2)
let p_x = %{Self::x, %cut} Point {x:1.0};
    //
    // p_x : %{%permit Self::x, %deny Self::_} Point;
    //

let p_yz = %{Self::{y,z}, %cut} Point {y:1.0, z: 2.0};
    //
    // p_yz : %{%permit Self::{y,z}, %deny Self::_} Point;
    //
```

Also it could be nice if constructor allows several filler variables (which do not overlap fit-fields)
```rust
// case (G3)
let p_xyz = %max Point {..p_x, ..p_yz};
    //
    // p_xyz : %{%permit Self::{x,y,z}, %deny Self::{t,w}};

// case (G4)
let p2_full = Point {t:1.0, w:2.0, ..p_xyz};
    //
    // p2_full : Point;
    // p2_fill : %full Point;
```

A bit unclear how to fill unused fields, so we write unused values to a fill the type for tuple constructor

```rust
// case (G5)
let t4_02 = %{Self::{0,2}, %cut} ("str", 1i32, &0u16, 0.0f32);
    //
    // t4_02 : %{%permit Self::{0,2}, %deny Self::{1,3}} (&str, i32, &u16, f32);
```

access filter could help to deconstruct types for matching:

```rust
// case (G6)
let opt_t4_1 = Some (%{Self::1, %cut} ("str", 1i32, &0u16, 0.0f32));
    //
    // opt_t4_1 : Option<%{%permit Self::{1}, %deny Self::{1,3}} (&str, i32, &u16, f32)>;
    //
    let Some (%{Self::1, %cut} (_, ref y, _, _)) = opt_t4_1;
```

## Private fields

And finally, what to do with private fields?

If variable has private field, it is an  always `%hidden Self::private` quasi-field.
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
    // p1 : %full HiddenPoint;
    // p1 : %{%permit Self::pub, %private} HiddenPoint;
    // p1 : %{%permit Self::{x, y}, %private} HiddenPoint;
    // p1 : %{%permit Self::{x, y}, %hidden<%full> Self::private} HiddenPoint;
```

Where :
 - `::pub` is a "all public fields" quasi-field
 - `::private` is a "all private fields" quasi-field
 - `%hidden<%a>` - it is some specific `%a` quasi field access, but we have no access to specify it
 - `%private` is a shortcut for `%hidden<%full> Self::private`

So, more fully we could write for struct witj private fields:
 - `%empty : %{%deny Self::pub, %hidden<%empty> Self::private}` access
 - `%full  : %{%permit  Self::pub, %hidden<%full>  Self::private}` access


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

## ReExtendeded Partial Types

We could add additional ReExtendeded Partial Types for **safe** Self-Referential Types. 

Theory of types do not forbid extension of Partial Type, but internal Rust representation of variables gives significant limitations on such action.

It is need the `%miss`(aka `%deny` but extendible) field access to initialized constructor consumption only. And additional "extender" `%%=`.

Partly self-referential types example:
```rust
struct SR <T>{
    val : T,
    lnk : & T, // reference to Self::val
}

// case (FP1)
let x = %{%miss Self::lnk, %permit Self::_} SR {val : 5i32 };
    //
    // x : %{%miss Self::lnk, %permit Self::val} SR<i32>
    //
x.lnk %%= & x.val;
    //
    // x : SR<i32>;
    // x : %full SR<i32>;
```
And even AlmostFully self-referential types:
And another shortcut `%unfill : %miss Self::_`

```rust
struct FSR <T>{
    val : T,
    lnk : & %{%deny Self::lnk, %permit Self::val} FSR<T>, 
    // reference to almost self!
}

// case (FP2)
let x = %{Self::val, %unfill} FSR {val : 5i32 };
    //
    // x : %{%miss Self::lnk, %permit Self::val} FSR<i32>;
    //
x.lnk %%= & %max  x;
    //
    // x : FSR<i32>;
    // x : %full FSR<i32>;
```

First difficulty - `%max` is no longer `id`,  `%max(on %miss) ~ %deny`. Both `filter-%permit on %miss` and `filter-%ignore on %miss` must cause a compiler error for 3 main consumers.

Second and most difficult, that `return` consumption (yes, 6th type of consumers) from function could preserve `%miss`, so also we need filter `%max_miss`, where `%max_miss(on %miss) ~ %miss`!

```rust
// case (FP3)
// FROM case (FP2)
fn create_var()-> %{%miss Self::lnk, %permit Self::_} FSR {
    let x = %{Self::val, %unfill} FSR {val : 5i32 };
        //
        // x : %{%miss Self::lnk, %permit Self::val} FSR<i32>
        //
    %max_miss return x; 
    // filter access before 'return' to not to confused with `move` consumer!
}

let y = create_var();
y.lnk %%= & %max  y;
    //
    // y : FSR<i32>;
    // y : %full FSR<i32>;
```

