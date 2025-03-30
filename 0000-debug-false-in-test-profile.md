- Feature Name: debug_false_in_test_profile_by_default
- Start Date: 2025-03-31
- RFC PR: [rust-lang/rfcs#0000](https://github.com/rust-lang/rfcs/pull/0000)
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)

# Summary
[summary]: #summary

In the test profile debug should have a false value by default.

# Motivation
[motivation]: #motivation

Many Rust developers say that target dir size is one of the things that doesn't look good. Especially if developers are using a shared target directory, it can grow to monstrous sizes. (I've seen some reports about hundreds of GB's on Twitter)

New Rust developers that come from other language backgrounds also report large sizes of target dir as a strange Rust thing.

Making debug = false a default value for test profile would help to address this issue partly.

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

I have a test template library. After I execute the cargo test, du command shows me the following directory size.

```sh
du target/
╭───┬───────────────────────────────────────┬──────────┬──────────╮
│ # │                 path                  │ apparent │ physical │
├───┼───────────────────────────────────────┼──────────┼──────────┤
│ 0 │ /home/michal/projects/test-lib/target │   7.0 MB │   7.1 MB │
╰───┴───────────────────────────────────────┴──────────┴──────────╯
```

I'm changing the profile debug value.

```toml
[profile.test]
debug = false
```

After running cargo clean and running cargo test again I have the following directory size.

```sh
 du target/
╭───┬───────────────────────────────────────┬──────────┬──────────╮
│ # │                 path                  │ apparent │ physical │
├───┼───────────────────────────────────────┼──────────┼──────────┤
│ 0 │ /home/michal/projects/test-lib/target │   1.4 MB │   1.5 MB │
╰───┴───────────────────────────────────────┴──────────┴──────────╯
```

Of course these numbers comes from template project. In real world projects we are talking about GB's.

IMO most of the users don't need debug information while executing tests. In case they would need it for some reason they can enable it in Catgo.toml. This change would improve the experience for the majority of the developers and would not break the experience to anyone, because the previous default is easy to reenable.

Another thing is that disabling the generation of debugging information increases the speed of compilation.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

# Drawbacks
[drawbacks]: #drawbacks

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

# Prior art
[prior-art]: #prior-art

# Unresolved questions
[unresolved-questions]: #unresolved-questions

# Future possibilities
[future-possibilities]: #future-possibilities
