pub use glam::*;
pub mod util;
pub use util::*;

/// Gets the minimum value of all impute. Works on most types.
///
/// Each float impute must specify its type like so.
///``` rust
///
///     min!(2.5_f32, 6.1_f32, -3.71_f32);
///
///     fn foo(x: f64, y: f64, z: f64) {
///         min!(x, y, z);
///     }
///
///```
#[macro_export]
macro_rules! min {
    ($x:expr) => ($x);

    ($x:expr, $($y:expr), +) => (
    $x.min(min!($($y), +))
    )
}

/// Gets the maximum value of all impute. Works on most types.
///
/// Each float impute must specify its type like so.
///``` rust
///
///     max!(2.5_f32, 6.1_f32, -3.71_f32);
///
///     fn bar(x: f64, y: f64, z: f64) {
///         max!(x, y, z);
///     }
///
///```
#[macro_export]
macro_rules! max {
    ($x:expr) => ($x);

    ($x:expr, $($y:expr), +) => (
$x.max(max!($($y), +))
    )
}

/// prints the input expression fallowed by the result.
/// useful for debugging.
#[macro_export]
macro_rules! println_expression {
    ($e:expr) => {
        println!("{:?} = {:?}", stringify!($e), $e);
    };
}

mod testing {
    #[test]
    fn min_test() {
        assert_eq!(min!(2, -5, 1, 3, 0, -1), -5)
    }

    #[test]
    fn max_test() {
        assert_eq!(max!(2, -5, 1, 3, 0, -1), 3)
    }

    #[test]
    fn min_max_test() {
        assert_eq!(min!(15, 13, max!(-5, 2, 3, 7)), 7)
    }

    #[test]
    fn min_f32_test() {
        assert_eq!(
            min!(2.0_f32, -5.0_f32, 1.0_f32, 3.0_f32, 0.0_f32, -1.0_f32),
            -5.0_f32
        )
    }

    #[test]
    fn max_f32_test() {
        assert_eq!(
            max!(2.0_f32, -5.0_f32, 1.0_f32, 3.0_f32, 0.0_f32, -1.0_f32),
            3.0_f32
        )
    }
}
