// pub mod spidermonkey;
pub mod quickjs;

trait ScriptingAdapter {
  /// Returns the name of this backend.
  /// Used mostly for debugging
  fn backend_name(&self) -> String;

  /// Check if the runtime is initialized.
  fn is_initialized(&self) -> bool;

  /// Perform initialization
  fn init(&mut self);

  /// Iteration
  // fn perform_iteration(&self, expression: &str) -> Vec<serde_json::Value>;
  // fn is_expression_truthy(&self, expression: &str) -> bool;
  // fn execute_expression(&self, expresion: &str);
  fn teardown(&self);
}

struct AbstractScriptStore {
  backend: Box<dyn ScriptingAdapter>
}