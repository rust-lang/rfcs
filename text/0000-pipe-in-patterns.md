- Feature Name: pipe_in_patterns.
- Start Date: 2017-02-03
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

## Summary
this RFC proposes allowing the `|` operator to be used within patterns to
allow for pattern matching with less boilerplate.

## Detailed Design
The current pattern matching is very powerful, however there is a lot of
boilerplate code needed to fully take advantage of it. Consider a
situation where you are building a state machine that is iterating through
`chars_indices`, and when you see `' '`, `'\n'`, `'\r'` or `'\f'`, you
want to change the state. Currently your match statement would look something
like the following example. There is no great way of reducing that boilerplate,
if anything that boilerplate only grows worse as you have more cases, and bigger
tuples.  Conservatively this feature would be simple syntactic sugar that just
expands to the old syntax.

```rust
match iter.next() {
    Some(_, ' ') | Some(_, '\n') | Some(_, '\r') | Some(_, '\u{21A1}') => {
        // Change state
    }
    Some(index, ch) => {
        // Look at char
    }
    None => return Err(Eof),
}
```

The solution to this would be to allow for `|` to be used within patterns. This
will significantly reduce how much boilerplate is required to have conditional
matches with tuples.

```rust
match iter.next() {
    Some(_, ' ' | '\n' | '\r' | '\u{21A1}') => {
        // Change state
    }
    Some(index, ch) => {
        // Look at char
    }
    None => return Err(Eof),
}
```
In terms of Rust's grammar `pats_or` would probably be changed to the following.
Which should allow for `Some(1 | 2 | 3, ' ' | '\n' | '\r')`.

```yacc
pats_or
: pat              { $$ = mk_node("Pats", 1, $1); }
| pat '|' pat  { $$ = ext_node($1, 1, $3); }
;
```

## How We Teach This
I think all that would be required is additional examples in sections on
pattern matching. The syntax is very intuitive.

## Drawbacks
None that I can think of.

## Alternatives
Keep syntax as is.

## Unresolved Questions
None at the moment.
