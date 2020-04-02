- Feature Name: `rewind_iterator`
- Start Date: 2020-04-02
- RFC PR: [rust-lang/rfcs#2896](https://github.com/rust-lang/rfcs/pull/2896)
<!--
- Rust Issue: [rust-lang/rust#0000](https://github.com/rust-lang/rust/issues/0000)
-->

# Summary
[summary]: #summary

Add the trait `RewindIterator` to `std::iter` for iterators that can be
rewinded.

# Motivation
[motivation]: #motivation

Today, we have [`std::iter::Peekable`], which allows a single-element
look-ahead in iterators. `Peekable` is useful, but it must buffer one
element of the iterator, since it has now knowledge of the underlying
iterator implementation. It also only allows peeking at a single
element, and only provides a reference to it at that. This means that if
you peek ahead in an iterator that provides mutable access, you cannot
mutate the peeked-at element.

Some iterators can provide much more robust navigation options for the
iterator, and can do so efficiently. The most immediately useful of
these is the ability to _rewind_ the iterator: effectively undoing the
last call to `Iterator::next`. An iterator that supports rewinding
allows the user to move both forwards _and backwards_ through the
underlying element stream. This in turn allows peeking all the way to
the end of the iterator if need be, and allows iterating over the same
set of elements more than once.

Beyond peeking, the most obvious use-cases for rewinding is for
iterators where consecutive elements are related in some fashion. For
example, for a `BTreeMap<(Game, Team, Player), usize>`, the user may
wish to compute aggregate statistics for each team for a game, but then
_also_ compute something per player that depends on the team statistics.
Currently, developers are forced to either do multiple lookups into the
map, or iterate over the whole map multiple times. With a rewinding
iterator, neither is necessary; the developer can simply rewind the
iterator after computing the team statistics, and then iterate again to
enumerate the players.

  [`std::iter::Peekable`]: https://doc.rust-lang.org/std/iter/struct.Peekable.html

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

The `RewindIterator` trait provides a mechanism for moving an iterator
_backwards_, effectively undoing a call to `Iterator::next`. If an
iterator implements `RewindIterator`, you can call `previous` on it to
get back to the _previous_ element of the iterator. You can also call
`rewind` to rewind the iterator all the way to the beginning.

This allows you to peek far ahead in an iterator:

```rust
let people = ["Alice", "Bob", "Carol"];
let mut iter = map.iter();
while let Some(name) = iter.next() {
    println!("{} is followed by:", name);
    let mut following = 0;
    while let Some(name) = iter.next() {
        println!(" - {}", name);
	following += 1;
    }
    for _ in 0..following {
        let _ = iter.previous();
    }
}
```

Which will print:
```
Alice is followed by:
 - Bob
 - Carol
Bob is followed by:
 - Carol
Carol is followed by:
```

Note that rewiding an iterator is different from reversing it (with
[`Iterator::rev`]) and then iterating over it. A call to `previous()`
will effectively _undo_ call to `next()`, rather than yielding the next
element from the back of the iterator.

You can also use `RewindIterator` to iterate over a part of the iterator
multiple times. This comes in handy when your iterator yields related
elements consecutively, and you want to operate on related items as a
group _and_ individually. For example, the code below computes the
player on the winning team who scored the greatest fraction of points in
each game. It does not need to buffer iterator elements, nor does it
allocate to hold intermediate results.

```rust
fn print_mvps(games: BTreeMap<(Game, TeamId, PlayerId), PlayerStats>) {
    let mut games = games.iter();
    while let Some(&(ref game, team, _), stats) = games.next() {
        let mut players = 0;
	let mut total_score = stats.score;
        let mut team1 = (team, stats.score);
        let mut team2 = None;
        // Compute the score of each team in this game
	while let Some(&(ref game_, team, _), stats) = games.next() {
	    players += 1;
	    if game_ != game {
	        // We went too far!
	        let _ = games.previous();
	        break;
	    }
	    if team == team1 {
	        team1.1 += stats.score;
	    } else if let Some(team2) = team2 {
	        team2.1 += stats.score;
	    } else {
	        team2 = Some((team, stats.score));
	    }
	    total_score += stats.score;
	}
	// Figure out who won
	let team2 = team2.unwrap_or(team1); // walkover
	let winner = match team1.1.cmp(&team2.1) {
	    Ordering::Greater => Some(team1.0),
	    Ordering::Lesser => Some(team2.0),
	    Ordering::Equal => None
	};
	// Find the player on the winning team with the highest score.
	// Rewind the iterator to the start of the game.
	for _ in 0..players { games.previous() }
	let mvp = None;
	while let Some(&(ref game_, team, player), stats) = games.next() {
	    if game_ != game {
	        let _ = games.previous();
	        break;
	    }
	    if winner.map(|t| t == team).unwrap_or(true) {
	        // Player is on the winning team!
	        let score = stats.score as f64 / total_score as f64;
		if let Some((p, s)) = mvp {
		    if score > *s {
		        // Player has the highest score!
		        *p = player;
			*s = score;
		    }
		} else {
		    mvp = Some((player, score));
		}
	    }
	}
	let (player, score) = mvp.expect("winning team had no players");
	println!("MVP of game {} was {} with {:.1}% of points", game, player, score * 100.0);
    }
}
```

  [`Iterator::rev`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.rev

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

This RFC proposes adding the following trait to `std::iter`:

```rust
trait RewindIterator: Iterator {
    fn previous(&mut self) -> Option<Self::Item>;
    fn nth_previous(&mut self, n: usize) -> Option<Self::Item> {
        for _ in 0..(n-1) {
	    self.previous()?;
	}
	self.previous()
    }
    fn rewind(&mut self) {
        while self.previous().is_some() {}
    }
}
```

Initial implementations should be added for the reference and mutable
reference iterators for slices, `Vec`, `HashMap`, `Range`, and
`BTreeMap`. For slices and `Vec`, the implementation is straightforward.
For `BTreeMap`, the `Range` and `RangeMut` types must be extended with a
`prev_unchecked` that calls `next_back_unchecked` on `self.front`. A
guard to make sure the user does not go beyond the start of the iterator
is also needed. For `HashMap`, I _believe_ it should be possible to
write the opposite of `<RawIterRange as Iterator>::next`, though do not
have the experience with `hashbrown` to say for sure.

`nth_previous` and `rewind` are provided for convenience. They have sane
default implementations, so those are provided. Some iterator types can
efficiently move iterators by larger steps (e.g., `[T]`, `Vec`, and
`HashMap` can move the full increment at once, and `BTreeMap` can move
in increments of the `B`), and may override these methods.

# Drawbacks
[drawbacks]: #drawbacks

As with any proposal that adds a trait to the standard library, this
expands the size of the standard library interface. None beyond that.

# Rationale and alternatives
[rationale-and-alternatives]: #rationale-and-alternatives

Setting aside obvious bikeshedding questions around naming (like
`Bidirectional`), here are some other considerations:

Many iterators implement `Clone`, which essentially provides something
equivalent to `RewindIterator`. However, not all iterators can implement
`Clone`. Primary among these are all iterators that mutably borrow the
underlying data structure, since they cannot be cloned. One could
imagine something like a `SnapshotIterator`, which allows you to get a
"temporary" iterator that rewinds the source iterator automatically when
it is dropped, but this would probably require generic associated types
to let us tie the returned snapshot iterator to the mutable borrow of
the original iterator.

Iterator types that support rewinding could instead just provide
inherent methods that allow you to move the iterator in interesting ways
(such as backwards). Some iterators, like those of `BTreeMap`, may even
provide ways for the iterator to "jump", such as to the next element in
a given range. This RFC does not prevent such designs, and merely
proposes that the relatively common ability to move an iterator
backwards be shared such that developers can take advantage of types
with this feature in a generic way.

The trait could be modified such that `previous` did not _return_ the
previous element, but instead _just_ moved the iterator back. The return
value of `previous` is already a little strange, as it is not
immediately obvious whether the element you get is the one that `next`
just yielded, or the one _before_ that. Specifically, should this assert
succeed or fail?

```rust
let next = iter.next();
let prev = iter.previous();
assert_eq!(next, prev);
```

I opted to keep `previous` as analogous to `next` as possible; the exact
behavior should ultimately be documented anyway.

# Prior art
[prior-art]: #prior-art

I am not aware of prior art in the Rust space for "rewindable"
iterators. In Java, this type of iterator is called a [`ListIterator`],
and it "allows the programmer to traverse the list in either direction".
Interestingly enough, in Java, the `ListIterator` is documented like
this:

> A `ListIterator` has no current element; its cursor position always
> lies between the element that would be returned by a call to
> `previous()` and the element that would be returned by a call to
> `next()`. An iterator for a list of length `n` has `n+1` possible
> cursor positions, as illustrated by the carets (^) below:
>
>                       Element(0)   Element(1)   Element(2)   ... Element(n-1)
>  cursor positions:  ^            ^            ^            ^                  ^

This model provide a clue as to what `RewindIterator::previous` should
return.

It's not clear why Java opted to make this iterator's name specifically
refer to "list", rather than a more generic name.

In C++, iterators that can be moved backwards are "tagged" as
[Bidirectional] to indicate that they support `--` in addition to `++`.
They also have the notion of a "[random access iterator]", which may be
in our future too.

Python and Ruby do not appear to have bidirectional iterators.

  [`ListIterator`]: https://docs.oracle.com/javase/8/docs/api/java/util/ListIterator.html
  [Bidirectional]: https://www.cplusplus.com/reference/iterator/BidirectionalIterator/
  [random access iterator]: https://www.cplusplus.com/reference/iterator/RandomAccessIterator/

# Unresolved questions
[unresolved-questions]: #unresolved-questions

Should `RewindIterator::previous` return the element that the last
`Iterator::next` yielded, or the element _before_ that? Essentially,
should it get the _current_ element, then move back, or should it move
back, and _then_ return the current element? In code, should this
assertion fail or succeed?

```rust
let next = iter.next();
let prev = iter.previous();
assert_eq!(next, prev);
```

A big part of answering this question is probably going to be which
option can be efficiently implemented for `BTreeMap` and `HashMap`.

Should `RewindIterator` interact with `DoubleEndedIterator` somehow, to
let a user undo a call to `next_back`. Or should that be a separate
trait? How many iterators do we expect will be able to undo a `next`,
but not a `next_back`?

# Future possibilities
[future-possibilities]: #future-possibilities

We may want to go full C++ and also provide a `RandomAccessIterator`,
but that's for another day and another RFC.

We may want to add more provided methods on `RewindIterator`.

We may want to implement `RewindIterator` on other data structures.
