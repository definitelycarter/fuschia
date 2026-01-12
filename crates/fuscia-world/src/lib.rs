//! Generated bindings for fuscia host world.
//!
//! This crate uses wasmtime's bindgen to generate Rust bindings from WIT interfaces.
//! The host world combines all imports the host provides and exports the host calls.

wasmtime::component::bindgen!({
    path: "../../wit",
    inline: r#"
        package fuscia:runtime;

        world host {
            include fuscia:fuscia/host;
        }
    "#,
    imports: { default: async },
    exports: { default: async },
});
