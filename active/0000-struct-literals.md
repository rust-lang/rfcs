- Start Date: 2015-02-13
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Change the syntax of struct literals. The first reason is that `:` is usually used for types.
The only other use of `:` is in struct literals and pattern matching on structs.
The other reason is that it might interfere with other future features like type ascription.

# Motivation

Because `:` is used for types, having general type ascription would be confusing.

Consider this:

```rust
fn foo(Foo { a: Bar, .. }: Foo<Bar>) { ... }
```

Currently this would bind a variable to `Bar` because it's a pattern on a struct.
Type ascription would then be something like:

```rust
fn foo(Foo { a: b: Bar, .. }: Foo<Bar>) { ... }
```

This doesn't look as good and raises questions about associativity of `:`

Another feature that the current syntax prevents is potential for anonymous record types.
By extension, this also prevents using that same syntax for keyword arguments.

```rust
x = {a: b};
```

Is this a block with type ascription or an anonymous record?


# Detailed design

Pick a syntax for structs. This syntax will be the One True Syntax for everything remotely struct related.
There are two requirements:

1. It is not ambiguous with anything currently used
2. It is nice to use in patterns

`let Point { x = a, y = b } = foo;` is probably unclear because it looks like `x` is being assigned to.
For this reason I would not suggest `=` as in previous closed RFCs.

If syntax like `Point { .x = 1, .y = 2 }` is picked, then it is quite natural that patterns would be something like:

```rust
Point { .x = a, .y = b} = foo;
```

it is clear that it is some `foo.x` that is being destructured to `a` because it `.x` is not a valid identifier.
The current syntax is not quite as clear:

```rust
let Point { x: a, y: b } = foo;
```

While there is only one logical way to write this, I still write it backwards sometimes.
This is because I am so used to writing

```rust
let a: i32 = bar;
```

so I always write `a: ` first in `let`s.

Another available syntax is `=>` which is used in keyword arguments in some languages and in hashtables in others.

# Drawbacks

Currently the use follows declaration

```rust
struct Point {
    x: i32,
    y: i32,
}

let Point{x: first, y: second} = Point{ x: 1, y: 2};
```

the same way that 

```rust
struct Color(f32, f32, f32);

let Color(red, green, blue) = Color(1.0, 1.0, 1.0);
```

where the types are replaced by their values

the counter-examples in Rust are functions

```rust
fn foo(x: i32) -> i32{
    x
}

let y = foo(5);
```

if functions uses followed declaration you'd expect `let x = foo(x: y);`

But it turns out there's symmetry, especially easily seen with the `=>` syntax:

```rust
struct Color(f32, f32, f32);

let Color(red: f32, green: f32, blue: f32) = Color(1.0, 1.0, 1.0);

fn foo(x: i32) -> i32{
    x
}

let y = foo(5: i32);

struct Point {
    x: i32,
    y: i32,
}

let Point{ x: i32 => first, y: i32 => second} = Point{ x: 1, y: 2};
```

Another drawback is that it will break almost every Rust program in existence while Rust is in alpha.
However, the fix is pretty mechanical. This also could be taken as an argument for this change.
If it is not changed now, it won't be ever changed, with consequences for later features.

# Alternatives

Keep the status quo.
Type ascription will have a weird `a: b: c` syntax or a different syntax like `be`.
Anonymous records types cannot be added backwards-compatibly.
Keyword arguments either added backwards-incompatibly or with a strange syntax.

# Unresolved questions

If greater flexibility in future versions of Rust is desired and this RFC gets accepted, what syntax is better?
The C99-style struct initializers `Point { .x = 1, .y = 2}` look like there's assignment being done, but there isn't.
That line is actually a value type where `.x = 1` is more like slotting the value into the struct.
At least the `.` makes it clear that this is inside the `Point` struct and not a valid identifier.

Some kind of syntax like PHP/Perl `=>` from hashtables does not look like assignment.
Because of destructuring patterns it is more clear to have some kind of syntax that shows the direction of slotting.