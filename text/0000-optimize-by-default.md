- Start Date: 2015-03-10
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Make rustc and cargo produce optimized binaries by default.

# Motivation


Optimizing by default is a safety (in the general sense) & usability feature
that everyone benefits from, but it's extra important for newcomers. Even
experienced developers coming from dynamic languages like JavaScript, Python,
Ruby or even static, enterprise languages like Java and C# are probably
completely unaware of the concept of "debug" and "optimized" builds. Note that
the above 5 languages represent the vast majority of the "first language" for
people learning to code today.

`rustc` needs to be designed so that its users **[fall into the pit of
success](http://english.stackexchange.com/questions/77535/what-does-falling-into-the-pit-of-success-mean/77541)**.
Today this is sadly not the case; a newbie running `rustc foo.rs` will get a
slow build by default, and might easily conclude that the equivalent code they
wrote in Java is faster and that Rust can then be discounted. This isn't a
theoretical concern; situations similar to this come up on the `#rust` IRC
channel and on the `/r/rust` subreddit all the time.

- [This user ran `cargo build`][ex1] and expected a fast binary, which is
  entirely sensible and yet _we failed them._
- [Here's another case.][ex2]
- [Yet another example.][ex3]
- [Experienced Rust user forgetting optimizations][ex6]. It's not just newbies
  who are getting bitten by this.
- [A user on IRC forgetting about optimizations][ex7] and thus confused about
  large binary size.
- [Seriously, we can do this all day.][ex4]

Anyone with more that passing experience with Rust will know how to get a debug
build if they need one, but the default should certainly be an optimized build.

The only reason why the unoptimized build is the default is convention started
many decades ago by ancient compilers that _didn't_ have an optimized build at
all, so when one was added later on, it was added behind a flag one had to pass
so as not to break the way people were using the compilers already. I don't
think anyone can make a reasonable case for "unoptimized by default" in a vacuum
where no previous precedent is set by other compilers.

`rustc` should break with convention here because in this case, the conventional
approach _is hurtful to users._ Today's default is "fail-deadly" instead of being
"fail-safe;" you forget to pass a flag you might not even be aware of and you
get a build that's very slow and almost certainly not what you wanted.

This problem is present with people who use C or C++ as well; StackOverflow
is full to the brim with examples. Here's an example from the Rust subbreddit of
[a user accidentally compiling C (and Haskell) code without optimizations][ex5]
and then comparing with Rust.

**Debug-by-default is a poor design.** It's a footgun of massive proportions.
Rust is already breaking with programming convention in a myriad ways to
eliminate footguns (which is excellent); it should break with convention here as
well.

**Let's not leave footguns in place because of obsolete conventions created by
backwards compatibility decisions made 40 years ago that aren't relevant to
Rust.**

## Why should we do this before 1.0?

Because doing it after 1.0 breaks backwards compatibility on a command-line
level. People will expect `rustc`/`cargo` to behave a certain way and will encode
those assumptions into their bash command history/build files/scripts/whatever.

# Detailed design

Change `rustc` and `cargo` to produce optimized binaries by default.

`cargo` currently has a `--release` flag; it should also get a `--debug` flag to
trigger a debug build. By default, running `cargo build` is the same as running
`cargo build --release`.

Similarly, `rustc` should by default run with the same flags that it would
receive from `cargo` if it were passed `--release`. It might make sense to also
add support for `--release` & `--debug` flags to `rustc`, so that with
`--debug` it becomes easy to instruct the compiler to not include optimizations
and to include debug info.

Debate is welcome on the specifics as long as "unadorned" `rustc` and `cargo
build` calls produce optimized binaries.

# Drawbacks

## Slower builds by default

Builds will only be slower _by default_; in other words, anyone calling `rustc
foo.rs` today can trivially call `rustc -C optlevel=0 foo.rs` or `rustc --debug
foo.rs` if the extra `rustc` flags are implemented.

Note the failure state of debug-by-default: if you wanted an optimized build
instead, your production binary is now incredibly slow. As someone who has
actually pushed unoptimized binaries to production by accident, I can
confidently say _this costs a lot of money._

For opt-by-default, if you actually wanted a debug build the failure state is
"you waited longer for your build to finish than you should have," which while
annoying, has _far_ less potential for damage.

## Breaks convention with compilers for other languages

While this is a real concern, it's the result of those compilers continuing to
carry a legacy burden. C compilers need to be backwards compatible with the
state-of-the-art from many decades ago, and C++ compilers need to be compatible
with C compilers. In other words, they don't have a choice.

Rust has been breaking plenty of language conventions for the sake of both
memory-safety and safety in general; it should continue on this commendable path
without being hamstrung by decisions made in 1972.

# Alternatives

## Only have `cargo` echo a message stating `debug` or `release` build mode

This has been recently talked about, and while it's a welcome and useful
feature, it sadly isn't enough. A Rust newcomer who's only used Python, Ruby,
JavaScript, Java, C# or a similar language is not going to understand the
consequences of a "building in debug mode" message.

Not to mention that it's easy to miss such messages when `cargo build` is run by
a script that will also call plenty of other commands and will thus have lots of
output.

Not to mention it does nothing for `rustc` calls by newbies. See the next
section for details.

## Only make `cargo` opt-by-default, but leave `rustc` as is

While this alone would be a large (general, not memory) safety & usability boost
over the current state, it would not help Rust newcomers or those playing around
with/evaluating the language. Such users are less likely to know about `cargo`,
and even if they do, are likely to shun it while experimenting with simple
programs. In such cases, they could easily conclude that "Rust is too slow"
based on their initial experiments and never reach `cargo build`.

`rustc foo.rs` is understandably very tempting when trying to wrap one's head around the
language and it would (like today) speak badly about Rust's performance.

## Make `rustc` include debug info along with optimized code

This is better than the previous alternative, but would still lead to questions
about small Rust programs leading to huge binaries.

### Have _no_ default; always force inclusion of `--debug` or `--release`

In theory, a default isn't necessary and we can always force the user to specify
what type of build they'd prefer. `rustc foo.rs` would produce an error message
informing the user about the necessity of choosing either a debug or release
build. The message would include appropriate documentation links so that users
unfamiliar with the concept of such build modes could learn more about them.

IMO this is the least bad alternative that would also prevent accidental usage
of unoptimized binaries.

This brings a possible issue of unnecessary complexity for newbies (and others).
There's something to be said for "if the user gives a compiler a program file,
they should get back a binary." Avoiding a reasonable, safe default and forcing
a choice every time seems like an unnecessary usability hurdle. We know what the
safe default is, and it certainly isn't debug-by-default, so why avoid making it
opt-by-default.

# Unresolved questions

- Should `rustc` gain new flags in addition to `--debug` for `cargo`? Without a
  new flag for `rustc`, getting a debug build (no optimizations but with debug
  info) might be fairly verbose.

# See also

[Original discussion on Discuss.](http://internals.rust-lang.org/t/optimizing-by-default/1532)

[ex1]: http://www.reddit.com/r/rust/comments/2vzxjr/poor_http_performance_iron_framework/
[ex2]: http://learncamlirust.blogspot.de/2015/02/day-1-porting-rollingsum.html
[ex3]: http://www.reddit.com/r/rust/comments/2xccw8/performance_of_reading_a_large_file/coyuz8l
[ex4]: https://www.reddit.com/r/rust/comments/2yk5z7/why_this_rust_code_slower_than_c/cpa97dh
[ex5]: https://www.reddit.com/r/rust/comments/2y0bas/benchmark_rustnom_vs_haskellattoparsec_vs_chammer/cp51mxe
[ex6]: https://botbot.me/mozilla/rust/2015-03-06/?msg=33512540&page=14
[ex7]: https://botbot.me/mozilla/rust/2015-02-18/?msg=32236594&page=5
