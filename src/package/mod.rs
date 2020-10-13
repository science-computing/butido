//! Module that contains all types and functionality that has to do with a package.

mod dependency; 
pub use dependency::*;

mod name; 
pub use name::*;

mod package; 
pub use package::*;

mod source; 
pub use source::*;

mod tree; 
pub use tree::*;

mod version; 
pub use version::*;

