- Start Date: 2014-10-12
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

The regions used for lifetime inference and borrow checking should be general
single-entry / multiple-exit regions, instead of being limited to the current
situation of lexical scopes, or even more general single-entry / single-exit
regions.

# Motivation

There are three classes of program fragments that are currently rejected by the
borrow checker that really seem like they should be accepted.

1. The borrow checker ignores liveness of borrows, so that a borrow is valid for
the entirety of a lexical scope, even after its last use. This even occurs in
straight-line code For example, a program like this is rejected:

    ```rust
    let mut i = 0i;
    let p = &mut i;
    *p = 1;
    let q = &mut i;
    *q = 2;
    ```

 The borrow checker should be able to realize that each of these borrows is no
longer valid at the point of the next borrow. The workaround for this is to give
each borrow its own lexical scope, e.g.

    ```rust
    let mut i = 0i;
    {
        let p = &mut i;
        *p = 1;
    }
    {
        let q = &mut i;
        *q = 2;
    }
    ```

 This is unnecesarily verbose and difficult to explain to new programmers. It
also doesn't handle all cases involving mutable borrows of multiple locations.

2. The borrow checker can't refine borrow information based on the branches of
a pattern match. The canonical example is a 'find-or-insert' pattern:

    ```rust
    match map.find(&key) {
        Some(...) => { ... }
        None => {
            map.insert(key, new_value);
        }
    }
    ```

 In an `Option<&T>` the `&T` only appears in the the `Some` constructor, so the
borrow shouldn't need to be considered in the `None` branch. The workaround for
this is usually to do a clone/copy and two separate lookups'. This is more
verbose, doesn't work in all cases, and is potentially a large performance
penalty.

3. Borrows in rvalues of the same overall expression often overlap when it
doesn't seem that they have to. For example, if `a` has methods `foo`, `bar`
and `baz` that all take `&mut self` parameters, but the latter two have no
lifetimes in output position, then an expression such as

    ```rust
    a.foo(a.bar(), a.baz())
    ```

 seems like it should be valid. The workaround is to instead write

    ```rust
    let bar = a.bar();
    let baz = a.baz();
    a.foo(bar, baz);
    ```

 All this is doing is making the evaluation of temporaries explicit, so it
shouldn't affect whether the program is accepted.

While all of these errors arise in the borrow checker, they are actually caused
by a deficiency in Rust's lifetime system. Currently, Rust models lifetimes as
lexical scopes, and borrows must last for a specific lifetime.

The way to fix these problems is to refine the notion of lifetime. The obvious
first step would be to generalize from lexical scopes to arbitrary single-entry
/ single-exit regions, but this would only fix the first and third of the
problems mentioned above. To fix the second problem, which is arguably the most
frustrating in actual use because workarounds are difficult or nonexistent, the
notion of lifetimes in Rust needs to be refined to include multiple exit points.

# Background

Unfortunately, there is no obvious candidate for a notion of a single-entry /
multiple-exit region to use from the literature. We assume that a Rust function
is viewed as a control-flow graph, with a single entry point and a single return
point (multiple return points can be handled with additional edges). This
includes the expansion of temporary rvalue evaluation. This is already done for
other reasons in `rustc`.

It's easier to first think about the single-entry / single-exit case first. If R
is a set of vertices in a CFG, the most obvious definition is that R is a
single-entry / single-exit region if the following conditions hold:

1. There is a vertex Entry in R such that for all vertices V in R, every path
from the function entry to V contains Entry.

2. There is a vertex Exit in R such that for all vertices V in R, every path
from V to the function exit contains Exit.

If we were to incorporate this definition of regions into Rust lifetimes, then
the following program fragment would be accepted:

```rust
let mut a = Some(0u);
let p = a.as_ref().unwrap();
loop {
    println!("{}", *p);
    a = None;
    if ... {
        break;
    }
}
```

The lifetime of the borrow associated with `p` would be a single-entry /
single-exit region with Entry at the call to `as_ref` and Exit at the last
use of `p`. Therefore the borrow checker would find nothing wrong with the
mutation right below the last use, even though this leads to memory unsafety
at runtime.

The problem here is that the region's Entry and Exit aren't always matched, i.e.
there are paths from the function entry to the function exit that go through
Entry a different number of times than they go through Exit. If we add this
condition, then we arrive at the standard definition of a SESE region in the
compiler literature. This is equivalent to saying that Entry and Exit belong
to the same cycles in the CFG.

Adopting SESE regions for Rust lifetimes would be sound. Moreover, SESE regions
form a lattice, where meet is intersection and join is the least region
containing the union of two regions. Since a single vertex forms its own region,
this also covers extending a region to contain a point. Since these are the
operations that the Rust type checker needs to infer lifetimes, it would be
possible to compute lifetimes in this manner without much difficulty.

It is possible to extend this notion of a region to have multiple exit points
instead of just one, but the details of the definition are a bit subtle (e.g.
we would have to track exit edges rather than exit vertices), and unfortunately
it becomes very difficult to compute the lattice operations given a
representation of two regions in terms of entry and exit points. The problem is
that when shrinking or extending a region, there is no easy way to compute the
new exit edges.

We don't actually need the exit edges of regions in the Rust compiler until
after type checking, when the dataflow analysis used for borrow checking inserts
borrow kill flags on exits from regions. In the type checker itself, we could
use any representation we want. Because of the difficulty of finding an
efficient sparse representation in terms of entry and exit points, this RFC
proposes a definition of single-entry / multiple-exit region that doesn't use
exit points at all.

# Detailed Design

This definition is inspired somewhat by the work done on liveness analysis and
register allocation of programs in [static single assignment form](http://en.wikipedia.org/wiki/Static_single_assignment_form).

If R is a set of vertices in the CFG, we say that R is a *single-entry /
multiple-exit region* if there is a vertex Entry in R such that if V is in R and
W is not in R, then every path from W to V contains Entry. If W is the CFG entry
point, then this condition specializes to the first condition in the definition
of a SESE region above. It isn't hard to see that if such a vertex Entry exists
then it must be unique, by considering paths from the CFG entry point.

This condition ensures that in the case of a program fragment like

```rust
let mut i = 0i;
let p = &mut i;
if ... {
    (use of p)
} else {
    (no use of p)
}
drop(p);
```

the region corresponding to the borrow in `p` includes both branches of the
`if`.

This definition is difficult to work with directly, but it is possible to
reformulate it to make it more palatable. We say that a vertex V *dominates* a
vertex W if every path from the CFG entry to W goes through V, and that it
strictly dominates W if V is distinct from W. By induction, every vertex V other
than the CFG entry has an *immediate dominator* that does not dominate any other
dominator of V. The immediate dominance relation forms a tree, and it is
possible to efficiently compute the dominator tree from the CFG. For more
details, see [the Wikipedia page](http://en.wikipedia.org/wiki/Dominator_%28graph_theory%29)
or any textbook on compilers.

We can then reformulate the definition above to say that R is a *single-entry /
multiple-exit region* if the following conditions hold:

1. R is a subtree (i.e. a connected subgraph) of the dominator tree.

2. If W is a nonroot vertex of R, and V -> W is an edge in the CFG such that V
doesn't strictly dominate W, then V is in R. 

Since the intersection of subtrees is a subtree, it is clear from this
definition that the intersection of two SEME regions is a SEME region, which
provides the meet lattice operation. Given set S of vertices in the CFG, it is
possible to define the least SEME region containing S by taking the intersection
of all SEME regions containing S, or by an iterative worklist algorithm that
tries to satisfy the two conditions above. By taking S to be the union of two
SEME regions, this provides the join lattice operation.

It is instructive to see that this definition implicitly handles the problems
with loops discussed for SESE regions. Consider the program fragment

```rust
let mut a = Some(0u);
let p = a.as_ref().unwrap();
loop {
    println!("{}", *p);
    a = None;
    if ... {
        break;
    }
}
```

If R is a SEME region rooted at the borrow used in the definition of `p` that
contains the entry point of the loop, then it must contain the entire loop. This
is because in the loop backedge V -> W, V doesn't strictly dominate W. You can
use similar reasoning to inductively show that the entire loop is contained in R.

Using SEME regions defined in this manner requires in the type checker storing
regions as sets of vertices. While these sets may be optimized, e.g. by using
sparse bitvectors or interval representations, they still have the potential for
poor worst-case behavior. After the regionck phase of the compiler, we can then
compute the exit edges of the region by just looking for edges whose origin is
in the region but whose target is not, and use these exit edges for later
dataflow analyses.

Since SEME regions still have a single entry point, and error messages generally
refer to "the region beginning at...", error messages won't need to change much
or at all. Since SEME regions allow strictly more programs than the current
behavior, this should never be confusing.

# Drawbacks

This is guaranteed to be an increase in complexity in the compiler, and it will
probably slightly impact compile times. However, it does seem like there is a
lot of room for constant-factor implementation improvements, and compilers
already use similar data structures for register allocation on much larger
functions after inlining.

This extended notion of scope also doesn't fit into LLVM's [scope-based aliasing
rules](http://llvm.org/docs/LangRef.html#noalias-and-alias-scope-metadata). This
will make informing LLVM about Rust-specific aliasing information more
difficult, but it does seem possible to extend the LLVM scheme to handle regions
like those defined here.

# Alternatives

1. We could just not do this, and keep borrows being lexical scopes.

2. We could implement SESE regions first, since they are easier to compute and
reason about.

3. If it is possible to implement efficiently, we could use an alternative
definition of a SEME region that is in terms of explicit entry and exit points.
If one is possible, it likely involves the [tree decomposition](http://en.wikipedia.org/wiki/Tree_decomposition)
of a directed graph. Such an algorithm would probably scale better than the one
described here, but would be considerably more complex (and would require a tree
decomposition as a prerequisite).

4. We could go further and try to generalize this to multiple-entry /
multiple-exit regions. This would enable variables to be defined with borrows
on multiple branches of a conditional, e.g.

    ```rust
    let mut i = 0i;
    let p = &mut i;

    let q: &mut int;
    if ... {
        drop(p);
        q = &mut i;
    } else {
        drop(p)
        q = &mut i;
    }
    ```

 This doesn't really seem useful enough to warrant the complexity. It would also
make creating good error messages more difficult.

# Unresolved questions

Does there exist an efficient sparse representation for SEME regions?
