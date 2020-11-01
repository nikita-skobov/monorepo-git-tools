/// in debug mode, use panic so we get a stack trace
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! die {
    () => (::std::process::exit(1));
    ($x:expr; $($y:expr),+) => ({
        panic!($($y),+);
    });
    ($($y:expr),+) => ({
        panic!($($y),+);
    });
}

/// in release mode, use print so its not ugly
/// Example:
/// ```
/// if bad_condition { die!("Oops, the condition was {}", bad_condition)}
/// ```
/// if in debug mode, this will panic, otherwise this will println, and then exit 1
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! die {
    () => (::std::process::exit(1));
    ($x:expr; $($y:expr),+) => ({
        println!($($y),+);
        ::std::process::exit($x)
    });
    ($($y:expr),+) => ({
        println!($($y),+);
        ::std::process::exit(1)
    });
}
