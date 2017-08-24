- Feature Name: from-iterator-tuple
- Start Date: 2017-08-26
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary
[summary]: #summary

Simplify conversion of `Vec<(_, _)>` and `HashMap<_, _>` to `(Vec<_>, Vec<_>)`.

# Motivation
[motivation]: #motivation

Currently, there is no way to collect iterator over tuples into two independent collections. Having such `FromIterator` would greatly improve programming ergonomics.

```rust
struct TupleShunt<'a, I, V> {
	iter: I,
	leftovers: &'a mut Vec<V>,
}

impl<'a, K, V, I> Iterator for TupleShunt<'a, I, V>
	where
		I: Iterator<Item = (K, V)>
{
	type Item = K;

	fn next(&mut self) -> Option<Self::Item> {
		match self.iter.next() {
			Some((k, v)) => {
				leftovers.push(v);
				Some(k)
			},
			None => None
		}
	}
}

impl<K, V, F, S> FromIterator<(K, V)> for (F, S) 
	where
		F: FromIterator<K>,
		S: FromIterator<V>
{
	fn from_iter<T: IntoIterator<(K, V)>>(iter: T) -> (F, S) {
		let mut leftovers = Vec::new();
		let mut shunt = TupleShunt {
			iter: iter.into_iter(),
			&mut leftovers,
		};
		let f = F::from_iter(shunt);
		let s = S::from_iter(leftovers);
		(f, s)
	}
}
```

# Guide-level explanation
[guide-level-explanation]: #guide-level-explanation

No new concepts are introduced in this rfc. It's just ergonimics.

# Reference-level explanation
[reference-level-explanation]: #reference-level-explanation

```rust
fn fetch(x: u32) -> (Foo, Bar);

fn foo(f: Foo) -> ProcessedFoo;

fn bar(b: Bar) -> ProcessedBar;

fn finished_processing_foo(f: Vec<ProcessedFoo>);

fn finished_processing_bar(b: Vec<ProcessedBar>);

fn old_usage() {
	let foobar: Vec<_> = (0..10)
		.into_iter()
		.map(fetch)
		.map(|(f, b)| (foo(f), bar(b)))
		.collect();

	let mut processed_foo = Vec::with_capacity(foobar.len());
	let mut processed_bar = Vec::with_capacity(foobar.len());

	for (f, b) in foobar {
		processed_foo.push(f);
		processed_bar.push(b);
	}

	finish_processing_foo(processed_foo);
	finish_processing_bar(processed_foo);
}

fn new_usage() {
	let (processed_foo, processed_bar) = (0..10)
		.into_iter()
		.map(fetch)
		.map(|(f, b)| (foo(f), bar(b)))
		.collect();

	finish_processing_foo(processed_foo);
	finish_processing_bar(processed_bar);
}

```

As you can see `new_usage` is 6 lines shorter than `old_usage`. Not only it's shorter, but it also more readable and allocates less memory on the heap than `old_usage`.

`old_usage` allocates at least `10 * (size_of(Foo) + size_of(Bar)) * 2`

`new_usage` allocates at least `10 * (size_of(Foo) + size_of(Bar)) + 10 * size_of(Foo)`

# Drawbacks
[drawbacks]: #drawbacks

-

# Rationale and Alternatives
[alternatives]: #alternatives

# Unresolved questions
[unresolved]: #unresolved-questions

None
