- Feature Name: N/A
- Start Date: 2026-10-19
- RFC PR: [rust-lang/rfcs#4021](https://github.com/rust-lang/rfcs/pull/4021)
- Rust Issue: none

Large language models were **not** used in the production of this RFC.

## Summary
[summary]: #summary

`std::env::args` and especially `std::env::vars` are footguns whose existence in the standard
library contradicts modern best practices of standard library API design. They do the language
user a disservice by obfuscating the handling of errors and elevating an ignorant and buggy
approach above one that is correct and only marginally more verbose. Accordingly, deprecate `args`
in favor of `args_os` and `vars` in favor of `vars_os`, and, in future editions, rename `args_os`
to `args` and `vars_os` to `vars`.

## Motivation
[motivation]: #motivation

The command-line argument vector and the environment block consist of entries that biject to
`OsString`. In the majority of cases, those entries are valid Unicode strings and a subsequent
conversion to `String` will succeed, but that is not true in the general case.

An example where the assumption that it will always succeed compromises program robustness is
Unix systems with the `ja_JP.sjis` (Shift JIS) locale. The Shift JIS encoding still sees use among
users in Japan, and certain systems may use it to the point of having Shift JIS filenames, or
environment variables whose value was set to data containing Shift JIS by some program.

An example where this assumption compromises program security is a Rust program with security
duties running under an untrusted parent process, as might be the case when the program is
set-user-ID root or has Linux file capabilities.

The distinguishing property of both `std::env::args` and `std::env::vars` is that they call
`.unwrap()` on a `Result` of a conversion from `OsString` to `String` when the caller could have
just as easily done the `unwrap` themselves. An equivalent to `args` can be obtained via
`args_os().map(|s| s.into_string().unwrap())`, and an equivalent to `vars` can be obtained via
`vars_os().map(|(k, v)| (k.into_string().unwrap(), v.into_string().unwrap()))`.

To provide a helper for promoting invalid user input to panics in the public API is in itself
an anti-pattern in Rust library design, but one that does not in itself warrant deprecation.
Unfortunately, both functions, in addition to being unidiomatic and unnecessary, constitute
footguns that cause bugs in real programs, even otherwise high-quality ones, and no wonder:
`vars` and `args` are shorter than `vars_os` and `args_os`, which carries the implication that
they are the default or typical iterators to use and that their `OsString` counterparts are for
advanced use and special cases. The documentation of `vars` and `args` at present significantly
undersells how fringe their use cases really are, and it is empirically true, as will be shown
below, that many people, including proficient Rust programmers such as rustc contributors, are
being misled (consciously or subconsciously) by the shape of the API into using the iterators that
have the wrong error handling.

The below two sections detail the footguns and how they affect the robustness of programs that
use them.

### `std::env::vars`

This is the substantially greater offender of the two in that the returned iterator is impossible
to use correctly in non-throwaway programs, as per the below polysyllogism, keeping in mind that
environment variables are user input which the program does not have any control over in the
general case.
1. Although the purpose of `vars` and `vars_os` is to enable programs to work with whole
   families of environment variable keys which are not exhaustively known in advance, it is never
   meaningful for a program to use the value of literally every single environment variable key
   for any purpose other than passing the environment through, serializing it, or performing
   other forms of environment plumbing – there is simply nothing to do about arbitrary environment
   variables meant to affect the behavior of programs that are not the current program other than
   pass them through verbatim to some destination. Accordingly, all use cases of iteration through
   the environment block either do not care about the UTF-8 validity of at least some of the
   variables (because the program ignores them), or do not care about their UTF-8 validity at all
   (because the program passes them through).
   - As a side note, some programs may have a (significantly misguided, but technically not
     incorrect) need to sanitize their entire environment block for UTF-8 validity, failing if any
     of the keys or values are not valid UTF-8. For those programs, panicking on invalid UTF-8 is
     the opposite of the desired behavior of producing a tractable sanitization failure.
2. The `OsString` type, as returned by `vars_os`, is normally used when a Rust program needs to
   pass a string received from the operating system along without attempting to interpret it.
3. Therefore, it is never correct to use `vars` in programs that do not wish to potentially ignore
   one or more environment variables.
4. The order of environment variables in `vars` is unspecified, meaning that, if the environment
   block contains any non-UTF-8 entries at all, there is always the possibility of the very first
   environment block entry traversed by the iterator to be a non-UTF-8 entry the program never
   needed to inspect to begin with.
5. Therefore, all programs for which it is not invalid per ③ to use `vars` are exposed to the
   hazard of panicking during UTF-8 validation of the key and value of an environment variable
   entry that the user does not intend the program to inspect.
6. Crashing due to inputs destined to a completely different program and otherwise ignored
   is bad and wrong.
7. Therefore, programs that use `vars` are guaranteed to have behavior that is bad and wrong.

As an aside, even presuming that the hazard described above is somehow okay to have for certain
(necessarily user-unfriendly) programs, validating UTF-8 of both the key and the value before a
program knows that the key of the environment block entry is of interest to it is a completely
unnecessary performance cost. As such, `std::env::vars` is also a minor performance footgun in
addition to being a significant robustness footgun.

It is also prudent to look at the real-world situation. A
[GitHub search for uses of `std::env::vars`][gh-search-vars] reveals 21.6k results, of which the
majority involves taking `std::env::vars` and `filter`ing or `filter_map`ping it to those where
the key `starts_with` with a literal. This use case is trivial to replace with `vars_os`, where
`k.starts_with("SOME_LITERAL_")` is to be replaced with
`k.as_encoded_bytes().starts_with(b"SOME_LITERAL_")`. In a good few results, it can be trivially
replaced with `var_os`, as there is only one key being queried, often for existence and not its
value.

[gh-search-vars]: https://github.com/search?q=lang%3ARust%20std%3A%3Aenv%3A%3Avars()&type=code

### `std::env::args`

Unlike with `vars`, there theoretically exist programs in which use of `args` produces correct
behavior, but those programs are disappearingly rare in practice. Recall that the first element in
the argument vector is, by convention, the path by which the program was invoked: for example, if
a program named `my-prog` is invoked as `my-dir/my-prog`, its argument 0 will be `my-dir/my-prog`.
Of course, if the program uses `args` and the name of `my-dir` contains invalid UTF-8, the program
will crash during command line argument parsing, even if it uses `.skip(1)` to ignore argument
0 entirely.

This means that, for `args` to never cause unnecessary panics, all of the following must be true:
- The program does not ignore argument 0, and the entire path in argument 0, not just the final
  component, needs to be UTF-8 for it to work correctly.
- All subsequent arguments, including ones that specify paths, are also required to be UTF-8 for
  reasons that will be remarked as silly but not further questioned here.
- It is acceptable to panic instead of displaying a user-friendly error during validation of both
  of the above conditions.

Hopefully, it is clear that programs for which use of `args` is actually appropriate are highly
exotic. What is not exotic, however, is programs using `args` when they should've used `args_os`,
[with the most recent example being rustc itself][rustc-args]. As for a
[GitHub search for `std::env::args`][gh-search-args], the number is a staggering 339k results,
many of them being `args().skip(1)`, `args().nth(1)`, and `args().skip(1).len()`, all of which
will senselessly panic on non-UTF-8 argument 0.

[rustc-args]: https://github.com/rust-lang/rust/issues/164585
[gh-search-args]: https://github.com/search?q=lang%3ARust%20std%3A%3Aenv%3A%3Aargs()&type=code

## Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

`std::env::vars` and `std::env::args` are deprecated and should not be used in new code. Their
existing usage is to be replaced with `std::env::vars_os` and `std::env::args_os` respectively,
and it is additionally prudent to audit call sites of the former for potential conversion to a
call to `std::env::var_os`, or, in certain cases, `std::env::var`. The following conversion
patterns are to be applied:
```rs
// before
let environment = std::env::vars().collect::<Vec<_>>();

// after
let environment = std::env::vars_os().collect::<Vec<_>>();
```
```rs
// before
std::proces::Command::new("my-program")
    .envs(std::env::vars())

// after
std::proces::Command::new("my-program")
// (environment variables are inherited by default)
```
```rs
// before
std::proces::Command::new("my-program")
    .envs(std::env::vars().filter(|(k, _)| is_env_var_permissible(k)))

// after
std::proces::Command::new("my-program")
    .envs(std::env::vars_os().filter(|(k, _)| is_env_var_permissible(k)))
```
```rs
// before
let cargo_features = std::env::vars()
    .filter_map(|(k, _)| Some(k.strip_prefix("CARGO_FEATURE_")?.to_owned()));

// after
let cargo_features = std::env::vars_os()
    .filter_map(|(k, _)| {
        let k = k.as_encoded_bytes().strip_prefix(b"CARGO_FEATURE_")?.to_owned();
        String::from_utf8(k).ok()
    });
```
```rs
// before
let has_foo_set = std::env::vars().any(|(k, _)| k == "FOO");

// after
let has_foo_set = std::env::var_os("FOO").is_some();
```

## Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

Deprecate `std::env::vars` and `std::env::args`. Rename `std::env::vars_os` to `std::env::vars`
and `std::env::args_os` to `std::env::args` in an edition change to mortalize the design mistake.

## Counterarguments and drawbacks
[counterarguments-and-drawbacks]: #counterarguments-and-drawbacks

> Those iterators are very useful for scripts and throwaway programs, where it is absolutely valid
> to have assumptions about UTF-8 validity of command-line arguments and environment variables.

That is true, but their presence, being of very magrinal utility, is not without a severe cost
(refer to the lead section of [*Motivation*][motivation]). Deprecation of `vars` and `args` would
benefit the ecosystem decidedly more than it would hurt scripting use cases, where a small amount
of additional typing would be necessary to perform error handling in the style that those scripts
already perform it for every other invariant they depend on.

> Invalid UTF-8 in the environment block and in filesystem paths is highly uncommon. So what if
> the default behavior of most Rust programs is to not work in exotic environments that likely
> break other programs anyway?

It is true that the edge cases that cause issues with `vars` and `args` occur infrequently, but
that is no excuse to ignore them; if that were the case, it would be unnecessary for the standard
library to have `OsString` to begin with. The design of the Rust language and standard library
strives to encourage people to handle edge cases not because they happen to cross some threshold
of frequency, but because it is the job of robust software to react appropriately to unusual
user input and to avoid introducing failure states not inherent to the problem space and the
application implementation techniques in use.

Furthermore, it is in actuality not common for programs written in other programming languages to
crash if they are invoked by a non-UTF-8 relative path or if their environment contains non-UTF-8
keys or values. This is discussed in the [*Prior art*][prior-art] section.

> Too many programs use `vars` and `args`. The deprecation warnings will be everywhere!

That is quite okay. The purpose of deprecation is to conspicuously but not forcefully inform
people of issues with APIs they are using and discourage them from continuing to do so, and that
is exactly what is necessary to do with `vars` and `args`.

## Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

- Elide the renaming of `vars_os` to `vars` and `args_os` to `args` on the next edition change.
  There is no obvious benefit to doing so, and it would keep the former design mistake visible
  in the standard library, which is undesirable, as the seemingly off-the-beaten-path names of
  `vars_os` and `args_os` are in all likelihood a large part of the reason why `vars` and `args`
  are used so frequently to begin with.
- Keep everything as is, with all the footguns and unnecessary panics, endure potential ridicule
  from the communities of other programming languages, and allow un-Rust-like APIs with easy
  idiomatic alternatives to remain in one of the most basic parts of the standard library.

## Prior art
[prior-art]: #prior-art

### C and C++
The encoding of values in the argument vector and environment block is left to the program's
own interpretation, meaning that all error conditions raised on invalid UTF-8 in either of those
places cannot come from anywhere other than the application programmer or a library author. In
practice, many C and C++ programs are indifferent to the encoding of the environment variables
they `getenv`, treating their contents not unlike how `var_os`, `vars_os`, and `args_os` do.

### Python
Invalid UTF-8 in the argument vector and environment block on Unix-like systems is converted into
rather strange-looking UTF-16 surrogates that cannot be converted to UTF-8:
```
[I] goat@CHARA /tmp> python -c 'print(*__import__("sys").argv)' (printf '\xc0')
Traceback (most recent call last):
  File "<string>", line 1, in <module>
    print(*__import__("sys").argv)
    ~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^
UnicodeEncodeError: 'utf-8' codec can't encode character '\udcc0' in position 0: surrogates not allowed
[I] goat@CHARA /tmp> env A=(printf '\xc0') python -c 'print(__import__("os").environ["A"])'
Traceback (most recent call last):
  File "<string>", line 1, in <module>
    print(__import__("os").environ["A"])
    ~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
UnicodeEncodeError: 'utf-8' codec can't encode character '\udcc0' in position 0: surrogates not allowed
```
Note that this exception only occurs when printing of the command-line argument or environment
variable value is attempted and that simply accessing it (as one might while iterating through the
command-line arguments akin to `std::env::args` or while iterating through the environment block
akin to `std::env::vars`) does not raise any exceptions:
```
[I] goat@CHARA /tmp> python -c 'a = __import__("sys").argv[1]; print("Everything is fine")' (printf '\xc0')
Everything is fine
[I] goat@CHARA /tmp> env A=(printf '\xc0') python -c 'a = __import__("os").environ["A"]; print("Everything is fine")'
Everything is fine
```

### Haskell
Does the same thing as Python:
```
[I] goat@CHARA /tmp> cat hi.hs
import Data.Char (ord)
import Numeric (showHex)
import System.Environment (getArgs, getEnv)
main :: IO ()
main = ((hexFirstArg <$> getArgs) >>= putStrLn) >> ((hexFirst <$> getEnv "A") >>= putStrLn)
  where hexFirst    (c:_) = showHex (ord c) ""
        hexFirstArg (a:_) = hexFirst a
[I] goat@CHARA /tmp> env A=(printf '\xc0') runhaskell hi.hs (printf '\xc0')
dcc0
dcc0
```

### Go
Strings in Go are not guaranteed to be UTF-8. Accordingly, the standard library does not check the
argument vector or environment block for UTF-8 validity, and programs proceed as normal until they
attempt to do something with the strings that requires them to be UTF-8.

### Java
For both the argument vector and the environment block, performs the equivalent of
`String::from_utf8_lossy` (converts into �):
```
[I] goat@CHARA /tmp> cat Hi.java
public class Hi {
    public static void main(String[] args) {
      for (var arg : args) System.out.format("%d %s\n", arg.codePointAt(0), arg);
      var a = System.getenv("A");
      System.out.format("%d %s\n", a.codePointAt(0), a);
  }
}

[I] goat@CHARA /tmp> /usr/lib/jvm/java-21-openjdk/bin/javac Hi.java
took 0.301 seconds
[I] goat@CHARA /tmp> env A=(printf '\xc0') /usr/lib/jvm/java-21-openjdk/bin/java Hi (printf '\xc0')
65533 �
65533 �
```

## Unresolved questions
[unresolved-questions]: #unresolved-questions

None.

## Future possibilities
[future-possibilities]: #future-possibilities

It would be worthwhile to audit a few more panic points in the standard library on the question
of whether they are appropriate. `std::env::set_var`'s panicking behavior comes to mind, but there
might be other APIs with similar issues as well.
