
#[macro_use]
extern crate mozjs;

use std::collections::HashMap;
use serde_json::Value;


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
  } else if rval.is_string() {
    return mozjs::conversions::jsstr_to_string(cx, rval.to_string())
  } else if rval.is_undefined() {
    return "undefined".into()
  } else if rval.is_null() {
    return "null".into()
  }

  unimplemented!()
}

pub fn eval_in_engine(global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>, rt: &Runtime, cx: *mut JSContext, contents: String) -> String {
  unsafe {
    rooted!(in(cx) let mut rval = UndefinedValue());

    rt.evaluate_script(global.handle(), &contents, "test", 1, rval.handle_mut());
    return stringify_jsvalue(cx, &rval)
        
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
        let docchildren: Ref<Vec<Rc<Node>>> = document.children.borrow();
        println!("docchildren len: {}", docchildren.len());
        let html : &Node = docchildren[1].borrow();
        let htmlchildren: Ref<Vec<Rc<Node>>> = html.children.borrow();
        println!("htmlchildren len: {}", htmlchildren.len());
        let body : &Node = htmlchildren.borrow()[2].borrow(); // Implicit head element at children[0].
        let mut bodychildren : RefMut<Vec<Rc<Node>>> = body.children.borrow_mut();
        {
            for a in bodychildren.iter_mut() {

              match a.data.borrow() {
                NodeData::Element { name, attrs, .. } => {
                    println!("element name: {:?}", name);
                    let mut attributes : RefMut<Vec<html5ever::interface::Attribute>> = attrs.borrow_mut();

                    for attr in attributes.iter_mut() {
                      println!("{:?}", attr);
                      let name = &attr.name.local;
                      if name.to_ascii_lowercase().starts_with(":") {
                        attr.name = QualName::new(None, "".into(), name.to_ascii_lowercase()[1..].to_string().into());
                        let string = String::from(&attr.value);

                        attr.value = eval_in_engine(&global, &rt, cx, string).into();
                      }
                    }

                    attributes.push(html5ever::interface::Attribute {
                      name: QualName::new(None, "".into(), "x-generated-ssr".into()),
                      value: "true".into()
                    })
                },
                NodeData::Comment { .. } => {
                  println!("is comment")
                },
                NodeData::Text { contents } => {
                  let tendril = contents.borrow_mut();
                  // let contents = String::from(tendril);

                },
                NodeData::Doctype { .. } => {
                  println!("is doctype")
                },
                NodeData::ProcessingInstruction { .. } => {
                  println!("is processing instruction")
                },
                NodeData::Document { .. } => {
                  println!("is document!")
                },
                _ => {
                  println!("is other")
                }
              }
            }
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


