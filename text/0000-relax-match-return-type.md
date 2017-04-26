- Feature Name: relax_return_type_constraints_for_match_and_if_expressions
- Start Date: 2016-02-03
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Branches of match and if expressions are required to return a value of exactly the same type.
This RFC suggests relaxing this constraint to only require all branches to return a value of a type which is usable by a surrounding function call.

# Motivation
[motivation]: #motivation

When we want to use the outcome of a match for a sufficiently generic function,
we currently have to duplicate the function call in each of the branches.

```rust
let file: Box<Read> = match fname {
	"-" => Box::new(stdin.lock()),
	_ => Box::new(fs::File::open(fname).unwrap())
};
```

This is shorter and more readable:

```rust
let file: Box<Read> = Box::new(match fname {
	"-" => stdin.lock(),
	_ => fs::File::open(fname).unwrap()
});
```

# Detailed design
[design]: #detailed-design

If branches in match or if expressions return a different type, the compiler should not give up yet.
Instead it should check if the respective expression is surrounded by a function call.
If all return types satisfy the trait bounds of that function call, the compiler should generate code to call the correct version of the function, in dependence of the taken branch.

Basically every time you can write something like this:

```rust
match … {
	… => myfun(…),
	… => { …; myfun(…) }
}
```

...you should instead be able to write this:

```rust
myfun(match … {
	… => …,
	… => { …; … }
})
```

...even if `myfun` is a generic function and not all branches of the match (or if) expression will return the exact same type.

If not all branches' return types fit the surrounding function call, this must of course still be an error.

# Drawbacks
[drawbacks]: #drawbacks

If a generic function does very different things depending on the input type, this feature leads to even more confusion.

# Alternatives
[alternatives]: #alternatives

If this RFC is not implemented, some code may remain slightly clumsier than necessary.
Honestly it's not a very big deal, but merely a small step towards making Rust even more user-friendly.

# Unresolved questions
[unresolved]: #unresolved-questions

We can also talk about making variables' type depend on a taken match branch like so:

```rust
let file: Read = match fname {
	"-" => stdin.lock(),
	_   => fs::File::open(fname).unwrap()
};
```

But in this case all the code following the match expression would have to be duplicated for all possible return types of the match.
If there are multiple such match expressions after one another, the number of code paths to be generated would *multiply*.
Such a code size blowup should IMHO not be hidden behind such a nice syntax.
