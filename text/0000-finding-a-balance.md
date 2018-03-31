- Feature Name: balanced-structure
- Start Date: 2018-04-01
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

This RFC proposes a new error style that strikes a better balance between structured output and human readability.

# Motivation
[motivation]: #motivation

The current `error-style` option of `rustc` provides three styles of error: `human`, `json` or `short`. When using `human`, the compiler will print errors like this:

```sh
error[E0277]: `Foo` doesn't implement `std::fmt::Debug`
 --> error-message.rs:4:22
  |
4 |     println!("{:?}", Foo);
  |                      ^^^ `Foo` cannot be formatted using `:?`; add `#[derive(Debug)]` or manually implement `std::fmt::Debug`
  |
  = help: the trait `std::fmt::Debug` is not implemented for `Foo`
  = note: required by `std::fmt::Debug::fmt`

error: aborting due to previous error
```

While this form is visually pleasing, it also suffers from some problems: many errors have custom display variants, needing the user to accomodate with a different error structure depending on what the error was.

Users interested in a more structured approach might reach for `json` here, only to figure out that the format is consistent, but far too verbose.

Reading them out aloud is also a pain and leads to inconsistencies. `short` would be useful for that case, bit it provides almost no additional context other then the error message.

We propose an additional error style that represents an in-between of the existing approaches. It strikes a balance between structure, human-readability and CLI brevity. Additionally, it allows downstream consumption through other tools. It follows Rusts tradition of using ideas from the past to implement the tooling of the future.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Beyond the classic error styles (`human`, `short` and `json`), `rustc` also provides you with a unique error style: `verbal`. It strikes a balance between being pleasing to read, familiar and also easier to read. These errors do away with the short notation generally used by many compilers.

For example, the error message for missing `Debug` implementation is:

```
You asked me to `Debug` a `Foo`
alas, I don't know what to do,
for `Foo` has no impl,
it's really that simple,
I'll leave the debugging to you.
```

The messages are especially useful in office environments, as they can be fluently read out to colleagues.

The error style is currently experimental, but we intend to write an extensive [collection of error messages and how to read them](https://books.google.de/books?hl=de&lr=&id=nFdOG5JxWZoC&oi=fnd&pg=PR9&dq=limericks+&ots=m1kV6ZKSFa&sig=2wAwaTDapYnj01S1IqujQL_z85M#v=onepage&q=limericks&f=false).


# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

User research has shown that while our error messages are great to read, they are hard to share. Especially in office environments, errors are often read
out loudly, leading to communication problems. We'd like to fix that.

Consider the following program:

```rust
fn main() {
    let mut vec = vec![1,2,3];

    let foo = &vec[2];

    // copious amount of work

    vec[1] = 2;
}
```

Which emits:

```
error[E0502]: cannot borrow `vec` as mutable because it is also borrowed as immutable
 --> src/main.rs:8:5
  |
4 |     let foo = &vec[2];
  |                --- immutable borrow occurs here
...
8 |     vec[1] = 2;
  |     ^^^ mutable borrow occurs here
9 | }
  | - immutable borrow ends here
```

While very clear, issues arise: verbalising this to a colleague requires the user to reassemble 4 messages and 2 snippets into fluid sentences. In these contexts, we can do better.

Sticking to Rust tradition of combining old and new approaches, we came up with an error format that is easy to write, easy to read and easy to communicate. Indeed, in this case, the applied base technique is roughly 500 years old.

2 lines are used for context, then 2 lines for explaining the core problem and one line to circle connecting to the first two lines, so that human readers have a sense of closure. Additionally, the last line acts as a "call to action", a best practice on the web.

It thus combines the ancient form of a limerick rhyme with techniques from modern web design.

The above error would read:

```
That `vec` which you wanted to borrow
Is giving my checker much sorrow
for meanwhile you mutate
it in line eight
let's fix it until tomorrow!
```

User tests on the #rustallhands have yielded positive results.

Additionally, we finally establish compatibility with `say(1)`.

## Examples of further errors

No error number:

```
An open parenthesis at
line ten in `http://foo.rs` had
no partner to close
it, I would say those
lines are syntactically bad.
```

E0596:

```
The value `z` which you mutate
at line four hundred five does negate
Your code otherwise good
is missing a `mut`
So add it before it's too late.
```

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

How is that even a question?

# Rationale and alternatives
[alternatives]: #alternatives

- Why is this design the best in the space of possible designs?

While other rhyme forms exists, Limericks are easy and simple.

- What other designs have been considered and what is the rationale for not choosing them?

Haiku have been attempted. While some current messages already fit into this form, others are very hard to fit into the limited syllables. In addition, the traditional form requires references to a season, which does not really fit well for an error message.

Shakespeare-style ABAB rhymes may possibly face legal troubles from the programming language of the same name.

- What is the impact of not doing this?

Sad faces all around.

# Prior art
[prior-art]: #prior-art

[Shakespeare](http://shakespearelang.sourceforge.net/)

[llogiq](https://twitter.com/llogiq)

[@lexlohr, a clone of llogiq](https://twitter.com/lexlohr)

# Unresolved questions
[unresolved]: #unresolved-questions

-- What parts of the design do you expect to resolve through the RFC process before this gets merged?

I hope that someone can find a better explanation what the aabba-structure of the limericks represents.

This RFC currently relies on a sole limerick author, which hopefully advances in cloning technology - kicked off by this RFC - will fix in the near future.

Alternatively, funding from Mozilla or the community for a full-time limerick working groups is an alternative.

We also hope finding new interaction patterns with other software, for example, a helpful Alexa skill could be developed and addressed as simply as:

```
echo "Alexa, `cargo build --message-format=verbal`" | say
```

-- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?

Many error messages will need to be ported. We believe a concerted community action will help fill the gaps quickly. Currently, only E502, E0277, E0596 and a parse error are ported.

This can be a community effort.

-- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

Puns.

Integration into Ember is desirable, maybe under the name of [Glimmerick](https://github.com/glimmerjs/glimmer-vm)?

## Thanks

To @badboy and @llogiq by helping out with this RFC!
