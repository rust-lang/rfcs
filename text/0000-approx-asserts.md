- Feature Name: approx_asserts
- Start Date: 2015-02-11
- RFC PR: (leave this empty)
- Rust Issue: (leave this empty)

# Summary

Complement the `assert_eq` macro with `assert_approx` and `assert_tol` macros,
intended for use in unit tests dealing with floating-point numbers.

# Motivation

When writing unit tests on floating-point calculations it is often appropriate
to test for approximate equality, rather than exact equality.

This proposal is to include two macros in the libstd prelude for this purpose:

    // A widely-applicable easy-to-use macro:
    assert_approx!(5.0, 5.0);
    assert_approx!(3.141592, f64::consts::PI);
    assert_approx!(1000000.0, 1000001.0);
    assert_approx!(0.0, 0.000001);
    
    // A macro allowing finer control:
    assert_tol!(1.0e8, 1.003e8, 1e-2, 0.0);
    assert_tol!(0.00008, 0.0, 0.0, 1e-4);

My limited experience is that these two macros provide the minimum necessary
for comfortable testing of floating-point calculations, and that they are
usually also sufficient (e.g. I have not found uses for *assert not approx eq*
or an *is approx eq* function). Comments contradicting or confirming this
experience are welcome.

# Detailed design

The macros could exist alongside or replace
[the existing (but internal) `assert_approx_eq`](https://github.com/rust-lang/rust/blob/master/src/libcore/num/float_macros.rs).

    /// Almost-equality.
    /// 
    /// `assert_tol!(x, y, r, a)` succeeds if `abs(x-y) <= a` or `abs(x/y - 1) <= r`.
    macro_rules! assert_tol {
        ($x:expr, $y:expr, $r:expr, $a:expr) => ({
            use ::std::num::Float;
            let (x, y, a, r) = (&$x, &$y, &$a, &$r);
            let (ad, rd) = ((*x - *y).abs(), (*x / *y - 1.0).abs());
            assert!(ad <= *a || rd <= *r,
                "{} not approx eq {} [abs(diff) = {} > {} AND abs(ratio - 1) = {} > {}]",
                *x, *y, ad, *a, rd, *r);
        })
    }

    /// Almost-equality with fixed (relative and absolute) tolerances of 1e-6.
    macro_rules! assert_approx {
        ($a:expr, $b:expr) => (assert_tol!($a, $b, 1.0e-6, 1.0e-6))
    }

    #[test] #[should_fail]
    fn test_fail_12() {
        assert_approx!(1.0, 2.0);
    }
    #[test] #[should_fail]
    fn test_fail_abs() {
        assert_approx!(1000002.0, 1000000.0);
    }
    #[test] #[should_fail]
    fn test_fail_rel() {
        assert_approx!(0.000002, 0.0);
    }

    #[test]
    fn test_successes(){
        // Simple usages:
        assert_approx!(5.0, 5.0);
        assert_approx!(3.141592, f64::consts::PI);
        assert_approx!(1000001.0, 1000000.0);
        assert_approx!(0.000001, 0.0);
        
        // With explicit tolerances:
        assert_tol!(1e8, 1.003e8, 1e-2, 0.0);
        assert_tol!(0.00008, 0.0, 0.0, 1e-4);
        
        // For 32-bit floats:
        assert_approx!(5f32, 5f32);
        assert_approx!(3.141592f32, f32::consts::PI);
        assert_approx!(1000001f32, 1000000f32);
        assert_approx!(0.000001f32, 0f32);
        assert_tol!(1.003e8f32, 1e8f32, 1e-2f32, 0f32);
        assert_tol!(0.00008f32, 0f32, 0f32, 1e-4f32);
    }

# Drawbacks

This adds two extra macros to the std prelude.

Argument order of `assert_tol` is ambiguous.

# Alternatives

Alternative 1: put this in `libcore`, thus making these available without `libstd`.

Alternative 2: put these in an external crate. A motivation against
doing this is that libstd requires an appropriate assertion macro internally.

Note that an *is approx eq* function is not a good alternative, for the same
reason that `assert_eq` exists: dedicated macros can yield more useful error
messages on test failure.

# Unresolved questions

The tolerances of `assert_approx` could be changed, but are already close to
the accuracy limits that an `f32` can represent (e.g.
`assert_approx!(1000000f32, 1000001f32)` already fails). Different tolerances
for `f32` and `f64` is perhaps not impossible, but do not seem especially
desirable.

Can macros take optional or named arguments?
