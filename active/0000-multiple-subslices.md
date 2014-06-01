- Start Date: 2014-06-01
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Change syntax in slice pattern matching from `..xs` to `xs..` (eg. `[head, tail..]`),
and allow multiple (fixed-size) subslices borrows in one pattern (eg. `[xs..3, ys..3]`).

# Motivation

(1) Current subslice matching syntax is inconsistent with constant-sized arrays types `[T, ..N]` and expressions `[e, ..N]`.
In both cases, token after `..` is a number – array size, but in current pattern matching it is a sublice's
name. Furthermore, I think that `xs..` is more intuitive (and is also similar to syntax that C++ uses in variadic templates).

(2) Current syntax allows to make only one subslice reborrow in pattern. But now it's impossible to do following
with pattern matching:

    // slice has type &[T]
    match slice {
        [..firsts @ [_,_], ..rest] => { /* currently a syntax error */ }
        // ...
    }

(3) Moving name binding before `..` also enables possibility to use right-hand side of `..` as subslice's size,
enabling for example simple parsing of fixed width data format:

    let line: Vec<Ascii> = get_a_line();
    match line.as_slice() {
        [h, ..] if h == '#'.as_ascii() => (),
        [record..45] => match record {
            [name..20, state..10, phone..15] if valid(phone) => { /* ... */ },
            _ => fail!("Invalid phone number")
        },
        _ => fail!("Invalid format")
    }

# Detailed design

There are actually three proposals in this RFC:

1. Change syntax from `..xs` to `xs..`.
2. Allow multiple subslices borrows in one pattern.
3. Allow `xs..N` syntax (depends on 1. and 2.).

In this section I will describe as if all of proposals were implemented.

Now the subslice matching is implemented as special case in the parser. What I want to have is
described in following grammar (which assumes trailing commas for simplicity)
(grammar syntax as in Rust manual):
        
    slice_pattern : "[" [[pattern | subslice_pattern] ","]* "]" ;
    subslice_pattern : ["mut"? ident]? ".." integer? ["@" slice_pattern]? ;

In the following example, all subslices are fixed-sized:

    [xs..5, mut ys..42, ..5, zs.. @[2,5], .. @[1, two, _]] => //...
    [xs..@[ref first, ref second], ..10] => //...

Because subslice patterns are not special cases now, there has to be additional
check that at most one non-fixed-size subslice pattern is used.

Multiple mutable borrows of subslices should also be allowed (this is connected to
[#8636](https://github.com/mozilla/rust/issues/8636)), for example:

    let mut arr = [10, 20, 30, 40, 50];
    match arr {
        [mut xs..2, 30, mut ys..] => for x in xs.mut_iter().chain(ys.mut_iter()) { *x += 1; },
        _ => ()
    }

Rest of semantics and syntax should stay as it is.

# Drawbacks

The change is semantically backwards-compatible, so the only drawback
I can think of is that there are too little usecases to make this RFC worth implementing.

# Alternatives

* Do nothing,
* do only {1}, {2} or {1,2},
* provide the same functionality with another syntax,
* provide the same/similar functionality with a macro.

# Unresolved questions

If 3. is implemented, should it be possible to bind subslice's length to a variable?
For example: `[_, tail..tail_len] => { /* ... */ }`.
