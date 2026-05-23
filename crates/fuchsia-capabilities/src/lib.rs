//! Universal capability interfaces consumed by fuchsia actors.
//!
//! Each capability is a small async trait + value types. Hosts implement
//! the traits (often via a default impl provided here) and inject the
//! resulting handles into the actors they register.

pub mod http;
