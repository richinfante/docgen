
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

pub fn eval_in_engine(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, contents: &str) -> String {
  unsafe {
    rooted!(in(cx) let mut rval = UndefinedValue());
    rt.evaluate_script(global.handle(), contents, "test", 1, rval.handle_mut());
    return stringify_jsvalue(cx, &rval)
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

pub fn get_attribute(node: &Rc<Node>, key: &str) -> Option<String> {
  match node.data.borrow() {
    NodeData::Element { attrs, .. } =>{
      let attributes : Ref<Vec<html5ever::interface::Attribute>> = attrs.borrow();

      for attr in attributes.iter() {
        println!("{:?}", attr);
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

pub fn render_children(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, node: &mut Rc<Node>) -> CondGenFlags {
    match node.data.borrow() {
      NodeData::Element { name, attrs, .. } => {
          println!("element name: {:?}", name);

          if name.local.to_string() == "script" {
            if get_attribute(&node, "ssr") == Some("true".to_string()) {
              let script = inner_text(node);
              eval_in_engine(&global, &rt, cx, &script);
            }
          }

          let mut attributes : RefMut<Vec<html5ever::interface::Attribute>> = attrs.borrow_mut();

          for attr in attributes.iter_mut() {
            println!("{:?}", attr);
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
            }
          }

          // attributes.push(html5ever::interface::Attribute {
          //   name: QualName::new(None, "".into(), "x-generated-ssr".into()),
          //   value: "true".into()
          // });

          let mut children = node.children.borrow_mut();

          for item in children.iter_mut() {
            render_children(global, rt, cx, item);
          }
          
          return CondGenFlags::default()
      },
      NodeData::Comment { .. } => {
        println!("is comment");

        return CondGenFlags::default()
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
      NodeData::Doctype { .. } => {
        println!("is doctype");
        return CondGenFlags::default()
      },
      NodeData::ProcessingInstruction { .. } => {
        println!("is processing instruction");
        return CondGenFlags::default();
      },
      NodeData::Document { .. } => {
        println!("is document!");

        let mut children = node.children.borrow_mut();

        for item in children.iter_mut() {
          render_children(global, rt, cx, item);
        }

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


