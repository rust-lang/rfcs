- Feature Name: first_sets_future_proofing
- Start Date: 2016-08-09
- RFC PR:
- Rust Issue:

# Summary
[summary]: #summary

Extend the restriction on `macro_rules!` macros using the concept of *FIRST sets*
to be able to check more complex macros (that is, multiple arm macros or macros
using sequence repetitions).

# Motivation
[motivation]: #motivation

`macro_rules!` are not future-proof, which means there exist valid programs where macro invocations could change behaviour if some grammar productions were added
to the language.

An existing solution based has been described and implemented. It aims at avoiding such macros by restricting what macros can and can't rely upon on one side, and what kind of grammar productions we can add on the other
side.

It relies on the concept of *FOLLOW sets*. It restricts macros matchers by
specifying what tokens can occur after a non-terminal matcher in a macro pattern.

There are [complex problems](https://internals.rust-lang.org/t/on-macros-future-proofing-follow-sets-and-related-stuff/) with this solution that make it
insufficient because it fails to check macros that feature two “parallel” parsing
possibilites, that is:

* Multiple arm macros
* Single-arm macros with sequence repetitions, where an input could match either
  the matchers inside the sequence or what comes after. This is currently
  blocking to be able to “backtrack“ when parsing macro-rules.

The problems with multiple arm macros [have been known for a long time](https://github.com/rust-lang/rust/issues/30531).
An example of an issue with sequence repetitions [can be found here](https://github.com/rust-lang/rust/pull/33840#issuecomment-224869252).

# Detailed design
[design]: #detailed-design

This new solution is intended to be added as a complement to the current
solution based on FOLLOW sets, not replace it.

### Problem statement

Let's start with a few examples:

```rust
macro_rules! foo(
    ( foo $e:expr ) => ... ;
    ( bar $i:ident : $t:ty ) => ...
)
```

The first arm of the macro will always match input sentences beginning with the `foo` identifier, and nothing else. The second arm will always match input sentences beginning with the `bar` identifier and nothing else. Thus, whatever changes are made to the languages recognized by `expr` or `ty`, no input sentence matched by the second arm will ever be matched by the first arm. The behaviour of this macro won't change in the future, we say that it is *future-proof*.

```rust
macro_rules bar(
    ( $e:expr ) => ... ;
    ( $i:ident : $t:ty ) => ...
)
```

A sentence of the form `id : t` where `t` is a valid type currently matches the second arm but not the first. If a grammar production such as `expr ::= expr : ty` was added to the parser, then such sentences would now match the first arm, which has priority, changin the behaviour of the macro for those inputs. In fact, such a production has already been added to the nightly version of the language. This macro is not future-proof.

The problem could be defined informally as: if some input sequence doesn't match an arm's matcher (mi) right now but might in the future, then none of the matchers of the arms that are placed after this matcher can accept this sequence, neither now nor in the future.
                           
Or, more formally:

Let `m1`, `m2`, ..., `mn` be the n matchers of a macro. We want to check that for any matcher `mi`, the following property is true:

For all input sequence `s`, `s ∈ MAYBE(mi) => forall mj in mi+1 ... mn, s ∈ NEVER(mj)`
                          
Where:

* `NEVER(m)` is the set of inputs that will never be accepted by a matcher m
* `NOW(m)` is the set of inputs that are now accepted by a matcher m
* `MAYBE(m)` is defined by: `forall sentence s, matcher m: s ∈ MAYBE(m) <=> s ∉ NOW(m) ⋀ s ∉ NEVER(m)`

Of course, the problem of deciding wether some input sequence may match some matcher in the future (that is, if, for a given matcher m, wether it belongs to NOW(m), MAYBE(m) or NEVER(m)) is virtually impossible. Instead, we use the concept of *FIRST sets* as an approximation.

### FIRST sets

For each non-terminal nt, we define `FIRST(nt)` the set of tokens that are
allowed to begin a sequence that matches this non-terminal, or will possibly be
in the future (this is the same, by computing the difference with the set of all
Rust tokens, as specifying a set of tokens that will never be allowed to start a
sequence that matches the non-terminal). That is, we specify that for every sentence `s`, `s ∈ NOW(nt) ⋁ s ∈ MAYBE(nt)` if and only if the first token of `s` is in `FIRST(nt)`. By doing this, we restrict our ability to expand the Rust language. For example, stating that the `@` token is not in the `FIRST(expr)` would prevent us from adding any grammar production that would make `expr` parse sentences that begin with an `@`.

Using FIRST sets, we can construct a sound approximation of the above: if `m1` and `m2` are two macro matchers, if `FIRST(m1)` and `FIRST(m2)` are disjoint, then we know that for every possible input sequence `s`, we have `s ∈ NEVER(m1) ⋁ s ∈ NEVER(m2)`, which is clearly strictly stronger than the desired property.

Using this, we can look for “obvious disambiguations”, that is, matchers whose accepted langages (or that may be accepted in the future) are mutually exclusive (their intersection is empty).

### Algorithm

This is still very conservative. To relax it a bit, we can, in some cases, continue the analysis to the next fragment of the matcher, provided that we can skip the same number of token trees in both matchers. We have a ‶concatenation property″:

Informally: if two matchers constitute an obvious disambiguation, then if we concatenate them with any other matchers, then the resulting matchers themselves constitute an obvious disambiguation. However, for the case where we concatenate them with a *prefix*, we have to make sure those prefixes *always* (that is, for all their possible inputs) match the same number of token trees, so that the original matchers will still try to match input in ‶parallel″ (‶at the same place in the input sentence″).

Example: `foo $e:expr` and `bar $i:ident : $t:ty` are an obvious disambiguation since no input sentence can match both (either a sentence starts with `foo`, it starts with `bar`).

* We clearly see that the same property is true whatever we add *after* those matchers: `foo $e:expr t1` and `bar $i:ident : $:ty t2` are still an obvious dismabiguation, for any `t1` and `t2`.

* If we add them prefixes who always match the same mumber of token trees, for example `( t1 ) foo $e:expr` and `( t2 ) bar $i:ident : $t:ty`, the property still holds. Whatever are `t1` and `t2` and whatever is the input sentence, we know that we will at most parse one token tree before reaching a point where `foo $e:expr` and `bar $i:ident : $t:ty` will try to parse the same fragment of the input sentence. Only one can succeed.

* If, however, this requirement on the number of token trees is not respected, this does not hold. Take for example `$expr foo $e:expr` and `$i:ident + bar $i:ident : $t:ty` and consider the input `foo + bar foo : i32`. This input could obviously match the second arm. But if we add to the language a production that allows `expr` to match `: i32`, then it could also match the first arm. Of course, this particualr language modification is unlikely to happen, but let's consider it anyway for the sake of the example. All that this example prooves is that it's impossible to say anything here, because the `expr` prefix can match an arbitrary number of token trees, thus leaving, in this example `foo $e:expr` parsing in parallel with `foo: i32`, which is not an obvious disambiguation.

Formally:

* for all matchers `ma`, `mb`, if for every input sentence `s`, `s ∈ NEVER(ma) ⋁ s ∈ NEVER(mb)`, then for any two matchers `ma'`, `mb'` we have:    for every input sentence `s`, `s ∈ NEVER(ma' ma) ⋁ s ∈ NEVER(mb' mb)`
provided that `ma'` and `mb'` always match the same number of token trees.

* for all matchers `ma`, `mb`, if for every input sentence `s`, `s ∈ NEVER(ma) ⋁ s ∈ NEVER(mb)`, then for every matchers `ma'`, `mb'` we have: for every input sentence `s`, `s ∈ NEVER(ma ma') ⋁ s ∈ NEVER(mb mb')`.

The algorithm relies entierly on this property and works the following way. We skip sub-matchers when we know exactly how many token trees they match until (in practice, it's only the case of matchers that always parse a single token-tree):

- We find two submatchers `a` in `A` and `b` in `B` such that for every sentence `s`, `s ∈ NEVER(a) ⋁ s ∈ NEVER(b)`, which implies – by the concatenaion property and by the fact that we only skip single-token tree matchers – that for every sentence `s`, `s ∈ NEVER(ma) ⋁ s ∈ NEVER(mb)`. We call such a pair of matchers an obvious disambiguation.

- We find two submatchers for which we do not know the exact number of token trees that they match, in which case we conservatively reject since we cannot know where we can continue the analysis.

- Otherwise, we continue, by using the contatenation property.

If at some point we find two submatchers for which we have no proof that there does not exist a sentence that is in `MAYBE(a)` but not in `NEVER(b)`, then, if we know the number of token trees matched by those submatchers, we can still continue to find an obvious disambiguation. We call such a pair of matchers an ‶error″.

If we skipped all the submatchers and reach the end of both the matchers without finding either a disambiguation or an error, then we have no proof that there are input sentences that are in `MAYBE(a)` but not in `NEVER(b)`. Since the analysis is conservative, then we can accept the matchers.

```rust
// check macro matchers two by two
fn check_matchers(ma, mb) {
    // need_disambiguation is set to true whenever
    // we encountered an ‶error″ at some point. we
    // need to find an obvious disambiguation to
    // accept this macro.
    need_disambiguation := false;

    // analyse until one of the cases happen:
    // * we find an obvious disambiguation, that is a proof that all inputs that
    //   matches A will never match B or vice-versa
    // * we find a case that is too complex to handle and reject it
    for a, b in ma, mb {

        // sequence analysis:
        // we must first handle the case for sequence repetitions, that is,
        // when a and/or b are of shape $(...)* or $(...)+.
        // I left their treatment to the next section in order to ease the
        // reading and understanding of this algorithm.
        // ignore them for now, and assume in the following that a and b
        // can never be sequence repetitions.

        if a and b are the same matcher {
            // if two matchers are the same matcher, then they always
            // match the same input, so the same number of token trees.            
            continue;
        }

        if FIRST(a) ∩ FIRST(b) = ∅ {
            // accept the macro: this is an obvious disambiguation.
            // we are sure that because of this, all inputs belong to
            // either NEVER(ma) or NEVER(mb)
            return Ok
        }        

        // now we cannot say anything in the general case but we can
        // still look if we are in a particular case we know how to handle...
        match a, b {
            _, NT(nt) if !nt_is_always_single_tt(nt) => {
                // at this point, we cannot say anything about what B might
                // accept in the future and we don't know where to continue,
                // so the only option is to make sure that A will never start
                // match new input so that MAYBE(A) = ∅
                // the function only_simple_tokens will return true if the
                // remaining sub-matchers of A are only made of tokens,
                // the `$tt`, or `$ident` NTs, or sequence and delimited
                // matchers that only contain similar objects. such a
                // matcher will effectively never match new input (unless
                // the Rust tokens themselves are expanded, which is not our
                // concern here, the important thing is that the behaviour of
                // those matchers will always remain the same).
                if only_simple_tokens(ma) && !need_disambiguation {
                    return Unsure
                } else {
                    return Error
                }
            }

            // invariant: now we know that B always matches a single TT
            // first case: NT vs _.

            NT(nt), _ if nt == "ident" || nt == "tt" => {
                // ident or tt will never start matching more input
                // as far as this specific sub-matcher is concerned,
                // MAYBE(A) = ∅
                continue
	        }

            NT("block"), _ {
                // FIRST(NT("block")) = { `{` }.
                // this means b is either NT("tt") or a brace-delimited TT.
                // we cannot say much here: even if the syntax of block should
                // not change, a block contains statement and expressions whose
                // syntax might change so that the whole block starts matching
                // B. we cannot look inside.
                // (Note: we could expand block to its definition, something
                // like `{ $(s:stmt);* }` but I'm not 100%-certain this would
                // work and complicates the analysis quite uselessly, since this
                // specific case appears to be quite rare.)
                // we can just hope we will find an obvious
                // disambiguation later:
                need_disambiguation <- true;
                continue;
            }

            NT(_), _ => {
                // A is a NT matcher that is not tt, ident, or block (that is, A
                // could match several token trees), we cannot know where we
                // should continue the analysis.
                return Error
            }

            // invariant: both A and B always match a single TT
            // second case: T vs _.

            Token(_), _ => {
                // the token will never match new input,
                // MAYBE(A) = ∅
                continue;
            }

            Delimited(delim), NT(_) => {
                // either { ... } vs `$block` or any delimited vs `$tt`.
                // as with several-TTs NTs, if the above is only
                // made of simple tokens (MAYBE(a) = ∅) this is ok...
                // otherwise A could start matching input from B, which
                // could be anything.
                if !only_simple_tokens(delim.tts) {
                    need_disambiguation <- true;
                }
                
                // we can still find an obvious disambiguation later.
                continue;
            }

            Delimited(d1), Delimited(d2) => {
                // we know that they have the same delimiter since their
                // FIRST sets intersect.
                // recursively descend into delimiters.
                match check_matchers(d1.tts, d2.tts) {
                    Ok => {
                        // there was an obvious disambiguation inside
                        return Ok
                    }
                    
                    Unsure => {
                        // no problem but we must continue.
                        continue
                    }
                    
                    Error => {                    
                        need_disambiguation = true;
                        continue
                    }
                }
            }
        }
    }

    // now we are at the end of one arm:
    // if the other arm always accept new input, that is, if it cannot accept
    // the end of stream, then this is a disambiguation.
    for every remaining submatcher m in the remanining arm {
        match m {
            Sequence(seq) if seq.op == `*` => continue,
            _ =>
                // this arm still expects input, while the other can't.
                // use this as a disambiguation
                return Ok
        }
    }

    if need_disambiguation {
        // we couldn't find any. we cannot say anything about those arms.
        // reject conservatively.
        Error
    } else {
        // either A is strictly included in B and the other inputs that match B
        // will never match A, or B is included in or equal to A, which means
        // it's unreachable. this is not our problem. accept.
        Unsure
    }
}
```

### Sequence repetitions

Sequence repetitions (`$(...)*` or `$(...)+`) must be handled separately. Even when their FIRST set are disjoints, we must still handle the case of inputs not matching the token trees inside the sequence: in those cases, we must continue the analysis with what comes *after* the sequence repetition.

We can handle this conservatively by just unrolling the sequence and recursively analysing all the possibilities. If we must test `$(a)* a'` against `b`, we must test `a $(a)* a'` against b, then `a'` against `b`. (Reciprocally, `a` against `$(b)* b'` will result in `a` against `b $(b)* b'*` and `a` against `b'`). Each of the newly generated pairs of matchers may in turn generate other items to check if it comes back at the sequence repetition.

We add the following case to our algorithm, before checking if the matchers are the same (even if they are, we must still run the checks because what comes after might be different):

```rust
if we already visited the pair of matchers (a, b) {
    return Unsure
}

add (a, b) to the visited pairs of matchers;

match a, b {
    // assuming ++ is the concatenation of matchers
    // and ma contains the rest of the matcher, with the
    // current element skipped.
    // we also assume here that the `&&` operator takes
    // the minimum value with Error < Unsure < Ok.
    
    Seq(s1), Seq(s2) => {
        check_matchers(seq1.tts ++ seq1.delim ++ a ++ ma, b ++ mb) &&
        check_matchers(a ++ ma, seq2.tts ++ seq2.delim ++ b ++ mb) &&
        check_matchers(ma, b ++ mb) &&
        check_matchers(a ++ ma, mb)
    }
    
    Seq(seq), _ => {    
        check_matchers(seq.tts ++ seq.delim ++ a ++ ma, b ++ mb) &&
        check_matchers(ma, b ++ mb)
    }
    
    _, Seq(seq) => {
        check_matchers(a ++ ma, seq.tts ++ seq.delim ++ b ++ mb) &&
        check_matchers(ta ++ ma, mb)
    }
    
    // note that when recursive checks return Unsure then we
    // must also take into account `need_disambiguation`
}
```

(Note that I intentionnaly left out the `+` sequence repetitions, since `$(tts)tok+` is trivially equivalent to `tt tok $(tt)tok*` for our concern.)

### Landing

Since this will generate a lot of false positives, and that even legitimate
errors will probably be hard to understand and to fix, we will first land it as a warning. We could then periodically use Crater to gather breakage statistics to see how fast people are adapting and decide when to turn it into an error (if we ever do, the other option being to wait for `macro_rules!` to be replaced by
something else).

An opt-out attribute, `#[unsafe_macro]` will also be added to ignore the future-proofing analysis on a specific macro.

# Drawbacks
[drawbacks]: #drawbacks

* This analysis is potentially slow. First, it's quadratic with the number of arms in a  multiple-arms macros since we compare them two-by-two. When encountering a sequence repetition, it becomes exponential in the number of tokens in the  other matcher (until the end of the matcher) because of the recusrive unrolling. However, in practice, it looks like for most macros it does not represent a lot of tokens, so it stays reasonable.

* It's very conservative, prefectly valid macros will be rejected. The first  results tell that approximately a third of the regressions are false positives.

# Alternatives
[alternatives]: #alternatives

* Not do this and either:      
  * Keep the status quo, that is, keep macro-rules not future proof.
  * Search another solution, based on the knowledge of grammar productions by the algorithm (see thoughts [here](https://internals.rust-lang.org/t/on-macros-future-proofing-follow-sets-and-related-stuff/3416)).     
  
* Only add this after investigating the FOLLOW set trick (allows to reduce the numebr of false positives by expanding the concatenation property with knowledge of the FOLLOW sets, explained in the second bullet of ‶unresolved questions″).

* Another interesting possibility to reduce the number of regressions is to look to the `$(foo:tt)*` pattern. Similar to the `_` pattern in `match` and `let`, this pattern effectively matches all the possible inputs. Therefore, when it's used in the second arm, it works as a “catch-all” and seems to be used like that in most the cases when we can encouter it. For example, take the following macro (taken from the `log` crate):
  
  ```rust
  macro_rules! foo(
      (target: $e:expr, $(tt:tt)*) => ...
      ($(tt:tt)*) => ...
  ```
  
  If `expr` ever starting matching new input, then macro invocations containing the new form will indeed switch from the second arm to the first. So this macro is legitimately not future-proof and reported as such by the algorithm. But clearly, it's the intention here that the second arm acts as a “catch-all” rule and that if new forms are added to `expr`, then they should indeed start matching the first arm.
  
  We could thus add a builtin knowledge of this pattern to the algorithm, to specially recognize it and accept a macro such as the above. The main problem with that is that it moves the responsibility of ensuring that the macro is future-proof from the compiler to the user who must know check that such behaviour is indeed its intention.
  
  This in turns brings a new unresolved question which is: where to recognize this pattern? Do we only recognize the pattern when a whole arm solely consists of a catch-all matcher, such as the `foo` macro above? Or can we also recognize it in the middle of an arm, provided that what comes before is accepted, for example:
  
  ```rust
  macro_rules! foo(
      (foo bar baz, target: $e:expr, $(tt:tt)*) => ...
      (foo bar baz, $(tt:tt)*) => ...
  )
  ```
  
  This looks fine but probably has a few corner cases. Same question regarding
  single-TT catch-all, for example, can this:
  
  ```rust
  macro_rules! foo(
      (foo bar ( a lot of complex matchers ) baz) => ...
      (foo bar $(tt:tt) baz ) => ...
  )
  ```
  
  be accepted too, considering that there is a ”local catch-all” of what's inside  the parenthesized delimited-TT matcher in the first arm (and provided that what comes before and after is unambiguous). This has a few corner cases, then again when combined with sequence repetitions, but I think they're easy to address. This too needs a bit of thinking. It can still be added to the analysis afterwards, though.

  Another option is to not specially-recognize `$tt` but introduce a new matcher explicitly designed for this goal, such as `$all`. This is exactly the same, but cleaner maybe, and this way we'll be sure that the behaviour we will give it will be intended in every use case. We can't be sure of that with `$(tt)*`. The problem, though, is that while it gives a simple easy-fix for existing use cases, it will still cause breakage and require user intervention.

* Land it as an opt-in analysis pass that must be invoked explicitly some `-option`, or using some `#[warn]` or `#[deny]` (locally overridable). This however reduces greatly the impact of such an analysis since most users probably won't invoke it at all.
  
* Land it as an error. This will cause a lot of breakage.

# Unresolved questions
[unresolved]: #unresolved-questions

* In order to turn this RFC into a true RFC, we must address the question of what we do with the FIRST sets. Currently (as in, in my experimental implementation), the FIRST sets are defined to be the exact sets of tokens that can start a given non-terminal, as computed from the grammar. So for any token that cannot start right now a non-terminal, it is considered that it will never be allowed to start this NT.
   
  It means we cannot restrict the FIRST sets, only relax them by saying “maybe we will reserve the possibility of expanding the language in the future by saying that this token will maybe allowed to start such NT”. This will add more regressions, though few I think. The T vs NT pattern doesn't represent a huge share of the regressions.
  
  Of course we could also leave the things as they are, but this means that we cannot add new prefix operators in the future, for example. It's not up to me to decide wether this is a restriction worth considering or not.
  
  That being said, it should be possible to restrict a bit the FIRST sets: currently, whenever a NT can start by an identifier, then it's also considered as being allowed to start with any keyword. For example, while `expr` can start with any arbitrary ident, maybe we could state that it will never be allowed to start by some reserved keyword, say `mut`. This means that the matchers `mut $e:expr` and `$e:expr` would be considered future-proof. However, the current Crater test results show that only half a dozen crates could benefit from this.
  
* Investigate the trick imagined by @nmatsakis [here](https://internals.rust-lang.org/t/on-macros-future-proofing-follow-sets-and-related-stuff/3416/24) to be able to recover after a “T vs NT” or “NT vs T” pattern, and, presumably, eliminate all remaining false positives. However, I still need to convince myself that there are no tricky cases (especially regarding sequence repetitions) that should be taken care of if we do not want this trick to introduce unsoundness in the algorithm.

  Formally speaking, it's like considering that `$t:nt tok1` and `... tok2` can be considered safe prefixes for the concatenation property if `tok2 ∈ FOLLOW(nt)`.

  Also, this trick relies on specific parser semantics that should be implemented. I do not think such changes in the parser implementation would be breaking but then again, this needs a bit more investigation. However, we should keep in mind that such an analysis can be added later without adding extra breakage. Adding it later will just cause code to be first broken then considered valid again. It's a good thing if we can avoid that, but it's not that bad if we clearly state that this will be implemented at some point, so that the users can, with caution, use `#[unsafe_macro]` on concerned macros.

* What to do in the future. I think @nrc wants to implement a new system of macros-by-example from scratch to eventually replace (and deprecate) `macro_rules!`. This new system probably won't actually suffer the same problems as `macro_rules!` if it still features NT matchers. We should directly embed this analysis in it right from the beginning, as an error, to avoid more future breakage. Maybe we can design this new system from the start to make the analysis more effective, for example by using the greedy parser semantics for the FOLLOW set trick, or by implementing the ‶catch-all″ matcher.

* Single-arm macros. As shown [here](https://github.com/rust-lang/rust/pull/33840#issuecomment-224869252), single-arm macros that feature sequence repetitions cannot benefit of backtracking unless we check the same property on them at any point where two portions of the matcher could start matching the input ‶in parallel″. The transposition of the algorithm I outliend in this RFC to this particular issue should be positively trivial, but I do not have any statistics on the additional amount on breakage, which would be necessary to decide if adding backtracking is worth it.

