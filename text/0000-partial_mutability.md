- Feature Name: `mixed_mutable_variables`
- Start Date: 2023-04-28
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

Mixed Mutable Variables proposal is an addition to Partial Types proposal.

This proposal suggest to add new type of variables - Mixed mutable variables, including Mixed mutable references.

Advantages: maximum flexibility, usability and universality.


# Motivation
[motivation]: #motivation

Safe, Flexible controllable partial parameters for functions and partial consumption (including partial not borrowing) are highly needed.

Partial Types without Mixed mutable variables has no full control on parameters, where Mixed mutable references are welcomed.

Mixed Mutable Variables are new kind of Types, which are possible due Partial Types and they need twice memory on each mixed and pseudo-unmixed variable (before optimization).

We could apply _theoretically_ this extension to all Partial Types.

So, most promised candidates are Structs and Tuples.

This extension is not only fully backward-compatible, but is fully forward-compatible! Forward-compatibility is an ability to use updated functions old way.


# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Mixed mutable variables are similar to partial types, we add partial access to mutability.
`mut = mut.%full`, `<const> = mut.{%deny *}`. We also add `mix`, which have different mutabiliry depends on content.


Its is possible to have mixed mutable variables and mixed mutable borrowing and (immutable)references:
```rust

let mix p2 : Point.%{mut x, y} = Point {x:1.0, y:2.0, was_x: 4.0, was_y: 5.0, state: 12.0};
	// same as
let mut.%type p2 : Point.%{mut x, y} = Point {x:1.0, y:2.0, was_x: 4.0, was_y: 5.0, state: 12.0};
	// infer mutability from mutability, lifted to type
	// p2 : mut.%{x,y} Point



let ref_p1 = &mix p1;
	// same as 
let ref_p1 = &mut.%max p1;
	// set mixed-mutable reference
	// ref_p1 : &mut%{x, y} Point
	
let refnr_p2 = &mut.%{x} p2;
	// set mixed-mutable reference by specific detailed mutability
	// refnr_p2 : &mut%{x} Point
	
let refnr2_p2 = & refnr_p2;
	// pseudo-unmixed reference
	// refnr2_p2 : && Point
```

We could write effective mixed-mutable references-arguments:
```rust
let mut p1 : Point2 = Point2 {x:1.0, was_x:2.0};

fn vx_store (&mix p : &mut.%{was_x} Point2)  
	// same as 
fn vx_store (&mut.%type p : &mut.%{was_x} Point2)  
{
   *p.was_x = *p.x;
}

vx_store(&mix p1); // effective reference
	// same as 
vx_store(&mut.%arg p1); // effective reference
```

We could get full control with Partial Types (A) together: we could write parallel using function implementation, which update either `x` or `y` and both read `state` field.
```rust
impl {
	pub fn mx_rstate(&mix self : &mix Self.%{mut x, state, %any})
	// infer mutability from mutability, lifted to type
	{ /* ... */ }
		
	pub fn my_rstate(&mix self : &mix Self.%{mut y, state, %any})
	// infer mutability from mutability, lifted to type
	{ /* ... */ }

	pub fn mxy_rstate(&mix self : &mix Self.%{mut x, mut y, state, %any})
	// infer mutability from mutability, lifted to type
	{ 
		/* ... */
		self.{x, state}.mx_rstate();
		/* ... */
		self.{y, state}.my_rstate();
		/* ... */		
	}
}

p1.mxy_rstate();
```
We need for this ... "mixed-mutable" references! Wow!

Anyway, it is easy, useful and universal!

`mut = mut.%full`

# Reference-level explanation

- [Mixed Mutability]
- [Fields names and field-mutability]
- [Detailed mutability]
- [Mutable Filter]
	- [Mutable binding on Let-assignment]
- [Mixed mutable Initialized Variables]
- [Mixed Parameters]
- [Mixed referential arguments]
- [Mixed Parameters in Traits]
- [Private fields]
- [Enum Type]

## Mixed Mutability

I propose to make mutability partial, like `var : &mut.%full (i16, &i32, &str).%full`.

## Fields names, field-access and field-mutability

Fields names inside detailed access:
- named field-names for structs(and units)
- unnamed numbered for tuples
- `*` "all fields" quasi-field
- `_` "rest of fields" quasi-field
- `self` pseudo-field for unsupported types

Filed-mutability can be in 2 values: 
- `^mut` - field is mutable,
- `^const` - field is immutable.

`^mut` filed-mutability: permit to read, to write, to mutable-borrow, to immutable-borrow, to move.

`^const` filed-mutability: permit to read, to immutable-borrow, to move.

## Detailed access and mutability

Detailed field-mutability has next structure: `^{^field-mutability1 field-name1, ^field-mutability2 field-name2, ..}` 

Detailed mutability is a set of unordered and unsorted field-names and their filed-mutability.

First `^field-mutability1` field-mutability if it is omitted means `^const`. First `^field-mutabilityM` field-mutability for _first omitted_ filed-name means `^opposite`. 

The rest of omitted `^field-mutabilityN` field-mutability means `^same` (same as previous `^field-mutabilityN-1`)

| `^field-mutabilityN-1` | `^same`   | `^opposite` |
|------------------------|-----------|-------------|
| `^mut`                 | `^mut`    | `^const`    |
| `^const`               | `^const`  | `^mut`      |

Examples:
```rust
	// rpfull : &^{was_x, was_y} Point
	//   same as
	// rpfull : &^{^mut x, y} Point
	//   same as
	// rpfull : &^{was_x, was_y, ^mut x, y} Point
	//   same as
	// rpfull : &^{^mut x, ^mut y, ^const was_x, ^const was_y} Point
```

Sharing mutability `&mut` is a synonym for `& ^allmut` which is a shortcut for `& ^{^mut *}`. Sharing mutability `&` is a synonym for `& ^allconst` which is a shortcut for `& ^{^const *}`. 

## Access-filter and Mutable Filter

Rust consumer use same names for action and for type clarifications, so we follow this style.

Mutable filter is an action, which is looks like detailed mutability and it is written left to access-filter at consuming (moving, borrowing, referencing, initializing, returning, pick fields).

| ↓filter / →var mut+access | `^mut %_` | `^const %permit` | `^const %deny/%miss` | `^const %ignore` |
|---------------------------|-----------|------------------|----------------------|------------------|
| `^mut`                    | `^mut`    | !ERROR           | `^mut`               | !ERROR           |
| `^const`                  | `^const`  | `^const`         | `^const`             | `^const`         |

It is allowed to write explicitly specific mutability-filter with same rules as detailed mutability.
Default filter on consumption if filter is omitted for:
 - `var  ~  ^max var`
 - `& var  ~  & ^allconst var`
 - `&mut var  ~  & ^allmut var`
 
 where filters are based on variable field-mutability:

| ↓var mutability | `^max`    |
|-----------------|-----------|
| `^mut`          | `^mut`    |
| `^const`        | `^const`  |

### Mutable binding on Let-assignment

Mutable binding is a special consumer on Let-assignment. If it is a value (not a reference) mutable binding could set any asked mutability!

Default Let-filter for:
- `let var = &...  ~  let ^max var = &...`
- `let var = ...   ~  let ^allconst var = ...`
- `let mut var = ...  ~  let ^allmut var = ...`

So, if we wish to have same mutability, we must write explicitly:
```rust
struct Point2 {x: f64, y: f64}

let ^max pfull = Point2 {^mut x: 4.0, y: 5.0};
	// ^{^mut x, ^const y} pfull : Point2
```

Or write specific mutability directly at `let`:
```rust
let ^{^mut x} pfull = Point2 {x: 4.0, y: 5.0};
	// ^{^mut x, ^const y} pfull : Point2
```

Or we could lift detailed mutability into the type and use `^type` mutability
```rust
let ^type pfull : ^{^mut x} Point2 = Point2 {x: 4.0, y: 5.0};
	// ^{^mut x, ^const y} pfull : Point2
```

## Mixed mutable Initialized Variables

Mixed Mutable Initialized Variable has next structure:
- for structs: `^mutability-filter Construct{^field-mutability1 field-let-mutability1: value1, ^field-let-mutability2 field-name2: value1, ..};` 
- for tuples:  `^mutability-filter (^field-let-mutability1 field-name1: value1, ^field-let-mutability2 field-name2: value1, ..);` 

Access filter `^mutability-filter` if it is omitted means `^max`.

All `^field-let-mutabilityN` field-access if they are omitted mean `^max`. We assume, that Literals and out-variable values has `^const` mutability.

## Mixed Parameters

Mixed Parameters has mutable binding-mutable action, like on let-expressions
```rust
impl {
	pub fn mx_rstate(&^type self : &^{^mut x, ^const state, _} %{x, state, %any} Self)  
	{ /* ... */ }
		
	pub fn my_rstate(&^type self : &^{^mut y, ^const state, _} %{y, state, %any} Self)  
	{ /* ... */ }

	pub fn mxy_rstate(&^type self : &^{^mut x, y, ^const state, _} %{x, y, state, %any} Self)  
	{ 
		/* ... */
		self.{x, state}.mx_rstate();
		/* ... */
		self.{y, state}.my_rstate();
		/* ... */		
	}
}
```

## Mixed referential arguments

For function argument if it a value is meaningless to set special mutability, so omitted argument mutability is `^max`.

But for reference it is possible to write mixed borrowing `&^arg`, but it refers not to variable mutability, but to parameter referential mutability, so we could use it in arguments consumption only! It is an compile error if `&^arg` is written outside of contents!

| param mutability | `&^arg`  |
|------------------|----------|
| `^mut`           | `^mut`   |
| `^const`         | `^const` |

Implementations always consumes `self` by `&^arg` filter!

```rust
let mut p1 : Point2 = Point2 {x:1.0, was_x:2.0};

fn pntm_store (&^type p : & ^{^const x, ^mut was_x} Point2)  {
   *p.was_x = *p.x;
}

pntm_store(&^arg p1); // effective reference

pntm_store(&mut p1); // still ok

pntm_store(& p1); // error
```

## Mixed Parameters in Traits

Mixed Parameters in Traits could have generalized sharing mutability variants:
```rust
trait Saveable {

    fn val_store<^a, %b>(&^type self: & ^a %b Self);

    fn val_restore<^a, %b>(&^type self: & ^a %b Self);
}
```

## Private fields

Rust allows to control outer mutability of private fields via bindings or mutable and immutable borrowing. So, it is secure to control mutability of private fields without extra rules.

## Enum Type

What's about Enums? Enum is not a "Product" Type, but a "Sum" Type (`ST = T1 or T2 or T3 or ..`).

So, this proposal ignore this type!


# Drawbacks
[drawbacks]: #drawbacks

- it is definitely not a minor change
- It is highly recommended to deprecate operator `^` as a bitwise xor-function (it is still no ambiguities to write "`\s+^\s+`"), and replace it with another operator (for example: `^xor` / `xor`) to not to be confused by sharing mutability. But it is not a mandatory.


# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

(A) A lot of proposals that are alternatives to Partial Types in a whole:
 - Partial Types v2 [#3426](https://github.com/rust-lang/rfcs/pull/3426)
 - Partial Types [#3420](https://github.com/rust-lang/rfcs/pull/3420)
 - Partial borrowing [issue#1215](https://github.com/rust-lang/rfcs/issues/1215)
 - View patterns [internals#16879](https://internals.rust-lang.org/t/view-types-based-on-pattern-matching/16879)
 - Permissions [#3380](https://github.com/rust-lang/rfcs/pull/3380)
 - Field projection [#3318](https://github.com/rust-lang/rfcs/pull/3318)
 - Fields in Traits [#1546](https://github.com/rust-lang/rfcs/pull/1546)
 - ImplFields [issue#3269](https://github.com/rust-lang/rfcs/issues/3269)

(any.details) Alternative for another names or corrections for Mixed mutable variables.
 - `^allconst` or `^constall` name (and `^allmut` or `^mutall`)


# Prior art
[prior-art]: #prior-art

Most languages don't have such strict rules for references and links as Rust, so this feature is almost unnecessary for them.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

None known.


# Future possibilities
[future-possibilities]: #future-possibilities

Mixed mutable variables could extend and add flexibility to Partial Types.

