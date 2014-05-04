- Start Date:
- RFC PR #:
- Rust Issue #:

# Summary

Change the syntax for struct literals to use `=` rather `:`.

# Motivation

Mostly in Rust we use `=` for assignment and `:` for relations between types or between values and types. For example, the type of a variable in a pattern, the (type) bounds of a type variable. We use `=` for assignment/initialisation everywhere except struct fields - in `let` expressions, in type aliases, for named parameters in format strings. The one exception to both rules is in struct literals where we use `:` for assignment (and pattern matching, note that we never use `:` for pattern matching elsewhere).

I believe using `=` in struct literals is more consistent. It is also more practical since it makes pattern syntax more distinguished from type ascription, which is allowed in similar places in code.

It should also open the door to allowing general type ascription (should we want that) because struct intitialisers are currently the only place `:` may appear in an expression.

In an even more bit of extreme possible future proofing and navel-gazing language design, if we ever add named arguments to functions, we are likely to use `=` and so we keep the correspondence between between argument lists and data structures - extending the correspondence from un-named args -> enums to named args -> structs.

# Detailed design

The syntax for a struct literal in either expression or pattern position would change from:

```
sl ::= name `{` (field `:` value)* [.., expr] `}`
```

to:

```
sl ::= name `{` (field `=` value)* [.., expr] `}`
```

Examples, given a struct `S`,

```
struct S {
    field1: int,
    field2: Vec<float>
}
```

initialisation (this is the important bit, look at how nice this looks!):

```
let x = S { field1 = 4, field2 = vec![3.14, -6.0] };

let x = S {
    field1 = 4,
    field2 = vec![3.14, -6.0]
};
```

pattern matching in argument positions and `let` and `match` statements:

```
fn foo(S {field1 = f1, field2 = f2}: S) {
    ...
}

let S {field1 = f1, field2 = f2} = x;

match x {
    S {field1 = f1, _} => println!("found an S, field1 i {}", f1),
    _ => {}
}

```

Although note that using such patterns should be relatively rare since the form which binds a variable to a field of the same name is more succint (and wouldn't change with this proposal).

# Drawbacks

It's a large change that will affect a lot of code. However, it is purely syntactic and should be easy to mechanise.

You could no longer copy and paste a struct def to a struct lit and replace the types with valus to get a valid literal. But you would only need to change one more character per field.

# Alternatives

Don't do this. We keep the familiar but inconsistent syntax.

# Unresolved questions

None, that I see.
