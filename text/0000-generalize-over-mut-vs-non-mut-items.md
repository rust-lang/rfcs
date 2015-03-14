- Start Date: 2015-03-14
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Codify the conventions of specifying both mutable and non-mutable versions of items.

# Motivation

We do this to reduce code duplication and to generalize over the divide between the mut and non-mut versions of items and thus, in some sence, tie those thus far completely separate (but logically related) items together. This would allow us to unify traits like `std::ops::Index` and `std::ops::IndexMut` into a single logical unit.

# Detailed design

Introduce a new keyword `opt` that acts as a kind of a placeholder for either `mut` or nothing (*"optionally mutable"*). Introduce two new special prefixes `opt:` and `mut:` for the names of items, like functions, traits and structs, which are affected by (or under the influence of) the new `opt` keyword. Normally the rules dictate that ':' is not a legal character in identifiers, but this would be an exception to that rule.

For example, code snippet #1 is the same as if you had written code snippet #2. Notice that `opt:Item` is the unqualified name of an item (it's an identifier even though it doesn't look like it), `mut:Item` is the name of an item, and also notice that you are not allowed to put whitespace before or after the ':' character.

Code snippet #1:
```
struct opt:Thing<'a> {
    x: &'a opt i32,
    y: &'a mut i32,
    z:         i32,
}

fn opt:foo(item: &opt Thing) -> &opt i32 {
    &opt item.x
}

// Notice that the qux method is not affected by the opt keyword
trait opt:Trait {
    fn opt:bar(&opt self) -> &opt Self;
    fn opt:baz(&mut self) -> &opt i32;
    fn qux(&self) -> i32;
}
```

Code snippet #2:
```
struct Thing<'a> {
    x:     &'a i32,
    y: &'a mut i32,
    z:         i32,
}

struct mut:Thing<'a> {
    x: &'a mut i32,
    y: &'a mut i32,
    z:         i32,
}

fn foo(item: &Thing) -> &i32 {
    &item.x
}

fn mut:foo(item: &mut Thing) -> &mut i32 {
    &mut item.x
}

// Notice that the qux method, which was not affected by the opt keyword,
// is placed under this non-mut trait.
trait Trait {
    fn bar(&self) -> &Self;
    fn baz(&mut self) -> &i32;
    fn qux(&self) -> i32;
}

// Notice that the qux method, which was not affected by the opt keyword,
// is not placed under this mut qualified trait.
trait mut:Trait {
    fn mut:bar(&mut self) -> &mut Self;
    fn mut:baz(&mut self) -> &mut i32;
}
```

Given either code snippet #1 or #2 (both are equivalent), you could then implement separately either `Trait` or `mut:Trait` or both to a type just like you normally do (just remember that in the `mut` version of the trait, the methods are named `mut:whatever` instead of `whatever`). But you could also implement `Trait` and `mut:Trait` for a certain type both at once by using the syntax introduced in code snippet #3.

Code snippet #3:
```
struct S { x: i32 }

impl opt:Trait for S {
    fn opt:bar(&opt self) -> &opt S {
        self
    }

    fn opt:baz(&mut self) -> &opt i32 {
        &opt self.x
    }

    fn qux(&self) -> i32 {
        self.x
    }
}
```

What's further, not only could you implement two traits for a single type at once (as demonstrated in code snippet #3), but you could also implement two traits for two types at once. That is, you can implement for example `Trait` for `Thing` and `mut:Trait` for `mut:Thing` at once (so, it doesn't implement one trait per each type but rather does a 1-to-1 mapping). This is demonstrated in the following code snippet #4.

Code snippet #4:
```
// Just to recap, this is what the original opt:Thing looked like:
struct opt:Thing<'a> {
    x: &'a opt i32,
    y: &'a mut i32,
    z:         i32,
}

impl opt:Trait for opt:Thing {
    fn opt:bar(&opt self) -> &opt Self {
        self
    }

    fn opt:baz(&mut self) -> &opt i32 {
        self.x
    }

    fn qux(&self) -> i32 {
        self.z
    }
}
```

You can call those `opt:` and `mut:` prefixed methods just like any other functions. Just think of the prefix as part of the function name:

Code snippet #5:
```
trait opt:OtherTrait {
    fn opt:other_foo(&mut self);
}

impl<T> opt:OtherTrait for T
    where T: opt:Trait
{
    fn opt:other_foo(&mut self) {
        let t = self.opt:bar();
        let n = t.qux();
        // Couldn't call t.opt:baz() here because `t` may be non-mut
    }
}

fn main() {
    let mut thing: mut:Thing = mut:Thing::new();
    thing.mut:bar();
    thing.mut:baz();
    thing.qux();
}
```

# Drawbacks

I bet people are going to think that a ':' as a valid character in an identifier is going to look weird. Don't worry, see the alternatives. And remember that when syntax highlighting paints the whole identifier with one color, it shouldn't look so weird then.

# Alternatives

The weird ':' in `opt:` and `mut:` could be also any one of the following characters:
'@'
'-'
'|'
'!'

I'm starting to think that '-' might be the most natural choice:
`container.mut-iter()`

# Unresolved questions

?
