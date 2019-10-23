#[macro_use]
extern crate mozjs;

use serde_json::Value;

use regex::{Captures, Regex};

use html5ever::driver::ParseOpts;
use html5ever::interface::QualName;
use html5ever::rcdom::RcDom;
use html5ever::rcdom::{Node, NodeData};

use html5ever::interface::Attribute;
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{parse_document, serialize};

use mozjs::jsapi;
use mozjs::jsapi::CallArgs;
use mozjs::jsapi::CompartmentOptions;
use mozjs::jsapi::JSContext;
use mozjs::jsapi::JSString;
use mozjs::jsapi::JS_NewGlobalObject;
use mozjs::jsapi::OnNewGlobalHookOption;
use mozjs::jsval::JSVal;
use mozjs::jsval::UndefinedValue;
pub use mozjs::rust::{JSEngine, ParentRuntime, Runtime, SIMPLE_GLOBAL_CLASS};

#[macro_use]
extern crate log;

use std::borrow::Borrow;
use std::cell::RefCell;
use std::cell::{Ref, RefMut};
use std::sync::Arc;

use std::default::Default;
use std::ffi::CStr;

use std::cell::Cell;
use std::ptr;
use std::rc::Rc;
use std::rc::Weak;

pub mod frontmatter;
pub mod render;

use frontmatter::EasyToJSVal;

unsafe extern "C" fn console_log(
    context: *mut JSContext,
    argc: u32,
    vp: *mut mozjs::jsapi::Value,
) -> bool {
    let args = CallArgs::from_vp(vp, argc);

    let mut arg_strings: Vec<String> = vec![];

    for i in 0..argc {
        let arg = mozjs::rust::Handle::from_raw(args.get(i));
        let js = mozjs::rust::ToString(context, arg);
        let message = mozjs::conversions::jsstr_to_string(context, js);
        arg_strings.push(message);
    }

    info!("console.log: {}", arg_strings.join(" "));

    return true;
}

unsafe extern "C" fn console_error(
    context: *mut JSContext,
    argc: u32,
    vp: *mut mozjs::jsapi::Value,
) -> bool {
    let args = CallArgs::from_vp(vp, argc);

    let mut arg_strings: Vec<String> = vec![];

    for i in 0..argc {
        let arg = mozjs::rust::Handle::from_raw(args.get(i));
        let js = mozjs::rust::ToString(context, arg);
        let message = mozjs::conversions::jsstr_to_string(context, js);
        arg_strings.push(message);
    }

    error!("console.error: {}", arg_strings.join(" "));

    return true;
}

unsafe extern "C" fn console_warn(
    context: *mut JSContext,
    argc: u32,
    vp: *mut mozjs::jsapi::Value,
) -> bool {
    let args = CallArgs::from_vp(vp, argc);

    let mut arg_strings: Vec<String> = vec![];

    for i in 0..argc {
        let arg = mozjs::rust::Handle::from_raw(args.get(i));
        let js = mozjs::rust::ToString(context, arg);
        let message = mozjs::conversions::jsstr_to_string(context, js);
        arg_strings.push(message);
    }

    warn!("console.warn: {}", arg_strings.join(" "));

    return true;
}

unsafe extern "C" fn console_debug(
    context: *mut JSContext,
    argc: u32,
    vp: *mut mozjs::jsapi::Value,
) -> bool {
    let args = CallArgs::from_vp(vp, argc);

    let mut arg_strings: Vec<String> = vec![];

    for i in 0..argc {
        let arg = mozjs::rust::Handle::from_raw(args.get(i));
        let js = mozjs::rust::ToString(context, arg);
        let message = mozjs::conversions::jsstr_to_string(context, js);
        arg_strings.push(message);
    }

    debug!("console.debug: {}", arg_strings.join(" "));

    return true;
}

unsafe extern "C" fn read_file_to_string(
    context: *mut JSContext,
    argc: u32,
    vp: *mut mozjs::jsapi::Value,
) -> bool {
    let args = CallArgs::from_vp(vp, argc);

    let mut arg_strings: Vec<String> = vec![];

    let arg = mozjs::rust::Handle::from_raw(args.get(0));
    let js = mozjs::rust::ToString(context, arg);
    let script_src = mozjs::conversions::jsstr_to_string(context, js);

    let loaded_script_file = std::fs::read_to_string(std::path::Path::new(&script_src)).unwrap();

    rooted!(in(context) let mut val = UndefinedValue());
    loaded_script_file.to_jsval(context, val.handle_mut());

    args.rval().set(val.get());

    debug!("fs.readFileSync(): {}", script_src);

    return true;
}

/// Stringify a jsval into a rust string for injection into the template
unsafe fn stringify_jsvalue(cx: *mut JSContext, rval: &JSVal) -> String {
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

    return "(error: stringify_jsvalue unimplemented type!)".to_string();
}

/// Boolean check for a jsval.
/// This is used for x-if.
unsafe fn boolify_jsvalue(cx: *mut JSContext, rval: &JSVal) -> bool {
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
    global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>,
    rt: &Runtime,
    cx: *mut JSContext,
    filename: Option<&str>,
    contents: &str,
) -> String {
    rooted!(in(cx) let mut rval = UndefinedValue());
    let res = rt.evaluate_script(
        global.handle(),
        contents,
        filename.unwrap_or("eval"),
        1,
        rval.handle_mut(),
    );

    if !res.is_ok() {
        print_exception(&rt, cx);
        panic!("Error evaluating: {}, {:?}", contents, res);
    }

    unsafe {
        return stringify_jsvalue(cx, &rval);
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

/// Flags used for conditional node generation
pub struct CondGenFlags {
    remove: bool,
    replace: Option<Vec<Rc<Node>>>,
}

impl CondGenFlags {
    /// Set up the default flags
    pub fn default() -> CondGenFlags {
        CondGenFlags {
            remove: false,
            replace: None,
        }
    }
}

/// Get the inner text of a node.
/// This allows us to do things like <script> elements, etc.
pub fn inner_text(node: &Rc<Node>) -> String {
    match node.data.borrow() {
        NodeData::Text { contents } => {
            let tendril = contents.borrow();
            format!("{}", tendril)
        }
        _ => {
            let children = node.children.borrow();

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
        NodeData::Element { attrs, .. } => {
            let attributes: Ref<Vec<html5ever::interface::Attribute>> = attrs.borrow();

            for attr in attributes.iter() {
                let name = &attr.name.local.to_string();

                if name == key {
                    return Some(String::from(&attr.value));
                }
            }

            return None;
        }
        _ => None,
    }
}

/// Clone the data inside of a node.
fn clone_node_data(data: &NodeData) -> NodeData {
    match data {
        NodeData::Document => NodeData::Document,
        NodeData::Text { contents } => {
            let tendril = contents.borrow();
            NodeData::Text {
                contents: RefCell::new(tendril.try_subtendril(0, tendril.len32()).unwrap()),
            }
        }
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => NodeData::Doctype {
            name: name.try_subtendril(0, name.len32()).unwrap(),
            public_id: public_id.try_subtendril(0, public_id.len32()).unwrap(),
            system_id: system_id.try_subtendril(0, system_id.len32()).unwrap(),
        },
        NodeData::Comment { contents } => NodeData::Comment {
            contents: contents.try_subtendril(0, contents.len32()).unwrap(),
        },
        NodeData::ProcessingInstruction { target, contents } => NodeData::ProcessingInstruction {
            target: target.try_subtendril(0, target.len32()).unwrap(),
            contents: contents.try_subtendril(0, contents.len32()).unwrap(),
        },
        NodeData::Element {
            name,
            attrs,
            template_contents,
            mathml_annotation_xml_integration_point,
        } => NodeData::Element {
            name: name.clone(),
            attrs: RefCell::new(
                attrs
                    .borrow()
                    .iter()
                    .map(|v| Attribute {
                        name: v.name.clone(),
                        value: v.value.try_subtendril(0, v.value.len32()).unwrap(),
                    })
                    .collect::<Vec<Attribute>>(),
            ),
            template_contents: template_contents.clone(),
            mathml_annotation_xml_integration_point: *mathml_annotation_xml_integration_point,
        },
    }
}

fn deep_clone(old_node: &Rc<Node>, parent: Option<Weak<Node>>) -> Rc<Node> {
    use std::borrow::BorrowMut;
    let mut node = Rc::new(Node {
        children: RefCell::new(vec![]),
        data: clone_node_data(old_node.data.borrow()),
        parent: Cell::new(parent),
    });

    let new: Vec<Rc<Node>>;
    let weak = std::rc::Rc::downgrade(&node);
    {
        new = old_node
            .children
            .borrow()
            .iter()
            .map(|v| deep_clone(v, Some(weak.clone())))
            .collect::<Vec<Rc<Node>>>();
    }

    let node_mut = node.borrow_mut();
    node_mut.children.replace(new);

    trace!(
        "deep clone info. {} kids had, {} kids found.",
        old_node.children.borrow().len(),
        node_mut.children.borrow().len()
    );

    node
}

use mozjs::conversions::ToJSValConvertible;

pub fn parse_for_notation(notation: &str) -> (String, String, String) {
    let simple_in_of = Regex::new(r###"^([a-zA-Z$_]+) (in|of) (.*)$"###).unwrap();

    match simple_in_of.captures(notation) {
        Some(captures) => {
            let name = &captures[1];
            let kind = &captures[2];
            let expr = &captures[3];
            return (name.to_string(), kind.to_string(), expr.to_string());
        }
        _ => panic!("unsupported for-loop notation: {}", notation),
    }
}

pub struct RenderContext {
    /// Contains a mapping of slot contents to the contexts for them.
    pub slots: std::collections::HashMap<String, Vec<Rc<RefCell<RenderContext>>>>,

    pub contents: Rc<html5ever::rcdom::Node>,

    pub children: Vec<Rc<RefCell<RenderContext>>>,
}

impl RenderContext {
    fn new_with_subcontext(
        contents: Rc<html5ever::rcdom::Node>,
        context: Option<Rc<RefCell<RenderContext>>>,
    ) -> RenderContext {
        if let Some(context) = context {
            RenderContext {
                slots: std::collections::HashMap::new(),
                contents,
                children: vec![context],
            }
        } else {
            RenderContext::new(contents)
        }
    }

    fn new(contents: Rc<html5ever::rcdom::Node>) -> RenderContext {
        RenderContext {
            slots: std::collections::HashMap::new(),
            contents,
            children: vec![],
        }
    }

    fn find_slot_contents(&self, slot: &str) -> Vec<Rc<RefCell<RenderContext>>> {
        let mut items: Vec<Rc<RefCell<RenderContext>>> = vec![];

        if let Some(vals) = self.slots.get(slot) {
            for item in vals {
                items.push(item.clone());
            }
        }

        for child in self.children.iter() {
            let child2: Rc<RefCell<RenderContext>> = child.clone();
            let ch = child2.borrow_mut();
            for item in ch.find_slot_contents(slot) {
                items.push(item.clone());
            }
        }

        return items;
    }
}

/// Render the children of a node, recursively.
unsafe fn render_children(
    global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>,
    rt: &Runtime,
    cx: *mut JSContext,
    node: &mut Rc<Node>,
    loop_expansion: bool,
    render_context: Rc<RefCell<RenderContext>>,
    slot_contents: Rc<Option<html5ever::rcdom::RcDom>>,
) -> CondGenFlags {
    let flags = CondGenFlags::default();

    match node.data.borrow() {
        NodeData::Document { .. } => {
            let mut children = node.children.borrow_mut();

            for item in children.iter_mut() {
                render_children(
                    global,
                    rt,
                    cx,
                    item,
                    true,
                    render_context.clone(),
                    slot_contents.clone(),
                );
            }

            return CondGenFlags::default();
        }
        NodeData::Element { name, attrs, .. } => {
            let node_name = name.local.to_string();
            debug!("-> enter element: {:?}", node_name);

            if node_name == "h1"
                || node_name == "h2"
                || node_name == "h3"
                || node_name == "h4"
                || node_name == "h5"
                || node_name == "h6"
            {
                // If id is not set, assign the ID of the <h> element for linking
                if get_attribute(&node, "id").is_none() {
                    let content_text = inner_text(node);
                    let mut attributes: RefMut<Vec<html5ever::interface::Attribute>> =
                        attrs.borrow_mut();
                    let attribute = html5ever::interface::Attribute {
                        name: QualName::new(None, "".into(), "id".into()),
                        value: content_text.to_lowercase().into(),
                    };
                    attributes.push(attribute);
                }
            }
            if name.local.to_string() == "script" {
                if get_attribute(&node, "static").is_some() {
                    let script = inner_text(node);
                    eval_in_engine(&global, &rt, cx, Some("inline_script"), &script);

                    // If it has a src="" attribute, load to string and execute.
                    if let Some(script_src) = get_attribute(&node, "src") {
                        let loaded_script_file =
                            std::fs::read_to_string(std::path::Path::new(&script_src)).unwrap();
                        eval_in_engine(&global, &rt, cx, Some(&script_src), &loaded_script_file);
                    }

                    return CondGenFlags {
                        remove: true,
                        replace: None,
                    };
                }
            }

            let mut needs_expansion: Option<String> = None;
            let mut replacements: Vec<Rc<Node>> = vec![];
            let mut needs_remove: bool = false;
            let mut needs_fork: bool = false;
            let mut final_attrs: Vec<html5ever::interface::Attribute> = vec![];
            let mut loop_name = get_attribute(node, "x-as").unwrap_or("item".to_string());
            let mut index_name = get_attribute(node, "x-index").unwrap_or("i".to_string());
            debug!(
                "{}: slot={:?}, name={:?}",
                node_name,
                get_attribute(node, "slot"),
                get_attribute(node, "name")
            );
            if let Some(val) = get_attribute(node, "slot") {
                debug!("found element named {} to add to {}", node_name, val);

                {
                    let mut inner = render_context.borrow_mut();
                    inner
                        .slots
                        .entry(val)
                        .or_insert_with(Vec::new)
                        .push(Rc::new(RefCell::new(RenderContext::new(node.clone()))));
                }

                return CondGenFlags {
                    remove: true,
                    replace: None,
                };
            } else if (node_name == "slot" && get_attribute(node, "name").is_some()) {
                let slot_name = get_attribute(node, "name").unwrap();

                if slot_name == "content" {
                    if let Some(contents) = slot_contents.clone().borrow() {
                        trace!("swapping slot contents into dom tree.");
                        let doc: &Node = contents.document.borrow();
                        let children: Ref<Vec<Rc<Node>>> = doc.children.borrow();
                        trace!("got {} childen to doc", children.len());
                        if children.len() == 0 {
                            return CondGenFlags {
                                remove: true,
                                replace: None,
                            };
                        }
                        let html: &Node = children[children.len() - 1].borrow();
                        let html_children = html.children.borrow();
                        trace!("got {} childen to html", html_children.len());
                        if html_children.len() == 0 {
                            return CondGenFlags {
                                remove: true,
                                replace: None,
                            };
                        }
                        let body: &Node = html_children[html_children.len() - 1].borrow();
                        let children = body.children.borrow();
                        // let items : &Vec<std::rc::Rc<html5ever::rcdom::Node>> = children.as_ref();
                        return CondGenFlags {
                            remove: true,
                            replace: Some(
                                children
                                    .iter()
                                    .map(|cn| deep_clone(cn, Some(std::rc::Rc::downgrade(node))))
                                    .collect(),
                            ),
                        };
                    }
                } else {
                    // let slot : &RenderContext = ;
                    let found_slot_contents =
                        render_context.borrow_mut().find_slot_contents(&slot_name);

                    if found_slot_contents.len() > 0 {
                        debug!("-> found slot contents: {}", &slot_name);
                        // let items : &Vec<std::rc::Rc<html5ever::rcdom::Node>> = children.as_ref();
                        return CondGenFlags {
                            remove: true,
                            replace: Some(
                                found_slot_contents
                                    .iter()
                                    .map(|v| v.borrow_mut().contents.clone())
                                    .collect(),
                            ),
                        };
                    } else {
                        return CondGenFlags {
                            remove: true,
                            replace: None,
                        };
                    }
                }
            }
            {
                let mut attributes: RefMut<Vec<html5ever::interface::Attribute>> =
                    attrs.borrow_mut();

                for attr in attributes.iter_mut() {
                    trace!("{:?}", attr);
                    let name = &attr.name.local.to_string();
                    let script = String::from(&attr.value);
                    if name.starts_with(":") && needs_expansion.is_none() {
                        let final_name = name[1..].to_string();
                        attr.name = QualName::new(None, "".into(), final_name.clone().into());
                        let value: JSVal = eval(&global, &rt, cx, &script).unwrap();

                        // support for boolean flag type attribute names
                        if final_name == "checked" // checkbox
                            || final_name == "selected" // select box
                            || final_name == "disabled" // input
                            || final_name == "readonly" // input 
                            || final_name == "autoplay" // video
                            || final_name == "controls" // video
                            || final_name == "loop" // video 
                            || final_name == "muted" // video
                            || final_name == "open" // summary
                        {
                            // only if it's a boolean, perform boolification of it for the conditional
                            if boolify_jsvalue(cx, &value) {
                                attr.value = String::new().into();
                                final_attrs.push(attr.clone());
                            }
                        } else {
                            attr.value = stringify_jsvalue(cx, &value).into();
                            final_attrs.push(attr.clone());
                        }
                    } else if name == "x-if" && needs_expansion.is_none() {
                        let included = eval_in_engine_bool(&global, &rt, cx, &script);
                        if !included {
                            return CondGenFlags {
                                remove: true,
                                replace: None,
                            };
                        }
                    } else if name == "x-each" && loop_expansion {
                        needs_expansion = Some(script);
                    } else if name == "x-for" && loop_expansion {
                        let (new_loop_name, _, expression) = parse_for_notation(&script);
                        loop_name = new_loop_name;
                        needs_expansion = Some(expression);
                    } else if (node_name == "slot" && name == "src") {
                        rooted!(in(cx) let child_global =
                        JS_NewGlobalObject(cx, &SIMPLE_GLOBAL_CLASS, ptr::null_mut(),
                                                    OnNewGlobalHookOption::FireOnNewGlobalHook,
                                                    &CompartmentOptions::default())
                        );

                        let _ac = mozjs::jsapi::JSAutoCompartment::new(cx, child_global.get());
                        assert!(mozjs::rust::wrappers::JS_InitStandardClasses(
                            cx,
                            global.handle()
                        ));

                        let page_name = std::ffi::CString::new("page").unwrap();
                        let page_ptr = page_name.as_ptr() as *const i8;
                        rooted!(in(cx) let val = mozjs::jsval::ObjectValue(child_global.get()));
                        mozjs::rust::wrappers::JS_SetProperty(
                            cx,
                            child_global.handle(),
                            page_ptr,
                            val.handle(),
                        );

                        let result_name = std::ffi::CString::new("parent").unwrap();
                        let result_name_ptr = result_name.as_ptr() as *const i8;
                        rooted!(in(cx) let val = mozjs::jsval::ObjectValue(global.get()));
                        mozjs::rust::wrappers::JS_SetProperty(
                            cx,
                            child_global.handle(),
                            result_name_ptr,
                            val.handle(),
                        );

                        let mut contents =
                            std::fs::read_to_string(std::path::Path::new(&script)).unwrap();
                        let (partial, child_render_context) = parse_and_render_dom(
                            &child_global,
                            &rt,
                            cx,
                            &mut contents,
                            None,
                            None,
                            std::rc::Rc::new(None),
                        );

                        {
                            let ccx = render_context.clone();
                            let mut mut_ctx = ccx.borrow_mut();
                            mut_ctx.children.push(child_render_context.clone());
                        }

                        let doc: &Node = partial.document.borrow();
                        let children: Ref<Vec<Rc<Node>>> = doc.children.borrow();
                        trace!("got {} childen to doc", children.len());
                        if children.len() == 0 {
                            continue;
                        }
                        let html: &Node = children[children.len() - 1].borrow();
                        let html_children = html.children.borrow();
                        trace!("got {} childen to html", html_children.len());
                        if html_children.len() == 0 {
                            continue;
                        }
                        let body: &Node = html_children[html_children.len() - 1].borrow();
                        // let body_children = &body.children;
                        // node.children.swap(&body_children);
                        let children = body.children.borrow();
                        // let items : &Vec<std::rc::Rc<html5ever::rcdom::Node>> = children.as_ref();
                        return CondGenFlags {
                            remove: true,
                            replace: Some(
                                children
                                    .iter()
                                    .map(|cn| deep_clone(cn, Some(std::rc::Rc::downgrade(node))))
                                    .collect(),
                            ),
                        };
                    } else if name == "x-as" || name == "x-index" {
                        // Do nothing.
                    } else {
                        final_attrs.push(attr.clone());
                    }
                }
            }

            attrs.replace(final_attrs);

            if let Some(script) = needs_expansion {
                if loop_expansion {
                    trace!("begin loop expansion on {:?}", name);
                    // 0. Run the script and setup replacement list.
                    let object = eval(&global, &rt, cx, &script).unwrap();

                    // 1. Symbol.Iterator
                    let sym = mozjs::jsapi::GetWellKnownSymbol(cx, jsapi::SymbolCode::iterator);
                    let id = mozjs::glue::RUST_SYMBOL_TO_JSID(sym);

                    if !object.is_object() {
                        trace!("eval returned non-object!");
                    }

                    // 2. object[Symbol.iterator]
                    rooted!(in(cx) let mut fnhandle = UndefinedValue());
                    rooted!(in(cx) let obj = object.to_object());
                    if !mozjs::rust::wrappers::JS_GetPropertyById(
                        cx,
                        obj.handle(),
                        mozjs::rust::Handle::new(&id),
                        fnhandle.handle_mut(),
                    ) {
                        panic!("Couldnt get iterator")
                    }

                    // 3. Call function to get an iterator.
                    rooted!(in(cx) let mut iterret = UndefinedValue());
                    rooted!(in(cx) let func = mozjs::rust::wrappers::JS_ValueToFunction(cx, fnhandle.handle()));
                    let args = mozjs::jsapi::HandleValueArray::new();
                    mozjs::rust::wrappers::JS_CallFunction(
                        cx,
                        obj.handle(),
                        func.handle(),
                        &args,
                        iterret.handle_mut(),
                    );

                    let mut loop_index: u64 = 0;

                    loop {
                        needs_remove = true;
                        trace!("iterating thing...");
                        // 4. Call the iterator.next() function
                        rooted!(in(cx) let returnvalue = UndefinedValue());
                        rooted!(in(cx) let iter_result = iterret.to_object());
                        let next_str = std::ffi::CString::new("next").unwrap();
                        let next_ptr = next_str.as_ptr() as *const i8;
                        let args = mozjs::jsapi::HandleValueArray::new();
                        rooted!(in(cx) let mut iteration_value = UndefinedValue());
                        mozjs::rust::wrappers::JS_CallFunctionName(
                            cx,
                            iter_result.handle(),
                            next_ptr,
                            &args,
                            iteration_value.handle_mut(),
                        );

                        if !iteration_value.is_object() {
                            trace!("iteration is not obj");
                            break;
                        }

                        // 5. Get result.value
                        rooted!(in(cx) let mut iteration_result_value = UndefinedValue());

                        if iteration_value.is_object() {
                            let c_str = std::ffi::CString::new("value").unwrap();
                            let ptr = c_str.as_ptr() as *const i8;
                            rooted!(in(cx) let iteration_value_obj = iteration_value.to_object());
                            mozjs::rust::wrappers::JS_GetProperty(
                                cx,
                                iteration_value_obj.handle(),
                                ptr,
                                iteration_result_value.handle_mut(),
                            );
                        }

                        // debug!("iter value -> {}", stringify_jsvalue(cx, &iteration_result_value));

                        // 6. Check if iterator is done
                        let done_str = std::ffi::CString::new("done").unwrap();
                        let done_str_ptr = done_str.as_ptr() as *const i8;
                        rooted!(in(cx) let iteration_value_obj = iteration_value.to_object());
                        rooted!(in(cx) let mut iteration_done_value = UndefinedValue());
                        mozjs::rust::wrappers::JS_GetProperty(
                            cx,
                            iteration_value_obj.handle(),
                            done_str_ptr,
                            iteration_done_value.handle_mut(),
                        );
                        trace!(
                            "iter done -> {}",
                            stringify_jsvalue(cx, &iteration_done_value)
                        );

                        // Boolify javascript
                        if boolify_jsvalue(cx, &iteration_done_value) {
                            trace!("is done. return.");
                            break;
                        }

                        // HACK: Store the value temporarily in the global scope.
                        // TODO: iterate and create new scopes with the values.
                        let result_name = std::ffi::CString::new(loop_name.clone()).unwrap();
                        let result_name_ptr = result_name.as_ptr() as *const i8;
                        rooted!(in(cx) let iter_val = iteration_result_value.clone());
                        mozjs::rust::wrappers::JS_SetProperty(
                            cx,
                            global.handle(),
                            result_name_ptr,
                            iter_val.handle(),
                        );

                        let index_name_str = std::ffi::CString::new(index_name.clone()).unwrap();
                        let index_name_ptr = index_name_str.as_ptr() as *const i8;
                        rooted!(in(cx) let mut index = UndefinedValue());
                        loop_index.to_jsval(cx, index.handle_mut());
                        mozjs::rust::wrappers::JS_SetProperty(
                            cx,
                            global.handle(),
                            index_name_ptr,
                            index.handle(),
                        );

                        // Mut and render
                        let x = node.parent.replace(None);
                        let parent: Option<Weak<Node>>;
                        if let Some(x) = x {
                            parent = Some(x.clone());
                        } else {
                            parent = None;
                        }

                        let mut expand_node = deep_clone(node, parent);
                        let mut should_append = true;
                        let mut final_expansion_attrs: Vec<html5ever::interface::Attribute> = vec![];
                        {
                            use std::borrow::BorrowMut;
                            {
                                let mut attributes: (RefCell<Vec<html5ever::interface::Attribute>>, RefMut<Vec<html5ever::interface::Attribute>>) =
                                match &expand_node.borrow_mut().data {
                                    NodeData::Element { attrs, .. } => {
                                        (attrs.clone(), attrs.borrow_mut())
                                    },
                                    _ => panic!("non element rendered in for")
                                };

                                for attr in attributes.1.iter_mut() {
                                    trace!("{:?}", attr);
                                    let name = &attr.name.local.to_string();
                                    let script = String::from(&attr.value);
                                    if name.starts_with(":") {
                                        let final_name = name[1..].to_string();
                                        attr.name = QualName::new(None, "".into(), final_name.clone().into());
                                        let value: JSVal = eval(&global, &rt, cx, &script).unwrap();

                                        // support for boolean flag type attribute names
                                        if final_name == "checked" // checkbox
                                            || final_name == "selected" // select box
                                            || final_name == "disabled" // input
                                            || final_name == "readonly" // input 
                                            || final_name == "autoplay" // video
                                            || final_name == "controls" // video
                                            || final_name == "loop" // video 
                                            || final_name == "muted" // video
                                            || final_name == "open" // summary
                                        {
                                            // only if it's a boolean, perform boolification of it for the conditional
                                            if boolify_jsvalue(cx, &value) {
                                                attr.value = String::new().into();
                                                final_expansion_attrs.push(attr.clone());
                                            } else {
                                                attr.value = String::new().into();
                                                attr.name.local = String::new().into();
                                            }
                                        } else {
                                            attr.value = stringify_jsvalue(cx, &value).into();
                                            final_expansion_attrs.push(attr.clone());
                                        }
                                    } else if name == "x-if" {
                                        let included = eval_in_engine_bool(&global, &rt, cx, &script);
                                        if !included {
                                            should_append = false;
                                        }
                                    } else if name == "x-each" && loop_expansion {
                                        // do nothing
                                    } else if name == "x-for" && loop_expansion {
                                        // do nothing
                                    } else if node_name == "slot" && name == "src" {
                                        panic!("cannot set slot inside iter");
                                    } else if name == "x-as" || name == "x-index" {
                                        // Do nothing.
                                    } else {
                                        final_expansion_attrs.push(attr.clone());
                                    }
                                }
                            }
                        }
                        if should_append {
                            // substitute rendered attributes into the node.
                            if let NodeData::Element { attrs, .. } = &expand_node.data {
                                // info!("{:#?}", attrs);
                                attrs.replace(final_expansion_attrs);
                                // info!("{:#?}", attrs);
                            }
                            render_children(
                                global,
                                rt,
                                cx,
                                &mut expand_node,
                                false,
                                render_context.clone(),
                                slot_contents.clone(),
                            );
                            replacements.push(expand_node);
                        }

                        // Increment iteration counter.
                        loop_index += 1;
                    }
                }
            }

            // attributes.push(html5ever::interface::Attribute {
            //   name: QualName::new(None, "".into(), "x-generated-ssr".into()),
            //   value: "true".into()
            // });

            if node_name != "script" {
                trace!("Rendering children...");
                let mut out_children: Vec<Rc<Node>> = vec![];

                {
                    let mut children = node.children.borrow_mut();
                    trace!("have {} children.", children.len());
                    for item in children.iter_mut() {
                        trace!("Rendering child...");
                        let flags = render_children(
                            global,
                            rt,
                            cx,
                            item,
                            true,
                            render_context.clone(),
                            slot_contents.clone(),
                        );

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

                debug!("-> leave element: {:?}", node_name);
                trace!(
                    "render done. injecting {} new child elements into node.",
                    out_children.len()
                );

                node.children.replace(out_children);
            }

            CondGenFlags {
                replace: Some(replacements),
                remove: needs_remove,
            }
        }
        NodeData::Text { contents } => {
            let mut tendril = contents.borrow_mut();
            // let contents : String = tendril.into();
            let x = format!("{}", tendril);

            let regex = Regex::new(r###"\{\{(.*?)\}\}"###).unwrap();

            let result = regex.replace_all(&x, |caps: &Captures| {
                if let Some(code) = caps.get(0) {
                    let string: &str = code.into();
                    trace!("render var: {}", string);
                    return eval_in_engine(&global, rt, cx, Some("variable_substitution"), string);
                }

                return "".to_string();
            });

            tendril.clear();
            tendril.try_push_bytes(result.as_bytes()).unwrap();
            return CondGenFlags::default();
        }
        NodeData::Comment { contents } => {
            let text = format!("{}", contents);
            let trimmed = text.trim();
            if trimmed.starts_with("slot:") {
                debug!("got slot processing instruction: '{}'", text);
                let x = trimmed.replace("slot:", "");
                let slot_name = x.trim();

                if slot_name == "content" {
                    if let Some(contents) = slot_contents.clone().borrow() {
                        trace!("slot: swapping slot contents into dom tree.");
                        let doc: &Node = contents.document.borrow();
                        let children: Ref<Vec<Rc<Node>>> = doc.children.borrow();
                        trace!("slot: got {} childen to doc", children.len());
                        if children.len() == 0 {
                            return CondGenFlags {
                                remove: true,
                                replace: None,
                            };
                        }
                        let html: &Node = children[children.len() - 1].borrow();
                        let html_children = html.children.borrow();
                        trace!("slot: got {} childen to html", html_children.len());
                        if html_children.len() == 0 {
                            return CondGenFlags {
                                remove: true,
                                replace: None,
                            };
                        }
                        let body: &Node = html_children[html_children.len() - 1].borrow();
                        let children = body.children.borrow();
                        // let items : &Vec<std::rc::Rc<html5ever::rcdom::Node>> = children.as_ref();
                        return CondGenFlags {
                            remove: true,
                            replace: Some(
                                children
                                    .iter()
                                    .map(|cn| deep_clone(cn, Some(std::rc::Rc::downgrade(node))))
                                    .collect(),
                            ),
                        };
                    }
                } else {
                    let found_slot_contents =
                        render_context.borrow_mut().find_slot_contents(&slot_name);

                    if found_slot_contents.len() > 0 {
                        debug!("slot: found slot contents: {}", &slot_name);
                        // let items : &Vec<std::rc::Rc<html5ever::rcdom::Node>> = children.as_ref();
                        return CondGenFlags {
                            remove: true,
                            replace: Some(
                                found_slot_contents
                                    .iter()
                                    .map(|v| v.borrow_mut().contents.clone())
                                    .collect(),
                            ),
                        };
                    } else {
                        return CondGenFlags {
                            remove: true,
                            replace: None,
                        };
                    }
                }
            }

            return CondGenFlags::default();
        }
        NodeData::Doctype { .. } => return CondGenFlags::default(),
        NodeData::ProcessingInstruction { .. } => {
            return CondGenFlags::default();
        }
    }
}

pub fn parse_and_render_dom(
    global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>,
    rt: &Runtime,
    cx: *mut JSContext,
    template: &mut String,
    _variables: Option<Value>,
    parent_render_context: Option<Rc<RefCell<RenderContext>>>,
    slot_contents: Rc<Option<html5ever::rcdom::RcDom>>,
) -> (html5ever::rcdom::RcDom, Rc<RefCell<RenderContext>>) {
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: false,
            ..Default::default()
        },
        ..Default::default()
    };

    // let dom2 = html5ever::parse_fragment(RcDom::default(), opts.clone(), QualName::new(None, "".into(), "div".into()), vec![])
    //     .from_utf8()
    //     .read_from(&mut template.as_bytes())
    //     .unwrap();

    let dom = parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut template.as_bytes())
        .unwrap();

    return render_dom(
        global,
        rt,
        cx,
        dom,
        _variables,
        parent_render_context,
        slot_contents,
    );
}

/// Render a template.
pub fn render_dom(
    global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>,
    rt: &Runtime,
    cx: *mut JSContext,
    dom: html5ever::rcdom::RcDom,
    _variables: Option<Value>,
    parent_render_context: Option<Rc<RefCell<RenderContext>>>,
    slot_contents: Rc<Option<html5ever::rcdom::RcDom>>,
) -> (html5ever::rcdom::RcDom, Rc<RefCell<RenderContext>>) {
    unsafe {
        // NOTE: this line is important, without it all JS_ calls seem to segfault.

        let spidermonkey_version = mozjs::jsapi::JS_GetImplementationVersion();
        let spidermonkey_version_c_str: &CStr = CStr::from_ptr(spidermonkey_version);
        let spidermonkey_version_str_slice: &str = spidermonkey_version_c_str.to_str().unwrap();

        // TODO: this is bad code.
        eval(
            &global,
            &rt,
            cx,
            &format!(
                r###"
                    docgen = {{
                        version: "{}",
                        spidermonkey_version: "{}"
                    }};
                "###,
                env!("CARGO_PKG_VERSION"),
                spidermonkey_version_str_slice
            ),
        )
        .unwrap();

        let render_context = Rc::new(RefCell::new(RenderContext::new_with_subcontext(
            dom.document.clone(),
            parent_render_context,
        )));

        {
            let document: &Node = dom.document.borrow();
            let mut docchildren: RefMut<Vec<Rc<Node>>> = document.children.borrow_mut();
            for a in docchildren.iter_mut() {
                render_children(
                    &global,
                    &rt,
                    cx,
                    a,
                    true,
                    render_context.clone(),
                    slot_contents.clone(),
                );
            }
        }

        (dom, render_context)
    }
}

pub fn init_runtime() -> (Runtime) {
    let engine = JSEngine::init().unwrap();
    Runtime::new(engine)
}

pub fn init_js() -> (Runtime, *mut JSContext) {
    let rt = init_runtime();
    let cx = rt.cx();
    (rt, cx)
}

pub fn render_injecting(
    global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>,
    rt: &Runtime,
    cx: *mut JSContext,
    template: &mut String,
    variables: Option<Value>,
    inject_dom: Rc<Option<html5ever::rcdom::RcDom>>,
) -> String {
    let (dom, render_context) =
        parse_and_render_dom(&global, rt, cx, template, variables, None, inject_dom);

    let mut buffer = vec![];

    serialize(&mut buffer, &dom.document, Default::default())
        .ok()
        .expect("serialization failed");

    return String::from_utf8(buffer).unwrap();
}

pub fn serialize_dom(dom: &RcDom) -> String {
    let mut buffer = vec![];

    serialize(&mut buffer, &dom.document, Default::default())
        .ok()
        .expect("serialization failed");

    return String::from_utf8(buffer).unwrap();
}

pub fn render2(
    global: &mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>,
    rt: &Runtime,
    cx: *mut JSContext,
    template: &mut String,
    variables: Option<Value>,
) -> String {
    render_injecting(&global, rt, cx, template, variables, Rc::new(None))
}

pub fn render_recursive(
    path: &std::path::Path,
    child_dom: Rc<Option<html5ever::rcdom::RcDom>>,
    child: Option<&mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>>,
    set_vars: Option<serde_json::Value>,
) -> String {
    let (rt, cx) = init_js();
    render_recursive_inner(&rt, cx, path, None, child_dom, child, set_vars)
}

pub fn print_exception(rt: &Runtime, cx: *mut JSContext) {
    unsafe {
        rooted!(in(cx) let mut exc = UndefinedValue());
        mozjs::rust::jsapi_wrapped::JS_GetPendingException(cx, &mut exc.handle_mut());
        if exc.is_object() {
            rooted!(in(cx) let exc_object = exc.to_object());
            let c_str = std::ffi::CString::new("message").unwrap();
            let ptr = c_str.as_ptr() as *const i8;
            rooted!(in(cx) let mut message_res = UndefinedValue());
            mozjs::rust::wrappers::JS_GetProperty(
                cx,
                exc_object.handle(),
                ptr,
                message_res.handle_mut(),
            );
            error!("Error: {}", stringify_jsvalue(cx, &message_res));
        }
    }
}
/// Perform a recursive render.
/// Attach parent global into jsengine if it exists
pub fn render_recursive_inner(
    rt: &Runtime,
    cx: *mut JSContext,
    path: &std::path::Path,
    parent_render_context: Option<Rc<RefCell<RenderContext>>>,
    child_dom: Rc<Option<html5ever::rcdom::RcDom>>,
    child: Option<&mozjs::rust::RootedGuard<'_, *mut mozjs::jsapi::JSObject>>,
    set_vars: Option<serde_json::Value>,
) -> String {
    let path_str = format!("{}", path.display());
    debug!("rendering path: {}", path.display());
    let mut contents = std::fs::read_to_string(&path).unwrap();

    unsafe {
        rooted!(in(cx) let global =
        JS_NewGlobalObject(cx, &SIMPLE_GLOBAL_CLASS, ptr::null_mut(),
                                    OnNewGlobalHookOption::FireOnNewGlobalHook,
                                    &CompartmentOptions::default())
        );

        let _ac = mozjs::jsapi::JSAutoCompartment::new(cx, global.get());
        assert!(mozjs::rust::wrappers::JS_InitStandardClasses(
            cx,
            global.handle()
        ));

        let function = mozjs::rust::wrappers::JS_DefineFunction(
            cx,
            global.handle(),
            b"console_log\0".as_ptr() as *const libc::c_char,
            Some(console_log),
            1,
            0,
        );
        assert!(!function.is_null());

        let function2 = mozjs::rust::wrappers::JS_DefineFunction(
            cx,
            global.handle(),
            b"console_error\0".as_ptr() as *const libc::c_char,
            Some(console_error),
            1,
            0,
        );
        assert!(!function2.is_null());

        let function3 = mozjs::rust::wrappers::JS_DefineFunction(
            cx,
            global.handle(),
            b"console_debug\0".as_ptr() as *const libc::c_char,
            Some(console_debug),
            1,
            0,
        );
        assert!(!function3.is_null());

        let function3 = mozjs::rust::wrappers::JS_DefineFunction(
            cx,
            global.handle(),
            b"console_warn\0".as_ptr() as *const libc::c_char,
            Some(console_warn),
            1,
            0,
        );
        assert!(!function3.is_null());

        let function4 = mozjs::rust::wrappers::JS_DefineFunction(
            cx,
            global.handle(),
            b"fs_readFileSync\0".as_ptr() as *const libc::c_char,
            Some(read_file_to_string),
            1,
            0,
        );
        assert!(!function4.is_null());

        eval(
            &global,
            &rt,
            cx,
            r###"
            var console = {};
            var fs = {};
            fs.readFileSync = fs_readFileSync;
            console.log = console_log;
            console.error = console_error;
            console.debug = console_debug;
            console.warn = console_warn;
            "###,
        )
        .unwrap();

        let result_name = std::ffi::CString::new("page").unwrap();
        let result_name_ptr = result_name.as_ptr() as *const i8;
        rooted!(in(cx) let val = mozjs::jsval::ObjectValue(global.get()));
        mozjs::rust::wrappers::JS_SetProperty(cx, global.handle(), result_name_ptr, val.handle());

        if let Some(child) = child {
            let result_name = std::ffi::CString::new("child").unwrap();
            let result_name_ptr = result_name.as_ptr() as *const i8;
            rooted!(in(cx) let val = mozjs::jsval::ObjectValue(child.get()));
            mozjs::rust::wrappers::JS_SetProperty(
                cx,
                global.handle(),
                result_name_ptr,
                val.handle(),
            );
        }

        if let Some(set_vars) = &set_vars {
            if let serde_json::Value::Object(map) = set_vars {
                for (key, value) in map.iter() {
                    // println!("Set value at {} to {:?}", key, value);
                    rooted!(in(cx) let val = value.convert_to_jsval(cx));
                    let fmname = std::ffi::CString::new(key.as_str()).unwrap();
                    let fmname_ptr = fmname.as_ptr() as *const i8;
                    mozjs::rust::wrappers::JS_SetProperty(
                        cx,
                        global.handle(),
                        fmname_ptr,
                        val.handle(),
                    );
                }
            }
        }

        let mut override_contents: Option<String> = None;
        if path_str.ends_with(".md") {
            match frontmatter::infer_type(&contents) {
                frontmatter::MatterType::YAML => {
                    if let (Some(matter), read_contents) =
                        frontmatter::extract_frontmatter(&contents)
                    {
                        override_contents = Some(read_contents.to_string());
                        let val: serde_yaml::Value = serde_yaml::from_str(matter).unwrap();

                        if let serde_yaml::Value::Mapping(mapping) = val {
                            for (k, value_to_set) in mapping.iter() {
                                if let serde_yaml::Value::String(string) = k {
                                    debug!("Set value at {} to {:?}", string, value_to_set);
                                    rooted!(in(cx) let val = value_to_set.convert_to_jsval(cx));
                                    let fmname = std::ffi::CString::new(string.as_str()).unwrap();
                                    let fmname_ptr = fmname.as_ptr() as *const i8;
                                    mozjs::rust::wrappers::JS_SetProperty(
                                        cx,
                                        global.handle(),
                                        fmname_ptr,
                                        val.handle(),
                                    );
                                } else {
                                    panic!("front matter keys must be strings.");
                                }
                            }
                        }

                        // HACK: copy the values into the "page" object.
                        // eval(&global, &rt, cx, "for (let i in frontmatter) { page[i] = frontmatter[i] }");
                    }
                }
                _ => {}
            }
            if let Some(override_contents) = override_contents {
                contents = override_contents;
            }
            let mut result = render::render_markdown(&contents);
            let (partial, child_render_context) = parse_and_render_dom(
                &global,
                &rt,
                cx,
                &mut result,
                None,
                parent_render_context,
                child_dom,
            );

            debug!("child render context info:");
            {
                use std::borrow::Borrow;
                let item: &RefCell<RenderContext> = child_render_context.borrow();
                let item2 = item.borrow();
                debug!(
                    "has {} slots: {:?}",
                    item2.slots.len(),
                    item2.slots.keys().collect::<Vec<&String>>()
                );
            }

            let c_str = std::ffi::CString::new("layout").unwrap();
            let ptr = c_str.as_ptr() as *const i8;
            rooted!(in(cx) let mut layout_result = UndefinedValue());
            mozjs::rust::wrappers::JS_GetProperty(
                cx,
                global.handle(),
                ptr,
                layout_result.handle_mut(),
            );
            debug!("layout -> {}", stringify_jsvalue(cx, &layout_result));

            if layout_result.is_string() {
                return render_recursive_inner(
                    &rt,
                    cx,
                    &std::path::Path::new(&stringify_jsvalue(cx, &layout_result)),
                    Some(child_render_context),
                    Rc::new(Some(partial)),
                    Some(&global),
                    set_vars,
                );
            } else {
                return serialize_dom(&partial);
            }
        } else if path_str.ends_with(".html") || path_str.ends_with(".htm") {
            // let output = render(&global, &rt, cx, &mut contents, None);
            debug!("-> render partial html.");
            let (partial, child_render_context) = parse_and_render_dom(
                &global,
                &rt,
                cx,
                &mut contents,
                None,
                parent_render_context,
                child_dom,
            );
            debug!("-> render partial html complete.");
            let c_str = std::ffi::CString::new("layout").unwrap();
            let ptr = c_str.as_ptr() as *const i8;
            rooted!(in(cx) let mut layout_result = UndefinedValue());
            mozjs::rust::wrappers::JS_GetProperty(
                cx,
                global.handle(),
                ptr,
                layout_result.handle_mut(),
            );
            debug!("-> layout -> {}", stringify_jsvalue(cx, &layout_result));

            if layout_result.is_string() {
                debug!("-> render recursive!");
                return render_recursive_inner(
                    &rt,
                    cx,
                    &std::path::Path::new(&stringify_jsvalue(cx, &layout_result)),
                    Some(child_render_context),
                    Rc::new(Some(partial)),
                    Some(&global),
                    set_vars,
                );
            } else {
                return serialize_dom(&partial);
            }
        } else {
            panic!("no way to parse file.");
        }
    }
}

#[test]
fn test_for_notation() {
    let (name, kind, expr) = parse_for_notation("x in abc.foo()");
    assert_eq!(name, "x");
    assert_eq!(kind, "in");
    assert_eq!(expr, "abc.foo()");
}

#[test]
fn test_for_of_notation() {
    let (name, kind, expr) = parse_for_notation("x_val of [1,2,3]");
    assert_eq!(name, "x_val");
    assert_eq!(kind, "of");
    assert_eq!(expr, "[1,2,3]");
}

#[test]
fn test_each_object_values_notation() {
    let (name, kind, expr) = parse_for_notation("item of Object.entries({ a: 1 })");
    assert_eq!(name, "item");
    assert_eq!(kind, "of");
    assert_eq!(expr, "Object.entries({ a: 1 })");
}
