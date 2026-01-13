wit_bindgen::generate!({
    path: "../../wit",
    world: "trigger-component",
    generate_all,
});

struct TestTrigger;

export!(TestTrigger);

impl exports::fuschia::trigger::trigger::Guest for TestTrigger {
  fn handle(
    event: exports::fuschia::trigger::trigger::Event,
  ) -> Result<exports::fuschia::trigger::trigger::Status, String> {
    use exports::fuschia::trigger::trigger::{Event, Status};

    // Log the event type
    fuschia::log::log::log(fuschia::log::log::Level::Info, "Trigger handle called");

    match event {
      Event::Poll => {
        // Check KV for a "ready" flag
        let ready = fuschia::kv::kv::get("ready");

        if ready.as_deref() == Some("true") {
          // Clear the flag
          fuschia::kv::kv::delete("ready");

          // Get config for payload
          let message =
            fuschia::config::config::get("message").unwrap_or_else(|| "default message".to_string());

          Ok(Status::Completed(format!(
            r#"{{"source": "poll", "message": "{}"}}"#,
            message
          )))
        } else {
          Ok(Status::Pending)
        }
      }
      Event::Webhook(request) => {
        // Store the request info in KV for verification
        fuschia::kv::kv::set("last_method", &request.method);
        fuschia::kv::kv::set("last_path", &request.path);

        // Return completed with request data
        let body = request.body.unwrap_or_else(|| "null".to_string());
        Ok(Status::Completed(format!(
          r#"{{"source": "webhook", "method": "{}", "path": "{}", "body": {}}}"#,
          request.method, request.path, body
        )))
      }
    }
  }
}
