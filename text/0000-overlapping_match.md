- Feature Name: overlapping_match_statements
- Start Date: 2017-09-08
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This idea facilitates the writing and using of `match` expressions where multiple branches are
executed. Writing `match` expressions with this idea allows for multiple branches to be matched
and for a check on no matches as well, similar to the current use of the `_` pattern.

# Motivation
[motivation]: #motivation

There is a very good software engineering principle where repeating a piece of code is bad.
This is the case because if that selection of code needs to be changed then it has to be
changed in two places which can easily not be done and thus create bugs. A way of doing this
for a large selection of lines of code is to put it into a function, a helper function. Allowing
overlapping match statements extends this paradigm to that where matching is a good idea, the
use of pattern matching, and where exhaustiveness checks are a nice thing.

This would support use cases where the required execution of several branches overlapped enough
that his would help. A use case for this is when the outcome of one branch is the same as a
combination of the other two branches of a match statement. The expected outcome of this is
the ability to have multiple branches of a match statement, and having those branches still be
checked for exhaustiveness, be executed if more than one of them match the value.

# Detailed design
[design]: #detailed-design

Basic Syntax:
```rust
match many val {
    pat | pat => expr,
    pat => expr
}

match many val {
    pat | pat => expr,
    pat => expr
} else {
    expr
}
```

Benefits of this syntax:
1. Even though a new keyword has been made it will not break any code because Rust is a context
sensitive language. And adding such a keyword increases the perceptual area of the new syntax
so as to make it clear which type of match is being used.
2. The word `many` is used because it implies that after a branch is finished then the
control falls through to the check of the next branch.

Meaning of parts:
1. The `else` is used in a similar sort of vein to that of the `_` pattern in normal matches.
The expression enclosed within this is only executed if none of the patterns within the
`match/many` expression are matched. If `else` and `_` are both present then the code within the
`else` would be marked as unreadable.

Edge cases:
1. If the `_` pattern in present in any of the contained matches and the `else` block is also
present then a `unreachable_code` lint is emitted on the code within the `else` block
2. Since the main reason for using a `match` is the exhaustiveness checks as long as there isn't
an `else` block then the compiler will output an error for `non-exhaustive patterns` if not all
branches of the `match/many` are exhaustive.

Implementation Assumptions:
1. Assuming that a `match` expression is currently implemented similar to a long chain of
`if/else if` expressions. By this, meaning that each branch is checked one at a time and if it
matches then it skips checking any of the other branches and jumps to the end of the expression.

Implementation:
1. This can be implemented as if it was a list of `if` expressions. And a flag to check if any
of the branches have been visited so as to not visit the `else`
2. To cover the `else` case the location to jump to at the end after checking all the branches
can be stored, initially set to the start of the `else` block but if it enters any of the
branches then it is set to immediately after the `else` block.

# How We Teach This
[how-we-teach-this]: #how-we-teach-this

This should be called `match/many` expressions since that is the combination of keywords
that are used. This idea would be best presented as a continuation of existing Rust patterns
since it expands on the `match` expression.

This proposal should be introduced to new users right after `match` expressions are taught. This
is the best time to teach it since it appears as an extension of that syntax and the ideas that
are used when using `match` expressions.

Within the _Rust Book_ a section after the section on the `_` placeholder could be called
_match/in Control Flow Operator Addition_. Within this section the syntax and differences would
be outlined. These would most notable include the multiple branches can be executed. The reader
should be able to understand by the end of this section that this allows for multiple branches
to be executed but it still will check for exhaustiveness when able. He should also know that
the branches are checked top first.

An example that could be used within the section:

You can turn this:
```rust
match cmp.compare(&array[left], &array[right]) {
    Less => {
        merged.push(array[left]);
        left += 1;
    },
    Equal => {
        merged.push(array[left]);
        merged.push(array[right]);
        left += 1;
        right += 1;
    },
    Greater => {
        merged.push(array[right]);
        right += 1;
    }
}
```
into
```rust
match many cmp.compare(&array[left], &array[right]) {
    Less | Equal => {
        merged.push(array[left]);
        left += 1;
    },
    Greater | Equal => {
        merged.push(array[right]);
        right += 1;
    }
}
```

Another example is an implementation of fizzbuzz:

```rust
for x in 1...100 {
    let mut res = String::from("");
    if x % 5 == 0 {
        res += "fizz";
    }
    if x % 7 == 0 {
        res += "buzz";
    }
    if res.len() == 0 {
        res = x.to_string();
    }
    println!("{}", res);
}
```
into
```rust
for x in 1...100 {
    match many x {
        _ if x % 5 == 0 => print!("fizz"),
        _ if x % 7 == 0 => print!("buzz")
    } else {
        print!("{}", x);
    }
    println!("");
}
```

# Drawbacks
[drawbacks]: #drawbacks

This should not be done because it increases the size of language and might not be used by
everyone.

# Alternatives
[alternatives]: #alternatives

1. Instead of using `match` as a basis instead removing patterns from the equation and having
some notation that asks the compiler to prove that some value will be set to true by the time
a certain point in the code has been reached. This has some downfalls:
    1. It requires the compiler to prove something as true which the compiler currently does not
    do so that would require a lot more work.
    2. There does not seem to be any syntax that makes sense to use in this case without adding
    a new keyword and avoiding that is preferable
2. Not doing anything, since the old code works and is somewhat usable this idea is not necessary
to have and so not implementing it could be an option.

# Unresolved questions
[unresolved]: #unresolved-questions

Whether or not `match/many` makes sense for this sort of control flow.
