# Host Extensibility

A host with custom capabilities needs to expose them to its actors. For
**native Rust actors** this is trivial — capabilities are just `Arc<dyn
SomeTrait>` parameters in the actor's constructor. For **Wasm and Lua
actors** there's an extra layer, because those languages need explicit
bindings on the host side.

That extra layer is the `WasmHost` and `LuaHost` traits.

## WasmHost

`fuchsia-actor-wasm` ships `WasmActor<H: WasmHost>` — generic over a host
trait that owns the world-specific bits:

```rust
#[async_trait]
pub trait WasmHost: 'static + Send + Sync {
    type State: WasiView + 'static + Send;
    type Bindings: Send;

    fn add_to_linker(&self, linker: &mut Linker<Self::State>) -> wasmtime::Result<()>;
    fn fresh_state(&self) -> Self::State;
    async fn instantiate(...) -> wasmtime::Result<Self::Bindings>;
    async fn execute(...) -> wasmtime::Result<Result<String, String>>;
}
```

The host owns:

- **The WIT world** the components import from and export to. This is the
  contract plugin authors target.
- **The `State` type** held inside each `wasmtime::Store`. Holds the
  `WasiCtx`, capability handles (HTTP client, KV store, MQTT client,
  whatever), and any per-Store bookkeeping.
- **The `Bindings` type** — output of `wasmtime::component::bindgen!`
  against the host's WIT.

`WasmActor` provides the rest: channel loop, JSON marshalling between
inbox messages and the component's `execute` function, fresh `Store` per
message, lifecycle and cancellation.

### Writing a custom WasmHost

Steps a custom host follows:

1. **Define a WIT world.** Import whatever capabilities you provide
   (kv, mqtt, modbus, etc.) and export the `fuchsia:task/task` interface
   that all components expose.

   ```wit
   package iot:actor;

   world iot-component {
     import fuchsia:log/log;
     import fuchsia:http/outbound;
     import iot:mqtt/publish;
     import iot:modbus/read;
     export fuchsia:task/task;
   }
   ```

2. **Generate bindings.** Inside a host module:

   ```rust
   wasmtime::component::bindgen!({
       path: "wit",
       world: "iot:actor/iot-component",
       exports: { default: async },
   });
   ```

3. **Define a state struct.** It holds the WASI context, the resource
   table, and any capability handles your imports need to delegate to:

   ```rust
   pub struct IotHostState {
       wasi: WasiCtx,
       table: ResourceTable,
       http: Arc<dyn HttpClient>,
       mqtt: Arc<dyn MqttPublisher>,
       modbus: Arc<dyn ModbusClient>,
   }

   impl WasiView for IotHostState { /* ... */ }
   ```

4. **Implement the WIT Host traits** that bindgen generated for each
   import. Each `impl iot::mqtt::publish::Host for IotHostState { ... }`
   delegates to the corresponding capability handle. Sync WIT functions
   that need to call async traits use `futures::executor::block_on` to
   bridge (same wart `DefaultHost` has for HTTP).

5. **Implement `WasmHost`** for your host struct. The methods are
   mechanical — they wire up the linker, build fresh state, and call
   the bindgen-produced `instantiate_async` / `call_execute` functions.
   Use `crates/fuchsia-actor-wasm/src/default.rs` as a reference.

6. **Register your actor.**

   ```rust
   let actor = WasmActor::builder(engine.clone(), IotHost::new(http, mqtt, modbus))
       .component_from_path("plugins/temp-sensor.wasm")
       .build()?;

   registry.register::<WasmActor<IotHost>, Value, _>(
       "plugins.temp-sensor",
       move |_| actor.clone(),
   );
   ```

Plugins built against `iot:actor/iot-component` now run in this host. A
plugin built against the canonical `fuchsia:platform/actor-component`
world would still work — but it can only import the universal
capabilities (log, http). The IoT-specific imports require the IoT host.

## LuaHost

Lua's version is simpler because there's no WIT or bindgen — just Rust
functions registered into the Lua state:

```rust
pub trait LuaHost: 'static + Send + Sync {
    fn populate(&self, lua: &mlua::Lua) -> mlua::Result<()>;
}
```

`populate` is called once per inbound message, immediately before the
script source is loaded. It's where you register globals — typically as
tables of functions.

### Writing a custom LuaHost

```rust
#[derive(Clone)]
pub struct IotLuaHost {
    http: Arc<dyn HttpClient>,
    mqtt: Arc<dyn MqttPublisher>,
}

impl LuaHost for IotLuaHost {
    fn populate(&self, lua: &mlua::Lua) -> mlua::Result<()> {
        // Register `log` and `http` exactly like DefaultLuaHost does:
        crate::register_default_globals(lua, &self.http)?;

        // Then add your own:
        let mqtt = Arc::clone(&self.mqtt);
        let table = lua.create_table()?;
        table.set("publish", lua.create_function(move |_, (topic, payload): (String, String)| {
            futures::executor::block_on(mqtt.publish(&topic, &payload))
                .map_err(mlua::Error::external)
        })?)?;
        lua.globals().set("mqtt", table)?;

        Ok(())
    }
}
```

Scripts written against this host can now call `mqtt.publish(...)` in
addition to `log.log(...)` and `http.send(...)`.

`DefaultLuaHost::populate` is at `crates/fuchsia-actor-lua/src/default.rs`
and serves as the reference for registering log/http; for a real host
you'll add your custom tables alongside those.

## Naming convention

Across the codebase, host-trait-implementing structs follow this
pattern:

- `DefaultHost` (Wasm) and `DefaultLuaHost` — the canonical Fuchsia
  default. Pair with the universal `fuchsia:platform/actor-component`
  world.
- `<Domain>Host` and `<Domain>LuaHost` for hosts that add domain
  capabilities — e.g. `IotHost` / `IotLuaHost` in an IoT project.

There's no enforced convention; this is just what reads cleanly.

## When to extend vs. write a native actor

Two questions worth asking before writing a custom `WasmHost` /
`LuaHost`:

1. **Do plugins genuinely need access to this capability?** If only
   your own first-party code uses it, write a native Rust actor that
   takes the capability handle directly. Extending the language hosts
   only pays off when third-party Wasm components or Lua scripts also
   need access.
2. **Is the capability stable enough to lock into a WIT contract?** WIT
   interface changes are version-incompatible — old plugins break. For
   experimental capabilities, prefer native actors first; promote to a
   WIT-exposed capability only once the shape is settled.
