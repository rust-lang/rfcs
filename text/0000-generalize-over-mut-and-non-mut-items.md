- Start Date: 2015-03-14
- RFC PR #: (leave this empty)
- Rust Issue #: (leave this empty)

# Summary

Codify the conventions of specifying both mutable and non-mutable versions of items.

# Motivation

We do this to reduce code duplication and to generalize over the divide between the mut and non-mut versions of items and thus, in some sence, tie those thus far completely separate (but logically related) items together. This would allow us to unify traits like `std::ops::Index` and `std::ops::IndexMut` into a single logical unit.

# Detailed design

Introduce a new keyword `opt` that acts as a kind of a placeholder that can stand for either `mut` or nothing (*"optionally mutable"*). Introduce two new special prefixes `opt-` and `mut-` for the names of items, like functions, traits and structs, which are affected by (or under the influence of) the new `opt` keyword. Normally the rules dictate that `-` is not a legal character in identifiers, but this would be an exception to that rule.

For example, code snippet #1 is the same as if you had written code snippet #2. Notice that `opt-Item` is the unqualified name of an item (it's an identifier even though it doesn't look like it) and `mut-Item` is the name of an item. Also notice that you are not allowed to put whitespace right before or after the `-` character.

Code snippet #1:
```
struct opt-Thing<'a> {
    x: &'a opt i32,
    y: &'a mut i32,
    z:         i32,
}

fn opt-foo(thing: &opt Thing) -> &opt i32 {
    &opt thing.x
}

// Notice that the qux method is not affected by the opt keyword. Also notice
// that by prefixing the tux method name with `mut-` we indicate that the
// method should be placed under the `mut-` version of the trait.
trait opt-Trait {
    fn opt-bar(&opt self) -> &opt Self;
    fn opt-baz(&mut self) -> &opt i32;
    fn qux(&self) -> i32;
    fn mut-tux(&self) -> i32 { 42 }
}
```

Code snippet #2:
```
struct Thing<'a> {
    x:     &'a i32,
    y: &'a mut i32,
    z:         i32,
}

struct mut-Thing<'a> {
    x: &'a mut i32,
    y: &'a mut i32,
    z:         i32,
}

fn foo(thing: &Thing) -> &i32 {
    &thing.x
}

fn mut-foo(thing: &mut Thing) -> &mut i32 {
    &mut thing.x
}

// Notice that the qux method, which was not affected by the opt keyword
// and not prefixed by `mut-`, is placed under this non-mut trait, and that
// the `mut-tux` method is not placed under here.
trait Trait {
    fn bar(&self) -> &Self;
    fn baz(&mut self) -> &i32;
    fn qux(&self) -> i32;
}

// Notice that the qux method, which was not affected by the opt keyword
// and not prefixed by `mut-`, is not placed under this mut qualified trait,
// and that the `mut-tux` method is placed under here.
trait mut-Trait {
    fn mut-bar(&mut self) -> &mut Self;
    fn mut-baz(&mut self) -> &mut i32;
    fn mut-tux(&self) -> i32 { 42 }
}
```

Given either code snippet #1 or #2 (both are equivalent), you could then implement separately either `Trait` or `mut-Trait` or both for a type just like you normally would (just remember that in the `mut-` version of the trait, the methods are named `mut-whatever` instead of `whatever`). But you could also implement the `Trait` and `mut-Trait` traits for a certain type both at once by using the syntax introduced in code snippet #3.

Code snippet #3:
```
struct S { x: i32 }

impl opt-Trait for S {
    fn opt-bar(&opt self) -> &opt S {
        self
    }

    fn opt-baz(&mut self) -> &opt i32 {
        &opt self.x
    }

    fn qux(&self) -> i32 {
        self.x
    }
}
```

What's further, not only could you implement two traits for a single type at once (as demonstrated in code snippet #3), but you could also implement two traits for two types at once. That is, you could implement for example `Trait` for `Thing` and `mut-Trait` for `mut-Thing` at once (so, it doesn't implement one trait per each type but rather does a 1-to-1 mapping). This is demonstrated in the following code snippet #4.

Code snippet #4:
```
// Just to recap, this is what the original opt-Thing looked like:
struct opt-Thing<'a> {
    x: &'a opt i32,
    y: &'a mut i32,
    z:         i32,
}

impl opt-Trait for opt-Thing {
    fn opt-bar(&opt self) -> &opt Self {
        self
    }

    fn opt-baz(&mut self) -> &opt i32 {
        self.x
    }

    fn qux(&self) -> i32 {
        self.z
    }
}
```

You can call those `opt-` and `mut-` prefixed methods just like any other functions. Just think of the prefix as part of the function name:

Code snippet #5:
```
trait opt-OtherTrait {
    fn opt-other_foo(&mut self);
}

impl<T> opt-OtherTrait for T
    where T: opt-Trait
{
    fn opt-other_foo(&mut self) {
        let t = self.opt-bar();
        let n = t.qux();
        // Couldn't call t.opt-baz() here because `t` may be non-mut
    }
}

fn main() {
    let mut thing: mut-Thing = mut-Thing::new();
    thing.mut-bar();
    thing.mut-baz();
    thing.qux();
}
```

One other use for the `opt-` prefix syntax is in `use` declarations. You could use it to import multiple items at once, so that instead of writing:
```
use foo::{bar, mut-bar};
```
...you could write it more concisely as:
```
use foo::opt-bar;
```

# Drawbacks

I bet people are going to think that a `-` as a valid character in an identifier is going to look weird. Don't worry, see the alternatives below. And remember that when syntax highlighting paints the whole identifier with one color, it shouldn't look so weird then.

# Alternatives

The weird `-` in `opt-` and `mut-` could be also any one of the following characters: `@`, `:`, `|`, `!`.

# Unresolved questions

?
