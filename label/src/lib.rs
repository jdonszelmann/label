//! # Label
//!
//! `label` is a library that can be used to create custom attributes for functions, through which you can list them and perform actions on them.
//!
//! # Example
//!
//! ```
//! use label::create_label;
//!
//! create_label!(fn test() -> (););
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
//! ```


pub use label_macros::__label;
pub use label_macros::create_label;
pub use ctor::ctor;

