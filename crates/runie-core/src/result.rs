//! Result type utilities for fallible operations.
//!
//! Provides [`RunieResult`] as a thin wrapper around the standard `Result` type,
//! along with [`RunieContext`] for ergonomic error handling.

use std::fmt;

/// A fallible operation result type.
///
/// This is a direct wrapper around `Result<T, E>` that adds ergonomic helper
/// methods while remaining compatible with the standard `Result` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunieResult<T, E> {
    /// The operation succeeded with value `T`.
    Ok(T),
    /// The operation failed with error `E`.
    Err(E),
}

impl<T, E> RunieResult<T, E> {
    /// Returns `true` if the result is `Ok`.
    #[inline]
    pub fn is_ok(&self) -> bool {
        matches!(self, RunieResult::Ok(_))
    }

    /// Returns `true` if the result is `Err`.
    #[inline]
    pub fn is_err(&self) -> bool {
        matches!(self, RunieResult::Err(_))
    }

    /// Unwraps the value, panicking if the result is `Err`.
    ///
    /// # Panics
    ///
    /// Panics if the value is `Err`, with a message including the error value.
    #[track_caller]
    pub fn unwrap(self) -> T
    where
        E: fmt::Debug,
    {
        match self {
            RunieResult::Ok(t) => t,
            RunieResult::Err(e) => panic!("called RunieResult::unwrap() on an Err value: {:?}", e),
        }
    }

    /// Unwraps the error value, panicking if the result is `Ok`.
    ///
    /// # Panics
    ///
    /// Panics if the value is `Ok`, with a message including the value.
    #[track_caller]
    pub fn unwrap_err(self) -> E
    where
        T: fmt::Debug,
    {
        match self {
            RunieResult::Ok(t) => panic!("called RunieResult::unwrap_err() on an Ok value: {:?}", t),
            RunieResult::Err(e) => e,
        }
    }

    /// Maps a `RunieResult<T, E>` to `RunieResult<U, E>` by applying a function to the value.
    #[inline]
    pub fn map<U, F>(self, f: F) -> RunieResult<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            RunieResult::Ok(t) => RunieResult::Ok(f(t)),
            RunieResult::Err(e) => RunieResult::Err(e),
        }
    }

    /// Maps a `RunieResult<T, E>` to `RunieResult<T, F>` by applying a function to the error.
    #[inline]
    pub fn map_err<F, O>(self, f: F) -> RunieResult<T, O>
    where
        F: FnOnce(E) -> O,
    {
        match self {
            RunieResult::Ok(t) => RunieResult::Ok(t),
            RunieResult::Err(e) => RunieResult::Err(f(e)),
        }
    }

    /// Chains a fallible operation on the value.
    ///
    /// Returns `Ok(())` if both operations succeed, otherwise returns the first error.
    #[inline]
    pub fn and_then<U, F>(self, f: F) -> RunieResult<U, E>
    where
        F: FnOnce(T) -> RunieResult<U, E>,
    {
        match self {
            RunieResult::Ok(t) => f(t),
            RunieResult::Err(e) => RunieResult::Err(e),
        }
    }
}

impl<T, E> From<Result<T, E>> for RunieResult<T, E> {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(t) => RunieResult::Ok(t),
            Err(e) => RunieResult::Err(e),
        }
    }
}

impl<T> From<anyhow::Error> for RunieResult<T, anyhow::Error> {
    fn from(e: anyhow::Error) -> Self {
        RunieResult::Err(e)
    }
}

impl<T> From<RunieResult<T, anyhow::Error>> for anyhow::Result<T> {
    fn from(result: RunieResult<T, anyhow::Error>) -> Self {
        match result {
            RunieResult::Ok(t) => Ok(t),
            RunieResult::Err(e) => Err(e),
        }
    }
}

/// Context for fallible operations with convenient error handling.
///
/// Provides a lightweight wrapper for chaining operations that may fail,
/// automatically converting errors along the way.
pub struct RunieContext;

impl RunieContext {
    /// Wraps a fallible operation that returns a `RunieResult`.
    #[inline]
    pub fn run<T, E, F>(f: F) -> RunieResult<T, E>
    where
        F: FnOnce() -> RunieResult<T, E>,
    {
        f()
    }

    /// Converts a `RunieResult<T, anyhow::Error>` to `anyhow::Result<T>`.
    #[inline]
    pub fn into_anyhow<T>(result: RunieResult<T, anyhow::Error>) -> anyhow::Result<T> {
        result.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ok_is_err() {
        let ok: RunieResult<i32, &str> = RunieResult::Ok(42);
        let err: RunieResult<i32, &str> = RunieResult::Err("oops");

        assert!(ok.is_ok());
        assert!(!ok.is_err());
        assert!(!err.is_ok());
        assert!(err.is_err());
    }

    #[test]
    fn test_unwrap() {
        let ok: RunieResult<i32, &str> = RunieResult::Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: RunieResult<i32, &str> = RunieResult::Err("oops");
        let result = std::panic::catch_unwind(|| {
            let _: i32 = err.unwrap();
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_unwrap_err() {
        let err: RunieResult<i32, &str> = RunieResult::Err("oops");
        assert_eq!(err.unwrap_err(), "oops");

        let ok: RunieResult<i32, &str> = RunieResult::Ok(42);
        let result = std::panic::catch_unwind(|| {
            let _: &str = ok.unwrap_err();
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_map() {
        let ok: RunieResult<i32, &str> = RunieResult::Ok(5);
        let mapped = ok.map(|x| x * 2);
        assert!(matches!(mapped, RunieResult::Ok(10)));

        let err: RunieResult<i32, &str> = RunieResult::Err("fail");
        let mapped = err.map(|x: i32| x * 2);
        assert!(matches!(mapped, RunieResult::Err("fail")));
    }

    #[test]
    fn test_map_err() {
        let ok: RunieResult<i32, &str> = RunieResult::Ok(5);
        let mapped = ok.map_err(|e| e.len());
        assert!(matches!(mapped, RunieResult::Ok(5)));

        let err: RunieResult<i32, &str> = RunieResult::Err("error");
        let mapped = err.map_err(|e| e.len());
        assert!(matches!(mapped, RunieResult::Err(5)));
    }

    #[test]
    fn test_and_then() {
        let ok: RunieResult<i32, &str> = RunieResult::Ok(10);
        let chained = ok.and_then(|x| RunieResult::Ok(x / 2));
        assert!(matches!(chained, RunieResult::Ok(5)));

        let err: RunieResult<i32, &str> = RunieResult::Err("early");
        let chained = err.and_then(|x: i32| RunieResult::Ok(x / 2));
        assert!(matches!(chained, RunieResult::Err("early")));
    }

    #[test]
    fn test_from_std_result() {
        let std_ok: Result<i32, &str> = Ok(42);
        let runie: RunieResult<i32, &str> = std_ok.into();
        assert!(runie.is_ok());

        let std_err: Result<i32, &str> = Err("fail");
        let runie: RunieResult<i32, &str> = std_err.into();
        assert!(runie.is_err());
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("something went wrong");
        let runie: RunieResult<i32, anyhow::Error> = anyhow_err.into();
        assert!(runie.is_err());
    }

    #[test]
    fn test_runie_context() {
        let result: RunieResult<i32, &str> = RunieContext::run(|| RunieResult::Ok(42));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        let anyhow_result: anyhow::Result<i32> =
            RunieContext::into_anyhow(RunieResult::<i32, anyhow::Error>::Ok(42));
        assert!(anyhow_result.is_ok());
    }

    #[test]
    fn test_debug_derive() {
        let ok: RunieResult<i32, &str> = RunieResult::Ok(42);
        let debug = format!("{:?}", ok);
        assert!(debug.contains("Ok"));

        let err: RunieResult<i32, &str> = RunieResult::Err("oops");
        let debug = format!("{:?}", err);
        assert!(debug.contains("Err"));
    }

    #[test]
    fn test_clone() {
        let original: RunieResult<i32, &str> = RunieResult::Ok(42);
        let cloned = original;
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_partial_eq() {
        let a: RunieResult<i32, &str> = RunieResult::Ok(1);
        let b: RunieResult<i32, &str> = RunieResult::Ok(1);
        let c: RunieResult<i32, &str> = RunieResult::Ok(2);
        let d: RunieResult<i32, &str> = RunieResult::Err("x");

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }
}
