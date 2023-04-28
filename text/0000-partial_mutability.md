- Feature Name: `partial_mutability`
- Start Date: 2023-04-28
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)


# Summary
[summary]: #summary

Partial Mutability proposal is an addition to Partial Types proposal.

This proposal suggest to add new type of variables - Mixed/Partial mutable variables, including Partial mutable references.

Advantages: maximum flexibility, usability and universality.


# Motivation
[motivation]: #motivation

Partial Types without Partial mutable variables has no full control on parameters, here Partial mutable references are welcomed.

Partial Mutable Variables are new kind of Types, which are possible due Partial Types and they need twice memory on each mixed and pseudo-unmixed variable (before optimization).

We could apply _theoretically_ this extension to all Partial Types.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Mixed mutable variables are similar to partial types, we add partial access to mutability.
`mut = mut.%full`(in type declaration), `<const> = mut.{} = mut.{<%deny> *}`. We also add `mix = mut.%type` mutability inside type declaration.


Its is possible to have partial mutable variables and partial mutable borrowing and (immutable)references:
```rust
struct Point {x : f32, y : f32, was_x : f32, was_y : f32, state : f32};

let mut p2 : mut.%{x,y} Point = Point {x:1.0, y:2.0, was_x: 4.0, was_y: 5.0, state: 12.0};
	// infer mutability from mutability, lifted to type
	// p2 : mut.%{x,y} Point


let ref_p1 = &mut p1;
	// set mixed-mutable reference
	// ref_p1 : &mut.%{x, y} Point
	
let refnr_p2 = &mut.%{x} p2;
	// set mixed-mutable reference by specific detailed mutability
	// refnr_p2 : &mut.%{x} Point
	
let refnr2_p2 = & refnr_p2;
	// pseudo-unmixed reference
	// refnr2_p2 : && Point
```

We could write effective mixed-mutable references-arguments:
```rust
struct Point2 {x : f32, was_x : f32};

let mut p1 : Point2 = Point2 {x:1.0, was_x:2.0};

fn vx_store (&mut p : &mut.%{was_x} Point2) {
   *p.was_x = *p.x;
}

vx_store(&mut p1); // effective reference
```

We could get full control with Partial Types (A) together: we could write parallel using function implementation, which update either `x` or `y` and both read `state` field.
```rust
impl Point {
	pub fn mx_rstate(&mut self : &mix Self.%{mut x, state, %any})
	// infer mutability from mutability, lifted to type
	{ /* ... */ }
		
	pub fn my_rstate(&mut self : &mix Self.%{mut y, state, %any})
	// infer mutability from mutability, lifted to type
	{ /* ... */ }

	pub fn mxy_rstate(&mut self : &mix Self.%{mut x, mut y, state, %any})
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


# Reference-level explanation

I propose to make mutability partial, like `var : &mut.%full (i16, &i32, &str).%full`.

Field access mutability can be in 2 states only: `%permit` and `%deny`.

Partiality of `mut` keyword, wchich is depend from contents (as with Types, but a bit more)!


| context `mut`/`mix`       | desugar                         |
|---------------------------|---------------------------------|
| `let r = &mut var`        | `let r = &mut.%max var`         |
| `self.call(&mut var)`     | `self.call(&mut.%arg var)`      |
| `let mut var = ..`        | `let mut.%full var = ..`        |
| `let mut var : .. = ..`   | `let mut.%lift var : .. = ..`   |
| `let .. : &mut Type = ..` | `let .. : &mut.%full Type = ..` |
| `let .. : &mix Type = ..` | `let .. : &mut.%type Type = ..` |
| `let .. : mix  Type = ..` | `let .. : mut.%type  Type = ..` |

If we wish to **mix** partial mutuality and partial types, we need to use `mix` (or `mut.%type`) instead of `mut` in Type section!

Mixed detail partiality is mix of partial type with mutability
```rust
	pub fn mx_rstate(&mut self : &mix Self.%{mut x, state, %any}) { /* ... */ }
	// infer mutability from mutability, lifted to type

	pub fn mxy_rstate(&mut self : &mix Self.%{mut x, mut y, state, %any}) { /* ... */ }
```
Where we fix each field is mutable or not.


Mixed Parameters in Traits could have generalized sharing mutability variants:
```rust
trait Saveable {

    fn val_store<%b>(&mut self: &mix Self.%b);

    fn val_restore<%a, %b>(&mut self: &mut.%a Self.%b);
}
```

# Drawbacks
[drawbacks]: #drawbacks

- it is definitely not a minor change


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


# Prior art
[prior-art]: #prior-art

Most languages don't have such strict rules for references and links as Rust, so this feature is almost unnecessary for them.


# Unresolved questions
[unresolved-questions]: #unresolved-questions

None known.


# Future possibilities
[future-possibilities]: #future-possibilities

Mixed mutable variables could extend and add flexibility to Partial Types.

