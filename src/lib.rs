
#[macro_use]
extern crate mozjs;

use serde_json::Value;

use regex::{Regex, Captures};

use html5ever::driver::ParseOpts;
use html5ever::rcdom::RcDom;
use html5ever::tendril::TendrilSink;
use html5ever::tendril::StrTendril;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{parse_document, serialize};
use html5ever::rcdom::{Node, NodeData};
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

#[macro_use]
extern crate log;

use std::collections::HashMap;
use std::default::Default;
use std::io::{self, Write};
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::{BorrowMut, Borrow};
use std::cell::{Ref, RefMut};
use std::ffi::{CStr, CString};
use std::ptr;

/// Stringify a jsval into a rust string for injection into the template
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

/// Boolean check for a jsval.
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

pub fn eval_in_engine(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, contents: &str) -> String {
  unsafe {
    rooted!(in(cx) let mut rval = UndefinedValue());
    rt.evaluate_script(global.handle(), contents, "docgen", 1, rval.handle_mut());
    return stringify_jsvalue(cx, &rval)
  }
}

pub fn eval(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, contents: &str) -> Result<JSVal, ()> {
  unsafe {
    rooted!(in(cx) let mut rval = UndefinedValue());
    rt.evaluate_script(global.handle(), contents, "docgen", 1, rval.handle_mut());

    return Ok(rval.clone());
  }
}

pub fn eval_in_engine_bool(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, contents: &str) -> bool {
  unsafe {
    rooted!(in(cx) let mut rval = UndefinedValue());
    rt.evaluate_script(global.handle(), contents, "docgen", 1, rval.handle_mut());
    return boolify_jsvalue(cx, &rval)
  }
}

/// Flags used for conditional node generation
pub struct CondGenFlags {
  remove: bool,
  replace: Option<Vec<Rc<Node>>>
}

impl CondGenFlags {
  /// Set up the default flags
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

use html5ever::interface::Attribute;

fn clone_node_data(data: &NodeData) -> NodeData {
  match data {
    NodeData::Document => NodeData::Document,
    NodeData::Text { contents } => {
      let tendril = contents.borrow();
      NodeData::Text {
        contents: RefCell::new(tendril.try_subtendril(0, tendril.len32()).unwrap())
      }
    },
    NodeData::Doctype { name, public_id, system_id } => {
      NodeData::Doctype {
        name: name.try_subtendril(0, name.len32()).unwrap(),
        public_id: public_id.try_subtendril(0, public_id.len32()).unwrap(),
        system_id: system_id.try_subtendril(0, system_id.len32()).unwrap(),
      }
    },
    NodeData::Comment { contents } => {
      NodeData::Comment {
        contents: contents.try_subtendril(0, contents.len32()).unwrap()
      }
    },
    NodeData::ProcessingInstruction { target, contents } => {
      NodeData::ProcessingInstruction {
        target: target.try_subtendril(0, target.len32()).unwrap(),
        contents: contents.try_subtendril(0, contents.len32()).unwrap()
      }
    }
    NodeData::Element { name, attrs, template_contents, mathml_annotation_xml_integration_point } => {
      NodeData::Element {
        name: name.clone(),
        attrs: RefCell::new(attrs.borrow().iter().map(|v| {
          Attribute {
            name: v.name.clone(),
            value: v.value.try_subtendril(0, v.value.len32()).unwrap()
          }
        }).collect::<Vec<Attribute>>()),
        template_contents: template_contents.clone(),
        mathml_annotation_xml_integration_point: *mathml_annotation_xml_integration_point
      }
    }
  }
}
use std::rc::Weak;
use std::cell::Cell;

fn deep_clone(old_node: &Rc<Node>, parent: Option<Weak<Node>>) -> Rc<Node> {
  let mut node = Rc::new(Node {
    children: RefCell::new(vec![]),
    data: clone_node_data(old_node.data.borrow()),
    parent: Cell::new(parent)
  });

  let new : Vec<Rc<Node>>;
  let weak = std::rc::Rc::downgrade(&node);
  {
    new = old_node.children.borrow().iter().map(|v| {
      deep_clone(v, Some(weak.clone()))
    }).collect::<Vec<Rc<Node>>>();
  }

  let node_mut = node.borrow_mut();
  node_mut.children.replace(new);

  debug!("deep clone info. {} kids had, {} kids found.", old_node.children.borrow().len() ,node_mut.children.borrow().len());

  node
}

/// Render the children of a node, recursively.
unsafe fn render_children(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, node: &mut Rc<Node>, loop_expansion: bool) -> CondGenFlags {
    let mut flags = CondGenFlags::default();

    match node.data.borrow() {
      NodeData::Document { .. } => {
        let mut children = node.children.borrow_mut();

        for item in children.iter_mut() {
          render_children(global, rt, cx, item, true);
        }

        return CondGenFlags::default();

      },
      NodeData::Element { name, attrs, .. } => {
          // debug!("element name: {:?}", name);

          if name.local.to_string() == "script" {
            if get_attribute(&node, "ssr") == Some("true".to_string()) {
              let script = inner_text(node);
              eval_in_engine(&global, &rt, cx, &script);
            }

            return CondGenFlags {
              remove: true,
              replace: None
            }
          }

          let mut needs_expansion: Option<String> = None;
          let mut replacements : Vec<Rc<Node>> = vec![];
          let mut needs_remove : bool = false;
          let mut final_attrs : Vec<html5ever::interface::Attribute> = vec![];

          {
            let mut attributes : RefMut<Vec<html5ever::interface::Attribute>> = attrs.borrow_mut();

            for attr in attributes.iter_mut() {
              // debug!("{:?}", attr);
              let name = &attr.name.local.to_string();
              let script = String::from(&attr.value);
              if name.starts_with(":") {
                attr.name = QualName::new(None, "".into(), name[1..].to_string().into());
                attr.value = eval_in_engine(&global, &rt, cx, &script).into();
                final_attrs.push(attr.clone());
              } else if name == "x-if" {
                let included = eval_in_engine_bool(&global, &rt, cx, &script);
                if !included {
                  return CondGenFlags {
                    remove: true,
                    replace: None
                  }
                }
              } else if name == "x-each" && loop_expansion {
                needs_expansion = Some(script);
              } else {
                final_attrs.push(attr.clone());
              }
            }
          }

          attrs.replace(final_attrs);

          if let Some(script) = needs_expansion {
            if loop_expansion {
              debug!("begin loop expansion on {:?}", name);
              // 0. Run the script and setup replacement list.
              let object = eval(&global, &rt, cx, &script).unwrap();

              // 1. Symbol.Iterator
              let mut sym = mozjs::jsapi::GetWellKnownSymbol(cx, jsapi::SymbolCode::iterator);
              let id = mozjs::glue::RUST_SYMBOL_TO_JSID(sym);

              if !object.is_object() {
                debug!("eval returned non-object!");
              }

              // 2. object[Symbol.iterator]
              rooted!(in(cx) let mut fnhandle = UndefinedValue());
              rooted!(in(cx) let obj = object.to_object());
              if !mozjs::rust::wrappers::JS_GetPropertyById(cx, obj.handle(), mozjs::rust::Handle::new(&id), fnhandle.handle_mut()) {
                  panic!("Couldnt get iterator")
              }

              // 3. Call function to get an iterator.
              rooted!(in(cx) let mut iterret = UndefinedValue());
              rooted!(in(cx) let func = mozjs::rust::wrappers::JS_ValueToFunction(cx, fnhandle.handle()));
              let args = mozjs::jsapi::HandleValueArray::new();
              mozjs::rust::wrappers::JS_CallFunction(cx, obj.handle(), func.handle(), &args, iterret.handle_mut());
              
              loop {
                needs_remove = true;
                debug!("iterating thing...");
                // 4. Call the iterator.next() function
                rooted!(in(cx) let mut returnvalue = UndefinedValue());
                rooted!(in(cx) let iter_result = iterret.to_object());
                let next_str = std::ffi::CString::new("next").unwrap();
                let next_ptr = next_str.as_ptr() as *const i8;
                let args = mozjs::jsapi::HandleValueArray::new();
                rooted!(in(cx) let mut iteration_value = UndefinedValue());
                mozjs::rust::wrappers::JS_CallFunctionName(cx, iter_result.handle(), next_ptr, &args, iteration_value.handle_mut());

                if !iteration_value.is_object() {
                  debug!("iteration is not obj");
                  break
                }

                /// 5. Get result.value
                rooted!(in(cx) let mut iteration_result_value = UndefinedValue());

                if iteration_value.is_object() {
                  let c_str = std::ffi::CString::new("value").unwrap();
                  let ptr = c_str.as_ptr() as *const i8;
                  rooted!(in(cx) let iteration_value_obj = iteration_value.to_object());
                  mozjs::rust::wrappers::JS_GetProperty(cx, iteration_value_obj.handle(), ptr, iteration_result_value.handle_mut());
                }

                // debug!("iter value -> {}", stringify_jsvalue(cx, &iteration_result_value));

                /// 6. Check if iterator is done
                let done_str = std::ffi::CString::new("done").unwrap();
                let done_str_ptr = done_str.as_ptr() as *const i8;
                rooted!(in(cx) let iteration_value_obj = iteration_value.to_object());
                rooted!(in(cx) let mut iteration_done_value = UndefinedValue());
                mozjs::rust::wrappers::JS_GetProperty(cx, iteration_value_obj.handle(), done_str_ptr, iteration_done_value.handle_mut());
                debug!("iter done -> {}", stringify_jsvalue(cx, &iteration_done_value));

                // Boolify javascript
                if boolify_jsvalue(cx, &iteration_done_value) {
                  debug!("is done. return.");
                  break
                }

                // HACK: Store the value temporarily in the global scope.
                // TODO: iterate and create new scopes with the values.
                let result_name = std::ffi::CString::new("item").unwrap();
                let result_name_ptr = result_name.as_ptr() as *const i8;
                rooted!(in(cx) let iter_val = iteration_result_value.clone());
                mozjs::rust::wrappers::JS_SetProperty(cx, global.handle(), result_name_ptr, iter_val.handle());

                // Mut and render
                let x = node.parent.replace(None);
                let parent: Option<Weak<Node>>;
                if let Some(x) = x {
                  parent = Some(x.clone());
                } else {
                  parent = None;
                }

                let mut expand_node = deep_clone(node, parent);
                render_children(global, rt, cx, &mut expand_node, false);
                replacements.push(expand_node);
              }
            }
          }

          // attributes.push(html5ever::interface::Attribute {
          //   name: QualName::new(None, "".into(), "x-generated-ssr".into()),
          //   value: "true".into()
          // });

          debug!("Rendering children...");
          let mut out_children : Vec<Rc<Node>> = vec![];

          {
            let mut children = node.children.borrow_mut();
            debug!("have {} children.", children.len());
            for item in children.iter_mut() {
              debug!("Rendering child...");
              let flags = render_children(global, rt, cx, item, true);

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

          debug!("render done. injecting kids.");

          node.children.replace(out_children);

          CondGenFlags {
            replace: Some(replacements),
            remove: needs_remove
          }
      },
      NodeData::Text { contents } => {
        let mut tendril = contents.borrow_mut();
        // let contents : String = tendril.into();
        let x = format!("{}", tendril);

        // debug!("Text Node: {}", x);

        let regex = Regex::new(r###"\{\{(.*?)\}\}"###).unwrap();

        let result = regex.replace_all(&x, |caps: &Captures| {
          if let Some(code) = caps.get(0) {
            let string : &str = code.into();
            debug!("render var: {}", string);
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

/// Render a template.
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

    // NOTE: this line is important, without it all JS_ calls seem to segfault.
    let _ac = mozjs::jsapi::JSAutoCompartment::new(cx, global.get());
    assert!(mozjs::rust::wrappers::JS_InitStandardClasses(cx, global.handle()));

    let spidermonkey_version = mozjs::jsapi::JS_GetImplementationVersion();
    let spidermonkey_version_c_str: &CStr = unsafe { CStr::from_ptr(spidermonkey_version) };
    let spidermonkey_version_str_slice: &str = spidermonkey_version_c_str.to_str().unwrap();
      // TODO: this is bad code.
      eval(&global, &rt, cx, &format!(r###"
      docgen = {{
        version: "{}",
        spidermonkey_version: "{}"
      }};
"###, env!("CARGO_PKG_VERSION"), spidermonkey_version_str_slice));

    let empty_args : mozjs::jsapi::HandleValueArray = mozjs::jsapi::HandleValueArray::new();
    // let rp = &empty_args as *const mozjs::jsapi::HandleValueArray;
    // debug!("{:?} {:?}", empty_args, rp);

    // debug!("rooting undefined.");
    // rooted!(in(cx) let mut iterret = UndefinedValue());
    // let c_str = std::ffi::CString::new("fn1").unwrap();
    // let ptr = c_str.as_ptr() as *const i8;
    // debug!("creating thing: {:?}", ptr);

    // // mozjs::rust::wrappers::JS_GetProperty(cx, global.handle(), ptr, iterret.handle_mut());
    // mozjs::rust::jsapi_wrapped::JS_CallFunctionName(cx, global.handle(), ptr, &empty_args, &mut iterret.handle_mut());
    // debug!("Stringified: {}", stringify_jsvalue(cx, &iterret));
    // debug!("Done!");
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
          render_children(&global, &rt, cx, a, true);
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


