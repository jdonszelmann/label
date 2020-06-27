#![allow(clippy::needless_doctest_main)]
//! # Label
//!
//! `label` is a library that can be used to create custom attributes for functions, through which you can list them and perform actions on them.
//! Label uses no global state during the compilation process, to avoid incremental compilation breaking it.
//!
//! # Example
//!
//! ```
//! use label::create_label;
//!
//! create_label!(fn test() -> ());
//!
//! #[test::label]
//! fn my_fn()  {
//!    println!("Test!");
//! }
//!
//! fn main() {
//!     println!("calling all 'test' label");
//!     // using iter you can go through all functions with this annotation.
//!     for i in test::iter() {
//!         i();
//!     }
//! }
//!
//!
//! ```
//!
//! # Guarantees
//!
//! It is not supported to rely on the on the ordering of test::iter() in any situation.
//! However, it is guaranteed that once the order is set at the start of the application, it stays that way until the application is stopped.
//!

pub use ctor::ctor;
pub use label_macros::__label;
pub use label_macros::create_label;
