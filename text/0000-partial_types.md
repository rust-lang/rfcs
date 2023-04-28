- Feature Name: `partial_types`
- Start Date: 2023-04-28
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

Partial Types proposal is a generalization on "partial borrowing"-like proposals.

This proposal is universal flexible tool to work **safe** with partial parameters, partial arguments, partial references and partial borrowing.

Advantages: maximum type safety, maximum type control guarantee, no ambiguities, flexibility, usability and universality.


# Motivation
[motivation]: #motivation

Safe, Flexible controllable partial parameters for functions and partial consumption (including partial not borrowing) are highly needed.

Partial Types extension gives to type-checker a **mathematical guarantee** that using _simultaneously_ partial typed variable, it multiple partial references and partial borrowing is as  **safe** as using them _at a sequence_.

And since it is a guarantee by **type**, not by **values**, it has _zero cost_ in binary! Any type error is a compiler error, so no errors in the runtime.

We could apply _theoretically_ this extension to all Product Types (`PT = T1 and T2 and T3  and ...`) and it fully covers all variants of possible uses of Product Types.

So, most promised candidates are Structs and Tuples.

This extension is not only fully backward-compatible, but is fully forward-compatible! Forward-compatibility is an ability to use updated functions old way.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

For Product Types `PT = T1 and T2 and T3  and ...` (structs, tuples) we assume they are like structs with controllable field-access.

`<%permit>` - field is accessible (always implicit), `%deny` - field is forbidden to use, `%miss` - it is uninitialized field, field is forbidden to use.

Let we wish to have next structure:
```rust
struct SR <T>{
    val : T,
    lnk : & T, // reference to val
}
```
Now it is impossible to write immutable self-referential types. But with partial initialization it becomes easy:
```rust
let x = SR {val : 5i32 };
    // omitted field is uninitialized, it has %miss field-access
    // x : SR<i32>.{val, %miss lnk}

x.lnk let= & x.val;
    // (let=) "extender", late initialized field
    // x : SR<i32>;
    // x : SR<i32>.%full;
```
Easy-peasy! We create partial type and extend by late initializing fields!

```rust
let baz = (7i16, &51i32, "some_other_string").{1, 2, %unfill};
    // or set access-filer before tuple constructor 
    // baz : (i16, &i32, &str).{1, 2, %miss 0};

let bar = (%miss 7i16, &51i32, "some_other_string");
    // or set access-filer before tuple constructor 
    // baz : (i16, &i32, &str).{1, 2, %miss 0};

bar.0 let= 68;
	// (let=) "extender", late initialized field
	// bar : (i16, &i32, &str);
	// bar : (i16, &i32, &str).%full;
```
Same with tuples.

Ok, let's try a bit more impossible now - two mutual borrowings of same variable (but different fields):
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
	// ".{}" is a getter several fields, similar to "." get 1 field 
	// pfull : &mut Point.%{was_x, was_y, %unfill}
	
let ref_p1now = &mut p1.{x, y};
	// pfull : &mut Point.%{x, y, %unfill}
	
let Point {%deny x, %deny y, was_x: upd_x, was_y: upd_y, %deny state} = ref_p1was;
    // all %deny and %miss fields must be '_', error in other case
	// similar with binding deconstruction
```
It is simple and will be possible. 

Same easy to write functions, which consume partial parameters:
```rust
fn re_ref_t (&self : & Self.%{t, %any}) -> &f64 {
   &self.t
}

fn refmut_w (&mut self : &mut Self.%{w, %any}) -> &mut f64 {
   &mut self.w
}
```

We could do much harder things: we wish parallel using function implementation, which update either `x` or `y` and both read `state` field.
```rust
impl {
	pub fn mx_rstate(&mut self1 : &mut Self.%{x, %any}, &self2 : & Self.{state, %any})
	{ /* ... */ }
		
	pub fn my_rstate(&mut self1 : &mut Self.%{y, %any}, &self2 : & Self.{state, %any})  
	{ /* ... */ }

	// Unfortunately it is a mandatory to extensive mut-borrow 'state' field!
	pub fn mxystate(&mut self : &mut Self.%{x, y, state, %any})  
	{ 
		/* ... */
		self.{x, state}.mx_rstate();
		/* ... */
		self.{y, state}.my_rstate();
		/* ... */		
	}
}
```
We need for this ... several selfs! Wow!

Anyway, it is easy, useful and universal!


# Reference-level explanation

This proposal of Partial Types (A) for full control on parameters and arguments requires mixed-mutable (B) references and mixed-mutable dereference (of mixed-mutable references).

Full implementation of mixed-mutable variables (B) is welcomed, but minimal requirement is 2-links only of partly type variables as representative one variable of mixed-mutable type.

If mixed mutable variables (B) is hard to implement, simple Multi-Selfs (C) alternative without mixed mutable variables is possible, but it covers only part of problems.

It is totally Ok if on "Stage 1" implements (A + C), then on "Stage 2" implements (B).

- [Partial type access]
- [Fields names and field-access]
- [Detailed access]
- [Access-filter]
- [New Picker several fields]
- [Partially Initialized Variables]
	- [miss field-access]
	- [Auto converting miss by borrowing]
	- [Late initialized permit field from miss field]
- [Partial Parameters]
- [Partial Arguments]
- [Partial Parameters]
- [Private fields]
- [Enum Type]

## Partial type access

I propose to extend type system by adding type access after type name (for Product Types), like `(i16, &i32, &str).%full`.

If we omit to write type access, it means `.%full` access. Symbol (`%` percent) mean percent or part of the whole thing (variable in our case).

Access has similar names and meanings as lifetimes: `.%full`(full access, soft keyword), `.%_`(don't care how partial access is, soft keyword) and any other `.%a`(some "a" access). But in most cases we use detailed access `.%{}`.

## Fields names and field-access

Fields names inside detailed access:
- named field-names for structs(and units)
- unnamed numbered for tuples
- `*` "all fields" quasi-field
- `_` "rest of fields" quasi-field
- `self` pseudo-field for unsupported types

Filed-access can be in 4 values:
- `<nothing>` or `%permit` - field is accessible,
- `<omitting>` or %deny` - field is forbidden to use,
- `%miss` (E) - it is uninitialized field, field is forbidden to use,
- `%ignore` (F) - quasi-field access, which hides truthful field access.

`%permit` filed-access is a default behavior: permit to read, to write, to borrow, to move. 

_A word %permit is need **for explanations only**, just use default `<nothing>` access._

`%deny` filed-access forbids to have an access to read, to write, to borrow, to move (like outside access to private field). It is a compile error if someone try to access.

_A word %deny is need **for explanations mostly**, just use default `<omitted field>` access._ It can be used explicitly in tuple initialization and binding deconstructions.

`%unfill` is a shortcut for `%miss _`.

`%any` is a shortcut for `%ignore _`.

## Detailed access and mutability

Detailed field-access has next structure: `.%{%field-access1 field-name1, %field-access2 field-name2, ..}` 

Detailed access is a set of unordered and unsorted field-names and their filed-accesses.

First `%field-access1` field-access if it is omitted means `%permit`. First `%field-accessM` field-access for _first omitted_ filed-name means `%opposite`. 

The rest of omitted `%field-accessN` field-accesses means `%same` (same as previous `%field-accessN-1`)

| `%field-accessN-1` | `%same`   | `%opposite` |
|--------------------|-----------|-------------|
| `%permit`          | `%permit` | `%deny`     |
| `%deny`            | `%deny`   | `%permit`   |
| `%miss` (E)        | `%miss`   | `%permit`   |

Examples:
```rust
struct Point {x: f64, y: f64, was_x: f64, was_y: f64}

let pfull = Point {was_x: 4.0, was_y: 5.0};
	// pfull : Point.%{was_x, was_y, %miss x, y}
```

Type access `.%full` is a shortcut for `.%{*}`.

It is forbidden to have `.%{%deny *}` values and `.%{%deny *}` references (but who knows, maybe they are suitable uninhabited types, phantom types or proxy types).

## Access-filter and Mutable Filter

Rust consumer use same names for action and for type clarifications, so we follow this style.

Access-filter is an action, which is looks like detailed access and it is written left to variable at consuming (moving, borrowing, referencing, initializing, returning, pick fields).

| ↓filter / →var access | `%permit` | `%deny`   | `%miss` (E) | `%ignore` (D) |
|-----------------------|-----------|-----------|-------------|---------------|
| `%permit`             | `%permit` | !ERROR    | !ERROR      | !ERROR        |
| `%deny`               | `%deny`   | `%deny`   | `%deny`     | `%deny`       |
| `%miss` (E)           | !ERROR    | !ERROR    | `%miss`     | !ERROR        |
| `%ignore` (F)         | !ERROR    | !ERROR    | !ERROR      | `%ignore`     |

It is allowed to write explicitly specific access-filter with same rules as detailed access.

Default filter on consumption if filter is omitted for:
 - `var  ~  var.%max`
 - `& var  ~  & var.%max`
 - `&mut var  ~  &mut var.%max`
 
(A + D)
 - `return var  ~  return var.%exact`
 - `return & var  ~  return & var.%exact`
 - `return &mut var  ~ return &mut var.%exact`
 
 where filters are based on variable field-access:

| ↓var access  | `.%max`   | `.%exact` (D) |
|--------------|-----------|---------------|
| `%permit`    | `%permit` | `%permit`     |
| `%deny`      | `%deny`   | `%deny`       |
| `%miss` (E)  | `%miss`   | `%miss`       |
| `%ignore`    | `%deny`   | `%ignore`     |

## New Picker several fields

Rust has special syntax for "pick one field" (get field-access):
```rust
struct Point {x: f64, y: f64, was_x: f64, was_y: f64}

let mut pfull = Point {was_x: 4.0, was_y: 5.0};

let px = pfull.x;
let rpy = & pfull.y;
let bpwasx = &mut pfull.was_x;
```

But it is still impossible to "pick several fields". With partial types it become possible. 

I suggest to add additional picker for variables `var.{fld1, fld2, ..}` (similar to partiality `var.%{fld1, fld2}`):
```rust
let pxy = pfull.{x, y};
//   same as
let pxy = pfull.%{x, y};

let rpxy = & pfull.{x,y};
//   same as
let rpxy = & pfull.%{x,y};

let rpwas = & pfull.{was_x, was_y};
//   same as
let rpwas = & pfull.%{was_x, was_y};
```

## Partially Initialized Variables

Partially Initialized Variable has next structure:
- for structs: `Construct{%field-access1 field-name1: value1, %field-access1 field-name2: value2, ..}.%access-filter;` 
- for tuples:  `(%field-access1 value1, %field-access1 value2, ..).%access-filter;` 

Access filter `%access-filter` if it is omitted means `.%max`.

All `%field-accessN` field-access in tuples if they are omitted mean `%permit`. 

All `%field-accessN` field-accesses (for explicit filed-names in structs) if they are omitted mean `%permit`. All `%field-accessM` field-accesses for omitted filed-names in structs mean `%miss`.

Now it is impossible to initialize outside a new struct with private fields. With partial types it is possible, but variable type access cannot be `.%full` in that case.

Also, constructor could copy "rest of fields" not from single variable, but from several variables (if they don't overlap permitted fields) and even fill empty constructor:
```rust
struct Point {x: f64, y: f64, was_x: f64, was_y: f64}

let pwas = Point {was_x: 4.0, was_y: 5.0};
	// pwas : Point.%{was_x, was_y}

let pnow = Point {x: 1.0, y: 2.0};
	// pnow : Point.%{x, y}
	
let pfull1 = Point {..p1, ..p2};
	// pfull1 : Point
    // pfull1 : Point.%full

let pfull2 = Point {x: 42.0, was_x: -5.0, ..p1, ..p2};
	// pfull2 : Point
    // pfull2 : Point.%full
```

## miss field-access

(A + E)

Theory of types do not forbid extension of Partial Type, but internal Rust representation of variables gives significant limitations on such action.

`%miss` field-access allows to extend Partial Type.

`%miss` filed-access like `%deny` filed-access forbids to have an access to read, to write, to borrow, to move like access to private field. It is a compile error if someone try to access.

`%unfill` is a shortcut for `%miss _`.

### Auto converting miss by borrowing

(A + E)

Mutable and immutable borrowing (but not moving) automatically convert `%miss` field access into `%deny` for reference.
```rust
let pfull = Point {was_x: 4.0, was_y: 5.0};
	// pfull : Point.%{was_x, was_y, %unfill}

let ref_pful = & pfull;
	// ref_pful : Point.%{was_x, was_y}
```

### Late initialized permit field from miss field

(A + E)

If field  has `%miss` field-access we could change it to `%permit` together with initializing the field by `let=` operator.
```rust
struct SR <T>{
    val : T,
    lnk : & T, // reference to val
}

let x = SR {val : 5i32 };
    // x : SR<i32>.%{val, %miss lnk}

x.lnk let= & x.val;
    // (let=) "extender", late initialized field
	// change from .%{%miss lnk, ..} to .%{%permit lnk, ..}
    // x : SR<i32>;
    // x : SR<i32>.%full;
```

It is an compiler error if `%%=` operator tries to extend not `%miss` filed-accessed fields (`%permit` or `%deny` or `%ignore`).

We could also extend several fields in single expression:
```rust
let pfull = Point {x : 5.0, y : 6.0, was_x : 7.0, was_y : 13.0};
    // pfull : Point
	
let pxy = Point {x : 5.0, y : 6.0 };
    // pxy : Point.%{x, y}

pxy.{was_x, was_y} let= pfull.{was_x, was_y};
    // pxy : Point;
    // pxy : Point.%full;
```

I assumed, that `%miss` field-access could preserve at move action, but maybe it was my over-optimistic guess.

Is is possible after creating one variable with missed field, move (partly) it into another variable, and then independently extend same field at both variables?
```rust
let p = Point{x : 1.0, y: 2.0};
let rp = & p.{x};
    // rp : & Point.%{x}  not  rp : & Point.%{x, %miss y}

rp.y let= &8.0;
// or
*rp.y let= 8.0;
// error : cannot extend denied field by let=
```
By theory of types it is not forbidden, but in reality due Rust variable representations this "extender" rewrites `p.y`.

That's why we need to distinguish `%deny` from `%miss`.

## Partial Parameters

(A + F) 

Partial Parameters has additional field-access on type - `%ignore`. 

Inside function body all `%ignore` fields of parameter hide filter-access of incoming argument into `%ignore` and they remain ignorant till return.

No one could consume `%ignore` fields (except return-consumer) because we have no guarantee, that some field is permitted. It is a compiler error!
```rust
    pub fn t_refmut(&self : &mut Self.%{t, %any}) -> &mut f64 {
        &mut self.x
        //   ^~~~~~
        // error: 'x' is an ignored field
    }
```

`%any` is a shortcut to `%any = %ignore _`.

(A + D)

Return consumers (omitted or explicit) could consume `%ignore` field, that's why default omitted access filter for return consumers is not `%max`, but `%exact`. 

Fill the difference:
```rust
    pub fn t_refmut1(&self : &mut Self.%{t, %any}) -> &mut Self.%{t} {
        &mut self.%max
    }

    pub fn t_refmut2(&self : &mut Self.%a@%{t, %any}) -> &mut Self.%a {
        &mut self
        // same as
        &mut self.%exact
    }
```

(A + C)

Multi-Sefs of Partial types partly allows to write controllable access to parameters. Sure, new keywords: `self1`, `self2`, `self3`, `self4`. 
```rust
impl {
	// we could accurate write this function
	pub fn mx_rstate(&mut self1 : &mut Self.%{x, %any}, & self2 : & Self.%{state, %any})
	{ /* ... */ }
		
	// we could accurate write this function
	pub fn my_rstate(&mut self1 : &mut Self.%{y, %any}, & self2 : & Self.%{state, %any})  
	{ /* ... */ }

	// Unfortunately it is a mandatory to extensive mut-borrow 'state' field!
	pub fn mxystate(&mut self : &mut Self.%{x, y, state, %any})  
	{ 
		/* ... */
		self.{x, state}.mx_rstate();
		/* ... */
		self.{y, state}.my_rstate();
		/* ... */		
	}
}
```

## Partial Arguments

For function argument we add another default omitted access filter `%arg` - qualified safe filter with minimum permit-fields, but it refers not to variable access, but to parameter accesses, so we could use it in arguments consumption only! It is an compile error if `%arg` is written outside of contents!

| param access  | `.%arg`   |
|---------------|-----------|
| `%permit`     | `%permit` |
| `%deny`       | `%deny`   |
| `%ignore` (F) | `%deny`   | 

Implementations always consumes `self` by `.%arg` filter!

```rust
let mut p1 : Point = Point {x:1.0, y:2.0, z:3.0, t:4.0, w:5.0};

fn re_ref_t (& p : & Point.%{t, %any}) -> &f64 {
   &p.t
}

let reft = re_ref_t(& p1);
//    same as
let reft = re_ref_t(& p1.%arg);

let mut p2 : Point2 = Point {x:1.0, was_x:2.0};

fn px_store (&mut p1 : &mut Point2.%{was_x, %any}, &p2 : & Point2.%{x, %any})  {
   *p1.was_x = *p2.x;
}

px_store(&mut p2, & p2);
//    same as
px_store(&mut p2.%arg, & p2.%arg);
```
(F)

The difference in argument use for parameters with ignored fields and without:
```rust
fn updx_with_ignore (&mut p : &mut Point.%{x, %any}, newx : f64) {
   *p.x = newx;
}

fn updx_without (&mut p : &mut Point.%{x}, newx : f64) {
   *p.x = newx;
}

updx_with_ignore(&mut p2, 6.0); // Ok
//    same as
updx_with_ignore(&mut p2.%arg, 6.0); // Ok

updx_with_ignore(&mut p2.{x,y}, 6.0); // still Ok
updx_with_ignore(&mut p2.{x,y,z}, 6.0); // still Ok
updx_with_ignore(&mut p2.%max, 6.0); // still Ok
updx_with_ignore(&mut p2.%full, 6.0); // almost Ok


updx_without(&mut p2, 6.0); // Ok
//    same as
updx_without(&mut p2.%arg, 6.0); // Ok

updx_without(&mut p2.{x,y}, 6.0); // error
updx_without(&mut p2.{x,y,z}, 6.0); // error
updx_without(&mut p2.%max, 6.0); // error
updx_without(&mut p2.%full, 6.0); // error
```

## Partial Parameters and Mixed Parameters in Traits

Partial Parameters in Traits could have generalized type access variants:
```rust
trait Getable {
	type Target;

    fn get_val<%a>(& self: & Self.%a) -> Self::Target;
}
```

## Private fields

Access to private fields looks like access to partial types. Extra denying private fields led to extra compile errors, but it is secure to control type access of private fields without extra rules.

## Enum Type

What's about Enums? Enum is not a "Product" Type, but a "Sum" Type (`ST = T1 or T2 or T3 or ..`).

But this proposal grant some **type** access, not a **value** access!

So, this proposal ignore this type!


# Drawbacks
[drawbacks]: #drawbacks

- it is definitely not a minor change
- type system became much more complicated


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

(A) A lot of proposals that are alternatives to Partial Types in a whole:
 - Partial Types [#3420](https://github.com/rust-lang/rfcs/pull/3420)
 - Partial borrowing [issue#1215](https://github.com/rust-lang/rfcs/issues/1215)
 - View patterns [internals#16879](https://internals.rust-lang.org/t/view-types-based-on-pattern-matching/16879)
 - Permissions [#3380](https://github.com/rust-lang/rfcs/pull/3380)
 - Field projection [#3318](https://github.com/rust-lang/rfcs/pull/3318)
 - Fields in Traits [#1546](https://github.com/rust-lang/rfcs/pull/1546)
 - ImplFields [issue#3269](https://github.com/rust-lang/rfcs/issues/3269)

(C), not (B): Instead of implementing (or before implementing) mixed mutable types, multi-selfs is quite simple alternative

(D) if retuning-consumer is complicated in implementation or if returning exact accessed variable has insignificant importance, this part of proposal could not be implemented. But this could led to backward incompatibility if return to implementing. So in this case we must reserve possible changes to return consumer.

(E) if implementation of operator`%%=` is almost impossible, we could get rid of `%miss` field-access and field-access filter.

(F) if we ignore forward-compatibility and ignore of access flexibility on arguments, we could not implement `%ignore` field-access and field-access filter.

(any.details) Alternative for another names or corrections for Partial Types.
 - `%empty` or `%!` name


# Prior art
[prior-art]: #prior-art

Most languages don't have such strict rules for references and links as Rust, so this feature is almost unnecessary for them.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

I assumed, that `%miss` field-access could preserve at move action, but maybe it was my over-optimistic guess.
Is is possible after creating one variable with missed field, move (partly) it into another variable, and then independently extend same field at both variables?


# Future possibilities
[future-possibilities]: #future-possibilities

(B) mixed mutable types after (A + C) realization.

