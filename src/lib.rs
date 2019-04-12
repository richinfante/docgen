
#[macro_use]
extern crate mozjs;

use std::collections::HashMap;
use serde_json::Value;
use regex::{Regex, Captures};

use std::default::Default;
use std::io::{self, Write};

use html5ever::driver::ParseOpts;
use html5ever::rcdom::RcDom;
use html5ever::tendril::TendrilSink;
use html5ever::tendril::StrTendril;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{parse_document, serialize};
use html5ever::rcdom::{Node, NodeData};
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::{BorrowMut, Borrow};
use std::cell::{Ref, RefMut};
use html5ever::interface::QualName;


use mozjs::jsapi::CompartmentOptions;
use mozjs::jsapi::JS_NewGlobalObject;
use mozjs::jsapi::OnNewGlobalHookOption;
use mozjs::jsval::UndefinedValue;
use mozjs::rust::{JSEngine, Runtime, SIMPLE_GLOBAL_CLASS};
use mozjs::jsval::JSVal;
use mozjs::jsapi::JSContext;
use mozjs::jsapi::JSObject;
use mozjs::jsapi::{JS, self};
use std::ffi::{CStr, CString};
use std::ptr;
// pub fn render_node_and_children(node: &mut Rc<Node>) {
//   if let NodeData::Text { contents } = &node.data {
//     println!("Got text node: {:?}", contents);
//   }

//   let mut arr = node.children.borrow_mut();
//   for child in arr.iter_mut() {
//     let mut c = child;
//     c.data = NodeData::Text { contents: RefCell::new(StrTendril::from("FOO")) }; 
//     // render_node_and_children(&mut child);
//   }
// }

unsafe fn stringify_jsvalue(cx: *mut JSContext, rval: &JSVal) -> String {
  if rval.is_number() {
    return format!("{}", rval.to_number())
  } else if rval.is_int32() {
    return format!("{}", rval.to_int32())
  } else if rval.is_double() {
    return format!("{}", rval.to_double())
  } else if rval.is_string() {
    return mozjs::conversions::jsstr_to_string(cx, rval.to_string())
  } else if rval.is_undefined() {
    return "undefined".into()
  } else if rval.is_null() {
    return "null".into()
  } else if rval.is_boolean() {
    if rval.to_boolean() {
      return "true".into()
    } else {
      return "false".into()
    }
  }

  unimplemented!()
}

unsafe fn boolify_jsvalue(cx: *mut JSContext, rval: &JSVal) -> bool {
  if rval.is_number() {
    return rval.to_number() != 0.0
  } else if rval.is_int32() {
    return rval.to_int32() != 0
  } else if rval.is_double() {
    return rval.to_double() != 0.0
  } else if rval.is_string() {
    return mozjs::conversions::jsstr_to_string(cx, rval.to_string()).len() > 0
  } else if rval.is_undefined() {
    return false
  } else if rval.is_null() {
    return false
  } else if rval.is_boolean() {
    return rval.to_boolean()
  } else if rval.is_object() {
    return true
  } else if rval.is_symbol() {
    return true
  }

  unimplemented!()
}

// pub fn get_symbol(cx: *mut JSContext, global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>,) -> JSVal {
//   unsafe {
//     let c_str = std::ffi::CString::new("Symbol").unwrap();
//     let ptr = c_str.as_ptr() as *const i8;

//     rooted!(in(cx) let mut rval = UndefinedValue());
//     if !jsapi::JS_GetProperty(cx, global.handle(), ptr, handle) {
      
//     }

//     unimplemented!()
//   }
// }

/// Returns the property with the given `name`, if it is a callable object,
/// or an error otherwise.
// pub fn get_callable_property(cx: *mut JSContext, object: *mut JSObject, name: &str) {
//     rooted!(in(cx) let mut callable = UndefinedValue());
//     rooted!(in(cx) let obj = object);
//     unsafe {
//         let c_name = CString::new(name).unwrap();
//         if !jsapi::JS_GetProperty(cx, obj.handle(), c_name.as_ptr(), callable.get().handle_mut()) {
          
//         }

//         // if !callable.is_object() || !IsCallable(callable.to_object()) {
//         //     return Err(Error::Type(format!(
//         //         "The value of the {} property is not callable",
//         //         name
//         //     )));
//         // }
//     }
// }

pub fn eval_in_engine(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, contents: &str) -> String {
  unsafe {
    rooted!(in(cx) let mut rval = UndefinedValue());
    rt.evaluate_script(global.handle(), contents, "test", 1, rval.handle_mut());
    return stringify_jsvalue(cx, &rval)
  }
}

pub fn eval(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, contents: &str) -> Result<JSVal, ()> {
  unsafe {
    rooted!(in(cx) let mut rval = UndefinedValue());
    rt.evaluate_script(global.handle(), contents, "test", 1, rval.handle_mut());

    return Ok(rval.clone());
  }
}

pub fn eval_in_engine_bool(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, contents: &str) -> bool {
  unsafe {
    rooted!(in(cx) let mut rval = UndefinedValue());
    rt.evaluate_script(global.handle(), contents, "test", 1, rval.handle_mut());
    return boolify_jsvalue(cx, &rval)
  }
}


pub struct CondGenFlags {
  remove: bool,
  replace: Option<Vec<Rc<Node>>>
}

impl CondGenFlags {
  pub fn default() -> CondGenFlags {
    CondGenFlags {
      remove: false,
      replace: None
    }
  }
}

/// Get the inner text of a node.
/// This allows us to do things like <script> elements, etc.
pub fn inner_text(node: &Rc<Node>) -> String {
  match node.data.borrow() {
    NodeData::Text { contents } => {
      let mut tendril = contents.borrow();
      format!("{}", tendril)
    },
    _ => {
      let mut children = node.children.borrow();

      let mut string = String::new();

      for item in children.iter() {
        string.push_str(&inner_text(item));
      }

      string
    }
  }
}

/// Get an attribute's value, if it exists.
/// Convert to a rust string for easy comparisons.
pub fn get_attribute(node: &Rc<Node>, key: &str) -> Option<String> {
  match node.data.borrow() {
    NodeData::Element { attrs, .. } =>{
      let attributes : Ref<Vec<html5ever::interface::Attribute>> = attrs.borrow();

      for attr in attributes.iter() {

        let name = &attr.name.local.to_string();

        if name == key {
          return Some(String::from(&attr.value))
        }
      }

      return None
    },
    _ => None
  }
}

unsafe fn render_children(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, node: &mut Rc<Node>) -> CondGenFlags {
    let mut flags = CondGenFlags::default();

    match node.data.borrow() {
      NodeData::Document { .. } => {
        let mut children = node.children.borrow_mut();

        for item in children.iter_mut() {
          render_children(global, rt, cx, item);
        }

        return CondGenFlags::default();

      },
      NodeData::Element { name, attrs, .. } => {
          // println!("element name: {:?}", name);

          if name.local.to_string() == "script" {
            if get_attribute(&node, "ssr") == Some("true".to_string()) {
              let script = inner_text(node);
              eval_in_engine(&global, &rt, cx, &script);
            }
          }

          let mut attributes : RefMut<Vec<html5ever::interface::Attribute>> = attrs.borrow_mut();

          for attr in attributes.iter_mut() {
            // println!("{:?}", attr);
            let name = &attr.name.local.to_string();
            let script = String::from(&attr.value);
            if name.starts_with(":") {
              attr.name = QualName::new(None, "".into(), name[1..].to_string().into());
              attr.value = eval_in_engine(&global, &rt, cx, &script).into();
            } else if name == "x-if" {
              let included = eval_in_engine_bool(&global, &rt, cx, &script);
              if !included {
                return CondGenFlags {
                  remove: true,
                  replace: None
                }
              }
            } else if name == "x-each" {
              let object = eval(&global, &rt, cx, &script).unwrap();

              let replacements : Vec<Rc<Node>> = vec![];

              // 1. Symbol.Iterator
              let mut sym = mozjs::jsapi::GetWellKnownSymbol(cx, jsapi::SymbolCode::iterator);
              let id = mozjs::glue::RUST_SYMBOL_TO_JSID(sym);

              // 3. object[Symbol.iterator]
              rooted!(in(cx) let mut fnhandle = UndefinedValue());
              rooted!(in(cx) let obj = object.to_object());
              if !mozjs::rust::wrappers::JS_GetPropertyById(cx, obj.handle(), mozjs::rust::Handle::new(&id), fnhandle.handle_mut()) {
                  panic!("Couldnt get iterator")
              }

              rooted!(in(cx) let mut iterret = UndefinedValue());
              // rooted!(in(cx) let args = mozjs::jsapi::AutoValueVector);
              // mozjs::rust::wrappers::JS_CallFunctionValue(cx, global.handle(), fnhandle.handle(), mozjs::jsapi::AutoValueVector, iterret.handle_mut());

              return CondGenFlags {
                remove:true,
                replace: Some(replacements)
              }
            }
          }

          // attributes.push(html5ever::interface::Attribute {
          //   name: QualName::new(None, "".into(), "x-generated-ssr".into()),
          //   value: "true".into()
          // });
          let mut out_children : Vec<Rc<Node>> = vec![];

          {
            let mut children = node.children.borrow_mut();

            for item in children.iter_mut() {
              let flags = render_children(global, rt, cx, item);

              if !flags.remove {
                out_children.push(item.clone())
              }

              if let Some(replacements) = flags.replace {
                for item in replacements {
                  out_children.push(item);
                }
              }
            }
          }

          node.children.replace(out_children);

          return CondGenFlags::default();
      },
      NodeData::Text { contents } => {
        let mut tendril = contents.borrow_mut();
        // let contents : String = tendril.into();
        let x = format!("{}", tendril);

        // println!("Text Node: {}", x);

        let regex = Regex::new(r###"\{\{(.*?)\}\}"###).unwrap();

        let result = regex.replace_all(&x, |caps: &Captures| {
          if let Some(code) = caps.get(0) {
            let string : &str = code.into();
            return eval_in_engine(&global, rt, cx, string)
          }

          return "".to_string();
        });

        tendril.clear();
        tendril.try_push_bytes(result.as_bytes()).unwrap();
        return CondGenFlags::default()
      },
      NodeData::Comment { .. } => {
        return CondGenFlags::default()
      },
      NodeData::Doctype { .. } => {
        return CondGenFlags::default()
      },
      NodeData::ProcessingInstruction { .. } => {
        return CondGenFlags::default();
      }
    }
}

pub fn render(template: &mut String, variables: Value) -> String {
  let engine = JSEngine::init().unwrap();
  let rt = Runtime::new(engine);
  let cx = rt.cx();
  unsafe {
      rooted!(in(cx) let global =
          JS_NewGlobalObject(cx, &SIMPLE_GLOBAL_CLASS, ptr::null_mut(),
                              OnNewGlobalHookOption::FireOnNewGlobalHook,
                              &CompartmentOptions::default())
      );

      // TODO: this is bad code.
      eval(&global, &rt, cx, &format!(r###"
      docgen = {{
        version: "{}"
      }};
"###, env!("CARGO_PKG_VERSION")));

    let opts = ParseOpts {
          tree_builder: TreeBuilderOpts {
              drop_doctype: false,
              ..Default::default()
          },
          ..Default::default()
      };
      // let stdin = io::stdin();
      let dom = parse_document(RcDom::default(), opts)
          .from_utf8()
          .read_from(&mut template.as_bytes())
          .unwrap();

      {
        let document : &Node = dom.document.borrow();
        let mut docchildren: RefMut<Vec<Rc<Node>>> = document.children.borrow_mut();
        for a in docchildren.iter_mut() {
          render_children(&global, &rt, cx, a);
        }
      }

    // The validator.nu HTML2HTML always prints a doctype at the very beginning.
    // io::stdout()
    //     .write_all(b"<!DOCTYPE html>\n")
    //     .ok()
    //     .expect("writing DOCTYPE failed");
    let mut buffer = vec![];

    serialize(&mut buffer, &dom.document, Default::default())
        .ok()
        .expect("serialization failed");

    return String::from_utf8(buffer).unwrap();
  }
 
}


