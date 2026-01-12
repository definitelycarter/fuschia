//! Generated bindings for fuscia host world.
//!
//! This crate uses wasmtime's bindgen to generate Rust bindings from WIT interfaces.
//! The host world combines all imports the host provides and exports the host calls.

wasmtime::component::bindgen!({
    path: "../../wit",
    inline: r#"
        package fuscia:host;

        world host {
            // Host imports (provided by engine to components)
            import fuscia:kv/kv@0.1.0;
            import fuscia:config/config@0.1.0;
            import fuscia:log/log@0.1.0;
            import wasi:http/outgoing-handler@0.2.0;

            // Component exports (called by engine)
            export fuscia:task/task@0.1.0;
            export fuscia:trigger/trigger@0.1.0;
        }
    "#,
    async: true,
});
