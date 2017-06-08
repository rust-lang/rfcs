- Feature Name: N/A
- Start Date: 2015-07-06
- RFC PR: 
- Rust Issue: 

# Summary

How should we format function declarations?

# Motivation

Since the last time this was discussed, function declarations have got busier -
we've added `where` clauses and function types have got larger and are more
often used in generics. The style I see in the compiler is also different to
that used in some of the libraries and has moved away from the style guide
(which is underspecified in any case).

For reference the current text in the style guide states:

> For multiline function signatures, each new line should align with the
> first parameter. Multiple parameters per line are permitted:
> 
> ``` rust
> fn frobnicate(a: Bar, b: Bar,
>               c: Bar, d: Bar)
>               -> Bar {
>     ...
> }
> 
> fn foo<T: This,
>        U: That>(
>        a: Bar,
>        b: Bar)
>        -> Baz {
>     ...
> }
> ```

I believe that this does not work very well, since when scanning it is hard to
distinguish arguments from type parameters and the return type. It also does not
specify anything for where clauses.

# Detailed design

## Examples

```
// a comment
#[an_attribute]
#[another_attribute]
pub unsafe extern "C" fn foo<A, B, C>(x: SomeType,
                                      y: Vec<B>,
                                      z: SomeOtherType<B, C>)
    -> ReturnType<A>
    where A: Foo + Bar,
          B: Baz<B, C>,
{
```

```
// a comment
fn foo<A, B, C>(x: SomeType,
                y: Vec<B>,
                z: SomeOtherType<B, C>)
    -> ReturnType<A>
{
```

```
// a comment
fn foo(x: SomeType,
       y: Vec<B>,
       z: SomeOtherType<B, C>)
{
```

```
// a comment
pub unsafe extern "C" fn foo<A, B, C>(x: SomeType,
                                      y: Vec<B>,
                                      z: SomeOtherType<B, C>)
    where A: Foo + Bar,
          B: Baz<B, C>,
{
```

```
fn foo<T>(x: T, y: A) -> T where T: Bar {

// alternative
fn foo<T>(x: T, y: A) -> T
    where T: Bar
{
```

```
fn foo<T>(x: T, y: A) -> T {
```

## Rules

* Comments first, then attributes, one line each, then the rest of the signature on a new line;
* if the whole signature fits on one line (excluding comments and attributes), do it;
* if the signature requires more than one line,
  - each argument gets its own line, argument names are aligned;
  - the return type gets its own line, with block indent + 4;
  - the where clause gets its own line, aligned with the return type (or block indent + 4, if no return type);
  - each predicate in a where clause gets its own line, with the start of the predicates aligned;
  - the opening brace gets a new line at block indent.

Prefer not to split type parameters. If it is necessary, then the opening
parenthesis and opening angle bracket should be aligned. Type parameter names
should be aligned (but prefer one line if possible):

```
// Prefer this
fn foo<A, B, C>
      (x: SomeType,
       y: Vec<B>,
       z: SomeOtherType<B, C>)
{

// If we need more room (but prefer a where clause, if it is bounds causing the problem):
fn foo<A,
       B,
       C>
      (x: SomeType,
       y: Vec<B>,
       z: SomeOtherType<B, C>)
{
```

If more space is necessary for arguments, then the opening parenthesis may be on
a new line with block indent. Consider this only in an emergency. E.g.,

```
pub unsafe extern "C" fn foo<A, B, C>
    (x: SomeType,
     y: Vec<B>,
     z: SomeOtherType<B, C>)
    -> ReturnType<A>
    where A: Foo + Bar,
          B: Baz<B, C>,
{
```

When to use inline bounds vs where clauses:

* If there is a where clause, use it for all bounds (i.e., never have both inline bounds and a where clause);
* if there is not enough space for bounds without splitting lines between type parameters or splitting arguments, use a where clause;
* if a bound is a function type, use a where clause;
* if all bounds are 'small', prefer inline bounds;
* if in doubt, use a where clause.


## Justification

I hope most of these guidelines are common sense. I'll justify some of the more
controversial aspects.

My motivations (in order):

* readability, especially when scanning code;
* keep small functions small;
* consistency;
* avoiding change for most Rust code today.


### Indenting return type

Aligning the return type with the arguments makes it hard to scan for (one could
argue that the argument type is just another component of the function type, but
in practice I find myself often wanting to scan functions for the return type,
or to quickly notice if a function returns something or not). Indenting the
return type at block indent is hard for scanning if there is only one argument:

```
fn foo<A, B, C>(x: SomeType)
-> ReturnType<A>
{
```

Indenting at block indent + 4 works in all circumstances and is easy to scan for.


### Where clauses

The same arguments about indenting return types apply here. I believe
distinguishing `->` from `where` is easy enough that having where clauses and
return types at the same level of indent is OK.

Where clause predicates can be long; aligning to block indent + 4 rather than to
the arguments gives more space for long predicates.

Starting the line with `where` makes the clause easy to scan for (as opposed by
having it on the same line as the last argument).

Aligning the type variables is tidy and consistent with arguments. Having one
predicate per line, rather than sometimes putting the whole clause on one line
is consistent and does not affect the principle of keeping small functions small,
since a function with a multiple-predicate where clause is not a small function.
Furthermore, predicates can be syntatically complex and separating them by line
is useful for readability.


### The opening brace

Keeping the opening brace on the last line of the signature where there is a
return type or where clause means that there is no whitespace between the
function signature and body. E.g.,

```
fn foo<A, B, C>(x: SomeType)
    -> ReturnType<A> {
    let a = bar(x);
    let b = baz(a, x);
    b
}
```

That is horrible for scanning (and just really ugly).

For the sake of consistency, we should use the newline brace style for any
multi-line function, even if there is no where clause or return type. I think it
is OK to use same line brace for one line function signatures for the sake of
keeping small functions small, but it seems that arguing for consistency here is
also reasonable. E.g.,

```
fn foo<T>(x: T, y: A) -> T
{
```

Note that both same-line and newline brace styles are common in most programming
languages, although usually they are not mixed. Also note that the style we use
for functions does not necessarily have to affect the style used for block
expressions (e.g., the Mozilla C++ style guide requires newline braces for all
functions and same-line braces for all block statements).


## Questions

* Should there be a terminating comma for where clauses?
* Should the opening brace always be on a new line? What about one line signatures (the shortest examples)?
* Should where clauses always get their own line, even for short functions (see alternatives in examples)?


# Drawbacks

These guidelines could be quite disruptive, especially the brace on a new line.
This is alleviated somewhat by [Rustfmt](https://github.com/nrc/rustfmt). I
propose that if this RFC is accepted, a version of Rustfmt is made available
which only changes function signatures to the style described here. All code can
then be formatted automatically (modulo some exceptions for macros).


# Alternatives

So many. Some discussed inline.


# Unresolved questions

Function types - I want to leave this for later, they complicate things because
they look like function signatures themselves and tend to be very long, often
needing multiple lines.

What if we need to break the line inside a type or pattern?

Specific line length. This is somewhat orthogonal and dealt with elsewhere in
the style guide.

Spaces per tab. I've assumed four in this RFC, but it is really orthogonal, and
is dealt with elsewhere in the style guide.
