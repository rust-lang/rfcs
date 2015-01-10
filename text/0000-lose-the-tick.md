- Start Date: 2015-01-10
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Summary

Remove the `'` sigil from lifetimes. Instead of a `'` sigil being required
everywhere a lifetime is *mentioned*, the `lifetime` keyword is used where a
lifetime variable is *introduced*. As this already entails changing existing
code everywhere explicit lifetimes are involved, a couple of related tweaks to
reference and lifetime syntax are also floated.

Old syntax:

    fn get_mut<'a>(foo: &'a mut Foo) -> &'a mut Bar { ... }

New syntax (pick one):

 *      fn get_mut<lifetime a>(foo: &a mut Foo) -> &a mut Bar { ... }

 *      fn get_mut<lifetime a>(foo: &mut a Foo) -> &mut a Bar { ... }

 *      fn get_mut<lifetime a>(foo: &mut{a} Foo) -> &mut{a} Bar { ... }


# Motivation

The `'` sigil on lifetimes is, for newcomers to the language, visual noise with
little discernible meaning. The unmatched quote is also extremely unnatural for
those accustomed to C-style syntax: our main target audience. (Some of the
author's more knowledgeable friends, acquainted with functional programming
languages and not only C, have expressed the sentiment that they find it weird,
and always reflexively want to insert the closing quote.) The rest of Rust's
syntax does a fantastic job of fitting new concepts into a familiar C-style
syntax in an intuitive wayl; it's only the `'` sigil on lifetimes which sticks
out like a sore thumb and breaks the metaphor.

The new syntax with the `lifetime` keyword is more explicit, meaning it is more
verbose and provides greater clarity. Considering that due to excellent lifetime
elision, explicit lifetime variables are infrequently required, and that
lifetimes are a concept not to be found in any other popular language, this
seems like an obviously worthwhile tradeoff.

(This syntax also meshes well with the author's personal preference for
[the future syntax of kind ascriptions](http://www.reddit.com/r/programming/comments/2ny8c1/rust_generics_and_collections/cmiqhyx),
should we add them.)


# Detailed design

As described in the summary, lifetimes would lose the `'` sigil, and would
instead be introduced with the `lifetime` keyword. For lifetime arguments to
user-defined types, this would be the extent of it:

    struct Foo<lifetime a> { ... }

    struct Bar<lifetime a> { foo: Foo<a> }

    struct Baz { foo: Foo<static> } // `'static`, as well

For the built-in reference types, we have a couple of options. One is to do just
the same, and simply drop the `'`:

    struct Foo<lifetime a> {
        cow: &a Cow,
        chicken: &a mut Chicken
    }

While we're doing this, however, we could also take the opportunity to fix up
the oddity that, in the case of `&mut`, the lifetime separates the `&` from the
`mut`. (The type itself is generally thought of as being named `&mut`;
everywhere else in the language, it written that way as a single unit.) Then we
would have:

    struct Foo<lifetime a> {
        cow: &a Cow,
        chicken: &mut a Chicken
    }

We can go one step further, however, and opt for even more suggestive syntax:

    struct Foo<lifetime a> {
        cow: &{a} Cow,
        chicken: &mut{a} Chicken
    }

This makes the type easier to parse visually, and makes it more obvious that the
`a` is a special modifier of the reference. (When lifetimes are elided, we have
`&T`; from `&{a} T`, it is clear that the `{a}` part is new; from `&a T`, it's
less immediately apparent which part is what.) This syntax is also suggestive
of the idea that the reference is valid within the `{ }` block `a`: while this
is not *perfectly* accurate (lifetimes don't, or won't, always precisely
correspond to scopes), as a near approximation, it can nonetheless be a very
helpful intuition to have.

Finally, labelled loops would also lose their ticks:

    x: loop {
        loop {
            break x;
        }
    }


# Drawbacks

> What happens during the alpha cycle?
>
> If you’re already a Rust user, the first thing you’ll notice during the alpha
> cycle is a dramatic drop in the pace of breaking changes.
