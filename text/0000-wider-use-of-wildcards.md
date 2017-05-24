- Start Date: 2014-12-10
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

This RFC proposes extended use cases of the wildcards (..) for closure
definition and tuple destructuing.

# Motivation

The wildcard (..) in the pattern matching is a very useful concept. They are
also used outside of a `match` expression, but there are some supplement cases
that are suitable for use with wildcards.

# Detailed design

1. Wildcards (..) and placeholders (\_) can be used in a pattern match.
However, only placeholders (\_) are allowed to use for ignoring some parameters
in a closure definition, like this:

 ```rust
Vec::from_fn(10, |_| rng.get_range(0i, 10));
```

 The use of a wildcard (..) in the closure definition would be useful when the
exact number of the arguments are forgotten, allowing the following code:

 ```rust
Vec::from_fn(10, |..| rng.get_range(0i, 10));
```

 This should also allow ignoring some part of the arguments, as follows:

 ```rust
|a, b, ..| a + b;
```

 Another benefit of using the wildcard parameters is that the code does not
break even if the callback definition is changed.

2. When destructuring a struct, a wildcard (..) is allowed to retrieve
only the desired fields.

 ```rust
let A { b: d, .. } = A { b: 1i, c: 2i };
```

 However, we can also think of a situation where only the part of a tuple are
needed, such as:

 ```rust
let (x, ..) = (1i, 2i, 3i);
```

 This makes the syntax more consistent with the one for structs.

# Drawbacks

Explicit breakage can be safer than implicitly accepting the changes of the
callback definition.

# Alternatives

We can remain the status quo.

# Unresolved questions

Not available.
