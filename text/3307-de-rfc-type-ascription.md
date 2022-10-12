- Feature Name: `ascription`
- Start Date: 2022-08-07
- RFC PR: [rust-lang/rfcs#3307](https://github.com/rust-lang/rfcs/pull/3307)
- Rust Issue: [rust-lang/rust#101728](https://github.com/rust-lang/rust/issues/101728)


_From the community that brought you the Pre-RFC and the e-RFC, we now introduce: the de-RFC!_

# Summary
[summary]: #summary

Type ascription ([RFC #803][ascript-rfc], [tracking issue]) has been a merged RFC for seven years, with no clear path to stabilization. Since then, syntactic norms in Rust have shifted significantly and it's becoming increasingly unlikely that this RFC if posted would have landed today. During this time the nightly support for this feature has impacted the quality of parser diagnostics; creating a large black hole of potential mistakes that lead to a suggestion around ascription (a feature most cannot use) when there could have been a more targeted and accurate diagnostic.

This RFC intends to advocate for the feature being removed entirely (or at least moving the implementation to a less prime area of the syntactic space) with a fresh RFC being necessary to add it again.

# Demotivation
[demotivation]: #demotivation

One of the primary demotivations is the negative effect on diagnostics. `:` is a pretty highly used syntactic component in Rust: a double colon is a crucial part of how paths work, and paths are found _everywhere_ in Rust (every variable and type name is a path). It's a rather easy to accidentally type `:` instead of `::`, and that is often interpreted as type ascription syntax. It's a terrible experience for the user to make an often hard-to-notice mistake and get told they are trying to use a feature they may not even have heard of. Here's [an example][ekuber-tweet] of a bad diagnostic caused by type ascription; while such diagnostics can be fixed, there are just so many of them. The fact that this is _still_ a problem despite the amazing work being put into diagnostics is a signal that it may always be a problem. And if this feature were to stabilize as-is, it would likely get worse since there would be backpressure to improve the diagnostics of legitimate uses of ascription. Good diagnostics are an exercise in guessing user intent; and the harder we make that the worse the diagnostics will be.

The other demotivation is a shift of syntactic norms.

Type ascription was originally RFCd in 2015. This is a time before `?`, an RFC that was _extremely_ controversial at its time but is now considered a very normal and sensible feature by the community. Similarly, while it may be harder to make that same exact claim about `.await`, the community has definitely softened on it since it was originally proposed. If type ascription were proposed today, it seems unlikely that the syntax would be chosen to be what it is now.

Syntax isn't the only reason; while type ascription is probably a good idea, a feature this significant deserves to be properly designed for the zeitgeist.

# Guide-level obfuscation
[guide-level-obfuscation]: #guide-level-obfuscation

The `:` type ascription syntax would be removed from the nightly language. It is up to the compiler team whether they wish to remove it completely from the compiler (or perhaps just make it unparsable and use some magical unstable `ascript!()` macro in the meantime so that it is testable).

This does not prevent future type ascription RFCs from happening, however they must propose the feature from first principles, and justify their choice of syntax. They are, of course, free to copy the work or text of the previous RFC.

# Reference-level obfuscation
[reference-level-obfuscation]: #reference-level-obfuscation

![diff shortstat of 275k removed lines](https://user-images.githubusercontent.com/1617736/187055431-2ab9f46b-4c23-4ec4-9884-d050501bf0c2.png)

# Drawforwards
[drawforwards]: #drawforwards

There are a couple advantages to keeping the feature around. In general, people do not seem against the idea of type ascription, rather, it's unclear to me that it will ever stabilize in _this form_. A potential path forward would be to simply restart the conversation around it and see what it would take to get it stabilized. This may have the same effective result of having the feature reexamined according to the _current_ Rust language and syntax and updated accordingly.

Perhaps even the existence of this de-RFC will spur someone to trying to do this.

# Irrationale and alternatives
[irrationale-and-alternatives]: #irrationale-and-alternatives

While the intent of this de-RFC is not to propose a new syntax, new syntax ideas that fit better into Rust today ought to illustrate why the feature should be removed.

Please do not use this RFC to discuss potential syntax; the examples below are to illustrate that there is a newer landscape of design choices; not to suggest any particular one.

`: Foo` and `as Foo` are both tightly-bound postfix syntaxes that don't _look_ tightly-bound. It's often surprising that e.g. `x / y as u8` has the cast apply to `y` and not the entire quotient expression, because it _looks like_ an arithmetic operator. Ascription syntax doesn't start with a space but it still has a similar problem due to the presence of the space. Perhaps that problem would go away if people got used to the syntax, but that's not clear.

On the other hand, the precedence for dot-operator postfix syntaxes — method calls, fields, and `.await` — is quite clear due to the lack of spaces. `?` benefits similarly though it's unary so it wouldn't have that problem either way. There has previously been talk of postfix macros which would also fall in this bucket.


Precedence isn't the only problem: chaining is a _huge_ problem with ascription (and `as`), where `foo: Foo.bar()` (and `foo as Foo.bar()`) doesn't work and you need to wrap it in parentheses. Given that popular targets for ascription like `.collect()` and `.into()` often return things the programmer wishes to process further, having to do `(x.into(): Foo).foo()` is not super ergonomic. `?` and `.await` have both been designed to avoid this problem, and it seems like we are in general moving away form needing parentheses to using chaining.


Some potential dot-postfix ascription syntaxes that could work are:

 - `.is::<Type>`
 - `.<Type>`
 - `.::<Type>`
 - `.become::<Type>` (already reserved! credit @mystor)


And that's just in the space of dot-postfix syntax. While the winds of Rust are blowing quite clearly in the dot-postfix direction, there are probably other syntax choices that would work well here too.



# Posterior art
[posterior-art]: #posterior-art

Rust has in the past removed nightly features entirely, sometimes even adding them back in a different form later.

For example, Rust's asynchronous programming support was removed by [RFC 230] before 1.0, and eventually came back in the form of Rust's pluggable async/await/Future support.

A lot more examples of nightly features being removed can be found [here][dispo-closed]. It's rather common for this to happen with libs features, less so for language features.

It's far more rare for this to happen to RFCd features, however, hence the de-RFC.

# Unresolved answers
[unresolved-answers]: #unresolved-answers

 - Should this be completely removed by the compiler, or left behind in a way that cannot be directly accessed through Rust syntax (or requires using a wrapper macro)?

# Future probabilities
[future-probabilities]: #future-probabilities


It's quite possible that in the future someone will have an RFC written to reintroduce this feature. Godspeed!

The clean slate given by this de-RFC may also lead to questions of whether pattern and expression ascription need to be the same feature: for example `:` syntax does make a lot of sense _in patterns_, and perhaps pattern ascription can use that whilst expression ascription ends up with something new. Maybe we need two RFCs!

It may also be worth looking at a lot of our other long-standing nightly language features that are in limbo and consider starting with a clean slate for them.

Finally, it may be worth coming up with a dot-operator postfix `as` syntax.

 [ascript-rfc]: https://rust-lang.github.io/rfcs/0803-type-ascription.html
 [tracking issue]: https://github.com/rust-lang/rust/issues/23416
 [ekuber-tweet]: https://twitter.com/ekuber/status/1554868154630897666
 [RFC 230]:https://rust-lang.github.io/rfcs/0230-remove-runtime.html
 [dispo-closed]: https://github.com/rust-lang/rust/issues?q=label%3Adisposition-close+label%3AC-tracking-issue