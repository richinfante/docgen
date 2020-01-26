use super::*;

pub use mozjs::rust::{JSEngine, ParentRuntime, Runtime, SIMPLE_GLOBAL_CLASS};
use mozjs::jsapi;
use mozjs::jsapi::CallArgs;
use mozjs::jsapi::CompartmentOptions;
use mozjs::jsapi::JSContext;
use mozjs::jsapi::JSString;
use mozjs::jsapi::JS_NewGlobalObject;
use mozjs::jsapi::OnNewGlobalHookOption;
use mozjs::jsval::JSVal;
use mozjs::jsval::UndefinedValue;

use std::ptr;
use std::ffi::CStr;

struct Ctx {
  rt: Runtime,
  cx: *mut JSContext,
  global: std::boxed::Box<mozjs_sys::jsgc::Heap<*mut mozjs::jsapi::JSObject>>
}

struct SpiderMonkeyAdapter {
  context: Option<Ctx>
}

impl SpiderMonkeyAdapter {
  pub fn init_runtime() -> (Runtime) {
      let engine = JSEngine::init().unwrap();
      Runtime::new(engine)
  }

  pub fn init_js() -> (Runtime, *mut JSContext) {
      let rt = SpiderMonkeyAdapter::init_runtime();
      let cx = rt.cx();
      (rt, cx)
  }

  
  /// Stringify a jsval into a rust string for injection into the template
  unsafe fn stringify_jsvalue(&self, rval: &JSVal) -> String {
    let cx = self.context.expect("engine to be initialized.").cx;
    if rval.is_number() {
        return format!("{}", rval.to_number());
    } else if rval.is_int32() {
        return format!("{}", rval.to_int32());
    } else if rval.is_double() {
        return format!("{}", rval.to_double());
    } else if rval.is_string() {
        return mozjs::conversions::jsstr_to_string(cx, rval.to_string());
    } else if rval.is_undefined() {
        return "undefined".into();
    } else if rval.is_null() {
        return "null".into();
    } else if rval.is_boolean() {
        if rval.to_boolean() {
            return "true".into();
        } else {
            return "false".into();
        }
    }

    // TODO: json stringify here.

    return "(error: stringify_jsvalue unimplemented type!)".to_string();
  }

  /// Boolean check for a jsval.
  /// This is used for x-if.
  unsafe fn boolify_jsvalue(&self, rval: &JSVal) -> bool {
    let cx = self.context.expect("engine to be initialized.").cx;

    if rval.is_number() {
        return rval.to_number() != 0.0;
    } else if rval.is_int32() {
        return rval.to_int32() != 0;
    } else if rval.is_double() {
        return rval.to_double() != 0.0;
    } else if rval.is_string() {
        return mozjs::conversions::jsstr_to_string(cx, rval.to_string())
            .trim()
            .len()
            > 0;
    } else if rval.is_undefined() {
        return false;
    } else if rval.is_null() {
        return false;
    } else if rval.is_boolean() {
        return rval.to_boolean();
    } else if rval.is_object() {
        return true;
    } else if rval.is_symbol() {
        return true;
    }

    unimplemented!()
  }

  /// Evaluate a javascript string in the engine, with respect to a specific global object.
  /// Return a string representing the value
  pub fn eval_in_engine(
    &self,
    filename: Option<&str>,
    contents: &str,
  ) -> Result<String, String> {
    let rt = self.context.unwrap().rt;
    let cx = self.context.unwrap().cx;
    let global = self.context.unwrap().global;

    rooted!(in(cx) let mut rval = UndefinedValue());
    let res = rt.evaluate_script(
        global.handle(),
        contents,
        filename.unwrap_or("eval"),
        1,
        rval.handle_mut(),
    );

    if !res.is_ok() {
        // print_exception(&rt, cx);
        let exception = fmt_exception(&rt, cx);
        error!("Error: {}", exception);
        error!("Error: evaluating: {}, {:?}", contents, res);
        return Err(exception);
    }

    unsafe {
        return Ok(stringify_jsvalue(cx, &rval));
    }
  }

  // Evaluate and return a javascript value
  pub fn eval(
    global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>,
    rt: &Runtime,
    cx: *mut JSContext,
    contents: &str,
  ) -> Result<JSVal, ()> {
    rooted!(in(cx) let mut rval = UndefinedValue());
    let res = rt.evaluate_script(
        global.handle(),
        contents,
        "docgen_eval",
        1,
        rval.handle_mut(),
    );

    if !res.is_ok() {
        print_exception(&rt, cx);
        panic!("Error evaluating: {}, {:?}", contents, res);
    }

    return Ok(rval.clone());
  }

  /// Evaluate an expression in the engine, and return whether or not the return value is truthy.
  pub fn eval_in_engine_bool(
    global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>,
    rt: &Runtime,
    cx: *mut JSContext,
    contents: &str,
  ) -> bool {
    unsafe {
        rooted!(in(cx) let mut rval = UndefinedValue());
        let res = rt.evaluate_script(
            global.handle(),
            contents,
            "docgen_eval_bool",
            1,
            rval.handle_mut(),
        );
        if !res.is_ok() {
            return false;
        }

        return boolify_jsvalue(cx, &rval);
    }
  }
}

impl ScriptingAdapter for SpiderMonkeyAdapter {
  fn backend_name(&self) -> String {
    unsafe {
      let spidermonkey_version = mozjs::jsapi::JS_GetImplementationVersion();
      let spidermonkey_version_c_str: &CStr = CStr::from_ptr(spidermonkey_version);
      let spidermonkey_version_str_slice: &str = spidermonkey_version_c_str.to_str().unwrap();

      String::from(spidermonkey_version_str_slice)
    }
  }

  fn init(&mut self) {
    let (rt, cx) = SpiderMonkeyAdapter::init_js();
    

    unsafe {
      // Intializae the globals
      let global = mozjs::jsapi::Heap::boxed(JS_NewGlobalObject(cx, &SIMPLE_GLOBAL_CLASS, ptr::null_mut(),
                                  OnNewGlobalHookOption::FireOnNewGlobalHook,
                                  &CompartmentOptions::default()));
      
      // Setup autocompartment.
      // Required or wierd things happen.
      let _ac = mozjs::jsapi::JSAutoCompartment::new(cx, global.get());

      // Setup global standard classes
      assert!(mozjs::jsapi::JS_InitStandardClasses(
          cx,
          global.handle()
      ));

      self.context = Some(Ctx { rt, cx, global: global });
    }
  }

  fn is_initialized(&self) -> bool {
    return self.context.is_some();
  }

  fn teardown(&self) {

  }
}