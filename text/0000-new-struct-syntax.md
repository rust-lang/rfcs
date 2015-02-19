- Feature Name: new_struct_syntax
- Start Date: 2015-02-15
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Get rid of the `:` in struct literals and make use of `:` consistent across the language.
The meaning of `:` is currently associated with specifying the type.
We need an operator that can be associated with mapping a keyword to a value.
`=>` could be such an operator.

# Motivation

The current use of `:` is inconsistent.
It prevents us from adding a number of otherwise trivial to implement features.
Currently it clashes with type ascription and named arguments RFCs.

Changing the struct literal syntax would make future features fit naturally and unambiguously within the language using the most obvious syntax.

## Problems with current syntax

Consider the following example, where the expression syntax is extended with `subexpression ":" type` for specifying expected type
(this is in line with type ascription RFC) and every pattern is extended with `subpattern ":" type`.

```rust
use std::num::Float;

struct Point<T: Float>{x: T, y: T}

fn distance(Point{x: x1: f32, y: y1}, Point{x: x2: f32, y: y2}) -> f32 {
	( (x2-x1).powi(2) + (y2-y1).powi(2) ).sqrt()
}

fn main() {
	println!("{}", distance(Point{x: 0: f32, y: 0}, Point{x: 10: f32, y: 10}));
}
```

Note how part of the function argument pattern reads `x: x1: f32`.
The syntax might not be completely ambiguous, but it definitely is confusing.

Another example: Assume we allow to omit the struct name in literal (essential for named arguments feature) together with type ascription:

```rust
foo({ a: b });
```

Is `{a: b}` a struct literal with value `b` mapped to `a` or is it a code block with expression `a: b` (`a` with ascribed type `b`)?

## What would be the ideal syntax?

There are few important characteristics that a struct literal syntax should have:

1.	It should not interfere with any existing operator that is currently (or might be in the future) used for different thing.

2.	It should conform to the initialization-follows-declaration rule so that the same syntax is used when declaring struct type and specifying a struct value.

3.	It should be easily distinguished from a code block so that single-value struct literal and single-statement code block are distinct.

4.	It should fit in a pattern naturally, and not clash with any operators used there (important for future named arguments feature).

# Detailed design

The proposal is to simply replace the `:` in structs with `=>`.
`=>` would then become the key-value mapping operator.
It is currently used in match blocks where it serves a similar purpose.

## Usage in expressions

Take the current definition of struct literal expressed roughly using PEG syntax:

```peg
expression ::= identifier "{" ( identifier ":" expression "," )* "}"
```

The new syntax would then become:

```peg
expression ::= identifier "{" ( identifier "=>" expression "," )* "}"
```

Which combined with type ascription (which extends `expression` with `expression ":" type`)
could allow us to instantiate a struct value with this syntax:

```rust
let p = Point { x => 1: f32, y => 2 }
```

## Usage in patterns

The current syntax of a struct in the context of a pattern could be defined as follows:

```peg
pattern ::= identifier "{" ( identifier ":" pattern "," )* "}"
```

The new syntax would then become:

```peg
pattern ::= identifier "{" ( identifier "=>" pattern "," )* "}"
```

Combined with type ascription (which extends `pattern` with `pattern ":" type`)
would allow patterns like:

```rust
let Point { x => a: f32, y => b } = get_origin();
// variables `a` and `b` now hold the coordinates
```

If `get_point` was generic, the `f32` type ascription on `x` field forces it to return `Point<f32>`.
Because pattern or subpattern can be specified as a variable name that gets the matched value assigned to,
destructuring syntax with type ascription becomes `{ field => variable: Type }`

## Declaration of struct type

The consensus is to leave the current syntax, even though it will break the initialization-follows-declaration rule.
It can be justified by the fact that `:` is usually followed by a type and it would hold true for struct definition:

```rust
struct Color {
	r: u8,
	g: u8,
	b: u8,
}

let yellow = Color { r=>255, g=>255, b=>0 };
```

## Possible future improvements

The fat arrow syntax can make code visually cluttered and puts too much focus on the arrow itself instead of what is being mapped.
Possible way to make things look better is to allow `{(a,b,c) => (1,2,3)}` or `{a,b,c =>> 1,2,3}` as sugar for `{a=>1, b=>2, c=>3}`.

---

Currently the input type of a function is required to always be of tuple type (the list of arguments).
With type ascription we can change the function syntax definition from `"fn" name "(" (arg ":" type ",")* ")" "->" type` to `"fn" name pattern "->" type`,
so the current function definitions become patterns matching tuples.
Then, by just allowing structs as possible input type for a function we introduce named arguments **almost seamlessly**.
(Such function would then be possibly defined as something like `fn foo{a => x: T, b => y: T} -> T`)

# Drawbacks

-	Another breaking syntax change.
-	Fat arrow doesn't look as nice as `:`, but at least it's unambiguous.
-	Fat arrow is already used in match block, which might be visually confusing.

# Alternatives

-	**Do nothing**: Not doing this will result in introducing new weird syntax stacked on top of weird syntax every time we introduce previously mentioned features.
	Fixing the syntax now will make the introduction of future features seamless with consistent syntax.

-	**Use C99 designated initializer syntax** (`{.x = 0}`): There's even more problems with that than with the original syntax:

	-	While the dot makes the struct distinct from a code block, the visual similarity is still to high.
	
	-	This would either break initialization-follows-declaration rule (which was proven to be a bad thing to do) or make struct declaration look just wrong (`struct Foo{.field=Type}`).
	
	-	The appeal of it stems from the fact that it resembles an assignment, except initializing a struct literal on it's own doesn't involve any assignments.
		Such syntax would be simply even more misleading.
	
	-	It looks pretty in context of an expression, but in patterns it's backwards, so then it becomes an reverse-assignment-thing which makes it not so pretty anymore.
	
	-	Syntax like this could be used for different purpose, e.g. a statement expansion thing:
	
		```rust
		foo{.x=0, .y=0} // foo.x=0; foo.y=0;
		foo{.bar(), .baz()}; // foo.bar(); foo.baz();
		foo{[0]='a', [1]='b'} // foo[0]='a'; foo[1]='b';
		```

-	**Use proposed syntax and change the declaration syntax aswell**:
	There's nothing wrong with the current declaratiom syntax,
	but leaving it unchanged while changing the other syntax would make structs
	the only data type which doesn't conform to the initialization-follows-declaration rule.
	To preserve the symmetry, the proposal would be to change `:` to `=>` in struct declaration aswell.

	```rust
	struct Color<T>(T, T, T); // declaration
	set_color(Color(0: u8, 0, 0)); // initialization
	let Color(red: u8, grn, blu) = get_color(); // pattern matching

	struct Color<T>{r=>T, g=>T, b=>T}; // declaration
	set_color(Color{r=>0: u8, g=>0, b=>0}); // initialization
	let Color{r=>red: u8, g=>grn, b=>blu) = get_color(); // pattern matching
	```

	Note how there was never a single `:` in named tuple declaration.
	Instead, it "counter-intuitively" puts types where the values should be.
	Following this logic - when declaring a struct, we write it exactly the same as we use it,
	except we put types where the values should be, just like in the case of tuple structs.

	This might seem counter-intuitive, but that is kinda the point.
	After this RFC `:` becomes the universal type ascribing operator in the context of expressions and patterns,
	but note that any part of type definition is neither an expression or a pattern.

	More argumentation:

	>	Remember that the declaration is meant to be "backwards" and `:` is now **actual** operator in patterns and expressions.
	>	Using `:` in struct declaration would be actually inconsistent, because it would then have another use over it's purpose of type ascription,
	>	which can only be valid in expressions and patterns (and struct def is just `<ident> => <type>`, no patterns/expressions here).
	>	(I know it's used for Trait bounds too, but it's a completely different context so I think that's okay)

# Unresolved questions

-	How to introduce the new syntax? do we allow using both for some time?
-	When should the old syntax be deprecated, if at all?
