- Start Date: 2014-09-28
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)


# Proposal

Represent all `enum`s which have two variants, both of which have either no data
or zero-sized data, the same way as `bool` is currently represented. Guarantee
that this will be so. (This includes any user-defined `enum` with two nullary
variants, `Result<(), ()>`, and so on.)

Make `bool` an `enum` in the `prelude`:

    #[lang(bool)]
    enum bool {
        false,
        true
    }

(It still has to be a lang item due to being implicated in built-in constructs
such as `if`..`else`. }

Remove `true` and `false` as keywords.


# Arguments for

There's no reason to make something a wired-in part of the language definition
which can be expressed equally well within the language itself. Expressing
something within the language which is a built-in of other languages is also a
good educational opportunity.

Guaranteeing the representation encourages practitioners to define their own
binary `enum`s where this is appropriate, with the confidence that it will not
be inferior in any way to the built-in one. (This also allows `transmute`s to be
done with confidence between such equivalent formulations, and when we add a
`Transmute` trait, with guaranteed safety.)

Fewer keywords.


# Arguments against

Guaranteeing the representation forecloses on the possibility of the compiler
choosing a different one. However, there is no apparent reason why the compiler
would ever wish to do so. In particular, the only reason the compiler might wish
to do so is if another representation would be superior, but then we have the
situation that the representation of `bool` is, compared to this one, inferior,
which would certainly be bizarre: why would we use an inferior representation
for such an important type as `bool`?

There is a sense that `bool` should be a primitive type in the same lineage as
the built-in integer types. However, apart from it being existing practice in
other languages, there is no clear reason why this should be so. If the other
types could reasonably be taken out of the language, then they should also be,
but unfortunately they can not.
