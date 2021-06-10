
pub mod export_parser;
pub mod filter;
pub mod filter_state;


/// used to make a simple io error with a string formatted message
/// use this when you want to do `some_call().map_err(ioerr!("message"))?;`
#[macro_export]
macro_rules! ioerr {
    ($($arg:tt)*) => ({
        ::std::io::Error::new(::std::io::ErrorKind::Other, format!($($arg)*))
    })
}

/// same as `ioerr` except this actually wraps it in an `Err()`
/// use this when you want to do: `return ioerre!("message")`
#[macro_export]
macro_rules! ioerre {
    ($($arg:tt)*) => ({
        Err($crate::ioerr!($($arg)*))
    })
}
