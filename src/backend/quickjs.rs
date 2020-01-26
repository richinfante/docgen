use quick_js::{Context, JsValue};

struct QJSScriptingAdapter {
  context: Option<Context>
}

impl QJSScriptingAdapter {
  fn get_iterator_values(&self, expression: &str) -> Vec<JsValue>{
    self.context.unwrap().eval(format!("var __iter = {};", expression));
    self.context.unwrap().eval(format!("var __iterator = __iter[Symbol.iterator]();", expression));
  }

  fn perform_iteration(&self, expression: &str) -> Vec<serde_json::Value> {

  }

  fn is_expression_truthy(&self, expression: &str) -> bool {
    self.context.unwrap().eval_as::<bool>(expression).unwrap();
  }



  fn execute_expression(&self, expresion: &str) {
    self.context.unwrap().eval(expresion).unwrap();
  }
}

impl ScriptingAdapter  for QJSScriptingAdapter {
  /// Returns the name of this backend.
  /// Used mostly for debugging
  fn backend_name(&self) -> String {
    "quickjs".into()
  }

  /// Check if the runtime is initialized.
  fn is_initialized(&self) -> bool {
    self.context.is_some()
  }

  /// Perform initialization
  fn init(&mut self) {
    self.context = Context::new().unwrap();
  }

  /// Iteration
  // fn perform_iteration(&self, expression: &str) -> Vec<serde_json::Value>;
  // fn is_expression_truthy(&self, expression: &str) -> bool;
  // fn execute_expression(&self, expresion: &str);
  fn teardown(&self) {
    self.context.unwrap().reset();
  }
}
