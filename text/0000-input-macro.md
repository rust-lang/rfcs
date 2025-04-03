- Feature Name: `input macros`
- Start Date: 2025-04-02)
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

This RFC propose the addition of macros and some functions that can be used to read input from the user in a more ergonomic way like the [Input built-in function in Python](https://peps.python.org/pep-3111/). 

With this initiative we can build a small interactive programs that reads input from standard input and writes output to standard output is well-established as a simple and fun way of learning and teaching Rust as a new programming language. 

```rust
println!("Please enter your name: ");
let possible_name: Result<String, _> = input!(); // This could fail for example if the user closes the input stream

// Besides we can show a message to the user
let possible_age: Result<u8, _> = input!("Please enter your age: "); // This could fail for example if the user enters a string instead of a number in the range of u8

// --- Other way to use the macro ---

struct Price {
    currency: String,
    amount: f64,
}

impl FromStr for Price {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 2 {
            return Err("String must have two parts".to_string());
        }
        let currency = parts[0].to_string();
        let amount = parts[1].parse().unwrap();
        Ok(Price { currency, amount })
    }
}

let price: Price = input!("Please introduce a price (format: '[currency] [amount]'): ")?; // This could fail for example if the input is reading from a pipe and we delete the file whose descriptor is being read meanwhile the program is running

```

In this examples I show many ways to use the `input!` macro.

In this macro we think that EOF is error case, so we return a `Result` with the error type being the error that caused the EOF. This is because is easily to handle the error for something new and we can mantain a similar behavior.

However we can use besides:

```rust
let name: Option<String> = try_input!("Please introduce a price: ")?;
```

For example, that in this we can handle the error in a different way.
If we get a EOF we can return `None` and handle it in a different way but it's not exactly a error, it's a different case, EOF is valid but doesn't have a value, a way to represent this, that is why we use a `Option`.

**DISCLAIMER**: The behavior of the `input!` to me is the most intuitive, but I think that the `try_input!` could be useful in some cases to be correct with the error handling. We can change the name of the macro `try_input!` or delete it if we think that is not necessary. It's just a idea, I'm open to suggestions.

# Motivation
[motivation]: #motivation

This kind of macros could be useful for beginners and reduce the barrier to entry for new Rustaceans. It would also make the language more friendly and help with the cognitive load of learning a new language.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

Explain the proposal as if it was already included in the language and you were teaching it to another Rust programmer. That generally means:

- Introducing new named concepts.
- Explaining the feature largely in terms of examples.
- Explaining how Rust programmers should *think* about the feature, and how it should impact the way they use Rust. It should explain the impact as concretely as possible.
- If applicable, provide sample error messages, deprecation warnings, or migration guidance.
- If applicable, describe the differences between teaching this to existing Rust programmers and new Rust programmers.
- Discuss how this impacts the ability to read, understand, and maintain Rust code. Code is read and modified far more often than written; will the proposed feature make code easier to maintain?

For implementation-oriented RFCs (e.g. for compiler internals), this section should focus on how compiler contributors should think about the change, and give examples of its concrete impact. For policy RFCs, this section should provide an example-driven introduction to the policy, and explain its impact in concrete terms.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This is the technical portion of the RFC. Explain the design in sufficient detail that:

- Its interaction with other features is clear.
- It is reasonably clear how the feature would be implemented.
- Corner cases are dissected by example.

The section should return to the examples given in the previous section, and explain more fully how the detailed proposal makes those examples work.

# Drawbacks
[drawbacks]: #drawbacks

Why should we *not* do this?

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Why is this design the best in the space of possible designs?
- What other designs have been considered and what is the rationale for not choosing them?
- What is the impact of not doing this?
- If this is a language proposal, could this be done in a library or macro instead? Does the proposed change make Rust code easier or harder to read, understand, and maintain?

# Prior art
[prior-art]: #prior-art

Discuss prior art, both the good and the bad, in relation to this proposal.
A few examples of what this can include are:

- For language, library, cargo, tools, and compiler proposals: Does this feature exist in other programming languages and what experience have their community had?
- For community proposals: Is this done by some other community and what were their experiences with it?
- For other teams: What lessons can we learn from what other communities have done here?
- Papers: Are there any published papers or great posts that discuss this? If you have some relevant papers to refer to, this can serve as a more detailed theoretical background.

This section is intended to encourage you as an author to think about the lessons from other languages, provide readers of your RFC with a fuller picture.
If there is no prior art, that is fine - your ideas are interesting to us whether they are brand new or if it is an adaptation from other languages.

Note that while precedent set by other languages is some motivation, it does not on its own motivate an RFC.
Please also take into consideration that rust sometimes intentionally diverges from common language features.

# Unresolved questions
[unresolved-questions]: #unresolved-questions

- What parts of the design do you expect to resolve through the RFC process before this gets merged?
- What parts of the design do you expect to resolve through the implementation of this feature before stabilization?
- What related issues do you consider out of scope for this RFC that could be addressed in the future independently of the solution that comes out of this RFC?

# Future possibilities
[future-possibilities]: #future-possibilities

Think about what the natural extension and evolution of your proposal would
be and how it would affect the language and project as a whole in a holistic
way. Try to use this section as a tool to more fully consider all possible
interactions with the project and language in your proposal.
Also consider how this all fits into the roadmap for the project
and of the relevant sub-team.

This is also a good place to "dump ideas", if they are out of scope for the
RFC you are writing but otherwise related.

If you have tried and cannot think of any future possibilities,
you may simply state that you cannot think of anything.

Note that having something written down in the future-possibilities section
is not a reason to accept the current or a future RFC; such notes should be
in the section on motivation or rationale in this or subsequent RFCs.
The section merely provides additional information.

Bullshit:
https://html.spec.whatwg.org/multipage/timers-and-user-prompts.html#dom-prompt-dev