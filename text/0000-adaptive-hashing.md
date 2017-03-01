- Feature Name: adaptive_hashing
- Start Date: 2016-11-20
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Implement adaptive hashing for HashMap. Initialize hash maps using the fastest practical hash
function, and fall back to SipHash in case of a potential DoS attack.

# Motivation

Hash DoS is an example of a DoS attack. The goal of DoS attacks is a denial of service. Consider
creating a HashMap from a given list of keys:

```rust
    fn make_map(keys: Vec<usize>) -> HashMap<usize> {
        let mut map = HashMap::new();
        for key in keys {
            map.insert(key, 0);
        }
        map
    }
```

Let's suppose that the `keys` array is an input that comes from the outside world. A simple case
of DoS happens when a server receives a HTTP request with thousands of deliberately chosen
parameters. Processing just one such request can take minutes.

The `keys`  array is manipulated to get the slowest possible run time. In the worst case, all keys
hash to the same bucket, so we no longer benefit from hashing. Each iteration of the loop in the
example code takes O(n) time. The entire function executes in O(n**2) time. The hash map behaves
like a typical dynamic array. We might as well write:

```rust
    fn make_map(keys: Vec<usize>) -> HashMap<usize> {
        let mut map = vec![];
        for key in keys {
            if let Some(index) = map.position(|(k, _)| k == key) {
                map[index] = 0;
            } else {
                map.push(0);
            }
        }
    }
```

We are only considering slow insertions, because we don’t need to worry about lookup. The cost of
inserting an element includes the cost of searching for that element. Immediately after inserting an
element, the cost of looking it up will be equal or smaller. Later, after some number of unrelated
insertions, the cost of looking up that element will still be limited by some threshold.

To prevent all Hash DoS attacks, we need to make sure that HashMap is protected.  The standard
library's HashMap currently uses SipHash-1-3 for all its lookups to protect from Hash DoS.
Unfortunately, this comes with a tradeoff. Some people believe SipHash is too slow. They consider
non-ideal performance of HashMap for small keys as its main drawback. Others see the use of SipHash
as a good solution to the tradeoff between security and speed.

Is SipHash really slow, and why? We can simply count the number of instructions it performs.
SipHash’s round involves 14 64-bit operations. SipHash-1-3 runs one round for each 8 bytes of input,
and three rounds for finalization, so it involves 16 operations for each 8 bytes of input, and 42
operations for finalization. Hashing an input of 8 bytes needs 58 operations. However, out-of-order
execution allows more than one operation per cycle on modern CPUs. Also, SipHash uses simple
operations, i.e. addition, bitwise rotation and XOR. Still, we can see that SipHash is relatively
slow for small values. Ideally, hashing an integer should take only 7 instructions.

Several dynamic programming languages use SipHash for their hash tables. However, Rust is a systems
programming language. The slowdown from hashing is more noticeable than in other languages.

Perl uses a mechanism similar to adaptive hashing for its dictionaries implemented with chaining.
Java uses chaining and changes a linked list to a binary tree when its length exceeds some
threshold.

Fortunately, Robin Hood hashing can be easily extended with adaptive hashing.

# Detailed design
## The algorithm for adaptive hashing

A HashMap with adaptive hashing has two states. One state is called “fast mode” and the other is
“safe mode”. The fast mode is the inital state for HashMaps with keys of a type that can be hashed
in one shot. Otherwise, a HashMap with complex keys is always in safe mode.  We switch to the safe
mode when the following conditions are met:

- an inserted entry's displacement >= 128, or the number of entries displaced by an inserted
  entry >= 1500
- the load of the map is smaller than 20%
- the map is in the fast mode

The second condition reduces the odds of switching to safe hashing. The chance that the first
condition is satisfied is tiny, and the chance that both are satisfied at the same time is
negligible. Moreover, we add a flag to the map. The flag delays displacement reduction until the
next insertion to make code simpler. Otherwise, rebuilding the map would invalidate our entry.
The pseudocode for a function that replaces `insert` is:

```
fn safeguarded_insert(map, key):
  entry = insert(map, key)
  if the entry's displacement >= 128 or the number of entries displaced by entry >= 1500:
    set the flag for reducing displacement
  return entry
```

Before the next insertion operation, the state must be checked. Conveniently, the `reserve` method
is always called before insertion and entry search, so we add the following code to `reserve`:

```
fn reserve(map, ...):
  if the flag for reducing displacement is set and the map uses fast hashing:
    if the load of the map is higher than 20%:
      grow the map
    else:
      switch the map's hash state to safe hashing
      rebuild the map
    clear the flag for reducing displacement
  // ...
```

Here’s a state diagram for HashMap with adaptive hashing. The dashed edge means the state change is
very unlikely, and the dotted edge means the state change is enormously unlikely.

<img width="800" src="https://cdn.rawgit.com/pczarn/code/d62cd067ca84ff049ef196aa1b7773d67b4189d4/rust/robinhood/adaptive.svg">

## Load factor

We decrease the load factor of `HashMap` from 0.909 to 0.833.

## Choosing constants

The thresholds of 128 and 1500 are chosen to minimize the chance of exceeding them. In particular,
we want that chance to be less than 10^-8 with a load of 90% and less than 10^-30 with a load of
20%. For displacement, the smallest k that fits our needs is 90, so we round that up to 128. For the
number of forward-shifted buckets, we choose k=1500. Keep in mind that the run length is a sum of
the displacement and the number of forward-shifted buckets, so its threshold is 128+1500=1628. We
can allow probability of exceeding our thresholds that is a bit worse than desirable.

### Lookup cost

```
At load factor 0.909
Pr{lookup cost >= 100} = 1.0e-9
Pr{lookup cost >= 128} = 3.1e-12
Pr{lookup cost >= 150} = 3.3e-14
```

```
At load factor 0.833
Pr{lookup cost >= 100} = 4.1e-16
Pr{lookup cost >= 128} = 2.0e-20
Pr{lookup cost >= 150} = 8.0e-24
```

```
At load factor 0.2
Pr{lookup cost >= 100} = 6.2e-116
```

### Forward shift cost

At load factor near the current limit of 0.909, the cost of forward shift is too high to allow it.

```
At load factor 0.833
Pr{forward shift cost >= 1200} = 2.6e-10
Pr{forward shift cost >= 1500} = 4.1e-12
Pr{forward shift cost >= 1800} = 6.4e-14
```

## Choosing hash functions

For hashing integers, the best choice is a mixer similar to the one used in SipHash’s finalizer. For
strings and slices of integers, we will use FarmHash. (The Hasher trait must allow one-shot hashing
for FarmHash.) Using any other key type means your HashMap will do safe hashing.

# Consequences
## For the hashing API

This RFC does not propose any public-facing changes to the hashing infrastructure.

## For the performance of Rust programs

The impact is minimal on programs that rarely use HashMaps. The load factor’s new value is well
within the reasonable range. The increase in binary size should be small. For programs that spend a
large portion of their run time using HashMap with primitive keys, the speedup should be noticeable.

On 32-bit platforms, the benefit of using a 32-bit hash function instead of SipHash is higher,
because each SipHash’s round involves 30 32-bit operations.

## For the HashMap API

One day, we may want to hash HashMaps. The hashing infrastructure can be changed to allow it. The
implementation of Hash for HashMap can hash the hashes stored in the map, rather than hash the
contents of each key in the map. However, adaptive hashing makes it harder to write a correct and
performant implementation of Hash for HashMap. If two HashMaps (that can be compared) have equal
values, they must hash to the same integer. However, with adaptive hashing, HashMap can switch to
the safe mode, which means it no longer stores the same hashes as other HashMaps that remain in the
fast mode. The only way to handle the situation for the safe mode is to rehash all keys as if the
HashMap were in the fast mode, which may take a significant time.

## For the order of iteration

Currently, HashMaps have nondeterministic order of iteration by default. This is seen as a good
thing, because testing will catch code that relies on a specific iteration order. Otherwise,
programmers might not know that their programs only work with a fixed iteration order. To keep
nondeterministic order, SipHash’s thread-local seed may be used for all hashers.

# Drawbacks

More complex code needs to be maintained. There’s a risk of having a bug in the algorithm or in the
code.

# Alternatives

- We can reject adaptive hashing. SipHash-1-3 may be fast enough.
- We can restrict adaptive hashing to integer keys. With this limitation, we don't need Farmhash in
  the standard library.
- We can use some other fast one-shot hasher instead of Farmhash.
- We can use an additional fast hash function for fast streaming hashing. The improvement would
  be small.
- We can set FarmHash's seed to a random value for nondeterminism.
- When a map is emptied, its hash function does not matter anymore. As a special case, we can detect
  operations that clear maps in safe mode, and reset them back to fast mode.
- We can let users declare their types as one-shot hashable. The following public trait may allow
  such one-shot hashing.

```rust
#[cfg(not(target_pointer_width = "64"))]
type ShortHash = u32;
#[cfg(target_pointer_width = "64")]
type ShortHash = u64;

trait OneshotHashable {
  fn hash(&self) -> ShortHash;
}
```

# Unresolved questions

Is there any hasher that is faster than Farmhash?

Are the chosen thresholds reasonably low?

# Appendices

## Image for the lookup cost chart

<img width="600" src="https://cdn.rawgit.com/pczarn/code/d62cd067ca84ff049ef196aa1b7773d67b4189d4/rust/robinhood/lookup_cost.png">

## Image for the forward shift cost chart

<img width="600" src="https://cdn.rawgit.com/pczarn/code/def92e19ae60b599e9620afa1bdcad1c36e6e982/rust/robinhood/extrapolated_insertion_cost_4.png">
