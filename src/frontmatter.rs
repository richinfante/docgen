use mozjs::jsval::UndefinedValue;

use cargo_toml::Value as TOMLValue;
use mozjs::conversions::ToJSValConvertible;
use mozjs::jsapi::JSContext;
use mozjs::jsapi::Value as JSValue;
use mozjs::jsval::JSVal;
use mozjs::rust::MutableHandleValue;
use mozjs::rust::SIMPLE_GLOBAL_CLASS;
use serde_json::Value as JSONValue;
use serde_yaml::Value as YAMLValue;

// pub enum Value {
//     /// Represents a YAML null value.
//     Null,
//     /// Represents a YAML boolean.
//     Bool(bool),
//     /// Represents a YAML numerical value, whether integer or floating point.
//     Number(Number),
//     /// Represents a YAML string.
//     String(String),
//     /// Represents a YAML sequence in which the elements are
//     /// `serde_yaml::Value`.
//     Sequence(Sequence),
//     /// Represents a YAML mapping in which the keys and values are both
//     /// `serde_yaml::Value`.
//     Mapping(Mapping),
// }

#[derive(Debug, PartialEq)]
pub enum MatterType {
    JSON,
    YAML,
    NotFound,
}

pub trait EasyToJSVal {
    unsafe fn convert_to_jsval(&self, cx: *mut JSContext) -> JSValue;
}

/// Bindings to JSVal for TOMLValue via ToJSValConvertible
impl EasyToJSVal for TOMLValue {
    unsafe fn convert_to_jsval(&self, cx: *mut JSContext) -> JSValue {
        match self {
            TOMLValue::Boolean(boolean) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                boolean.to_jsval(cx, val.handle_mut());
                val.get()
            }
            TOMLValue::String(string) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                string.to_jsval(cx, val.handle_mut());
                val.get()
            }
            TOMLValue::Float(number) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                number.to_jsval(cx, val.handle_mut());
                val.get()
            }
            TOMLValue::Integer(number) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                number.to_jsval(cx, val.handle_mut());
                val.get()
            }
            TOMLValue::Array(vector) => {
                let items = vector
                    .iter()
                    .map(|item| item.convert_to_jsval(cx))
                    .collect::<Vec<JSValue>>();

                rooted!(in(cx) let mut val = UndefinedValue());
                items.to_jsval(cx, val.handle_mut());
                val.get()
            }
            TOMLValue::Table(mapping) => {
                let obj = mozjs::jsapi::JS_NewObject(cx, &SIMPLE_GLOBAL_CLASS);
                for (string, v) in mapping.iter() {
                    let page_name = std::ffi::CString::new(string.as_str()).unwrap();
                    let page_ptr = page_name.as_ptr() as *const i8;
                    rooted!(in(cx) let object = mozjs::jsval::ObjectValue(obj).to_object());
                    rooted!(in(cx) let value = v.convert_to_jsval(cx));
                    mozjs::rust::wrappers::JS_SetProperty(
                        cx,
                        object.handle(),
                        page_ptr,
                        value.handle(),
                    );
                }

                mozjs::jsval::ObjectValue(obj)
            }
            _ => unimplemented!(),
        }
    }
}

/// Bindings to JSVal for JSONValue via ToJSValConvertible
impl EasyToJSVal for JSONValue {
    unsafe fn convert_to_jsval(&self, cx: *mut JSContext) -> JSValue {
        match self {
            JSONValue::Bool(boolean) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                boolean.to_jsval(cx, val.handle_mut());
                val.get()
            }
            JSONValue::Null => mozjs::jsval::NullValue(),
            JSONValue::String(string) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                string.to_jsval(cx, val.handle_mut());
                val.get()
            }
            JSONValue::Number(number) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                if number.is_f64() {
                    number.as_f64().unwrap().to_jsval(cx, val.handle_mut());
                } else if number.is_u64() {
                    number.as_u64().unwrap().to_jsval(cx, val.handle_mut());
                } else if number.is_i64() {
                    number.as_i64().unwrap().to_jsval(cx, val.handle_mut());
                } else {
                    panic!("unknown number type.");
                }

                val.get()
            }
            JSONValue::Array(vector) => {
                let items = vector
                    .iter()
                    .map(|item| item.convert_to_jsval(cx))
                    .collect::<Vec<JSValue>>();

                rooted!(in(cx) let mut val = UndefinedValue());
                items.to_jsval(cx, val.handle_mut());
                val.get()
            }
            JSONValue::Object(mapping) => {
                let obj = mozjs::jsapi::JS_NewObject(cx, &SIMPLE_GLOBAL_CLASS);
                for (string, v) in mapping.iter() {
                    let page_name = std::ffi::CString::new(string.as_str()).unwrap();
                    let page_ptr = page_name.as_ptr() as *const i8;
                    rooted!(in(cx) let object = mozjs::jsval::ObjectValue(obj).to_object());
                    rooted!(in(cx) let value = v.convert_to_jsval(cx));
                    mozjs::rust::wrappers::JS_SetProperty(
                        cx,
                        object.handle(),
                        page_ptr,
                        value.handle(),
                    );
                }

                mozjs::jsval::ObjectValue(obj)
            }
            _ => unimplemented!(),
        }
    }
}

/// Bindings to JSVal for YAMLValue via ToJSValConvertible
impl EasyToJSVal for YAMLValue {
    unsafe fn convert_to_jsval(&self, cx: *mut JSContext) -> JSValue {
        match self {
            YAMLValue::Null => mozjs::jsval::NullValue(),
            YAMLValue::String(string) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                string.to_jsval(cx, val.handle_mut());
                val.get()
            }
            YAMLValue::Bool(boolean) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                boolean.to_jsval(cx, val.handle_mut());
                val.get()
            }
            YAMLValue::Number(number) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                if number.is_f64() {
                    number.as_f64().unwrap().to_jsval(cx, val.handle_mut());
                } else if number.is_u64() {
                    number.as_u64().unwrap().to_jsval(cx, val.handle_mut());
                } else if number.is_i64() {
                    number.as_i64().unwrap().to_jsval(cx, val.handle_mut());
                } else {
                    panic!("unknown number type.");
                }

                val.get()
            }
            YAMLValue::Sequence(vector) => {
                let items = vector
                    .iter()
                    .map(|item| item.convert_to_jsval(cx))
                    .collect::<Vec<JSValue>>();

                rooted!(in(cx) let mut val = UndefinedValue());
                items.to_jsval(cx, val.handle_mut());
                val.get()
            }
            YAMLValue::Mapping(mapping) => {
                let obj = mozjs::jsapi::JS_NewObject(cx, &SIMPLE_GLOBAL_CLASS);
                for (k, v) in mapping.iter() {
                    match k {
                        YAMLValue::String(string) => {
                            let page_name = std::ffi::CString::new(string.as_str()).unwrap();
                            let page_ptr = page_name.as_ptr() as *const i8;
                            rooted!(in(cx) let object = mozjs::jsval::ObjectValue(obj).to_object());
                            rooted!(in(cx) let value = v.convert_to_jsval(cx));
                            mozjs::rust::wrappers::JS_SetProperty(
                                cx,
                                object.handle(),
                                page_ptr,
                                value.handle(),
                            );
                        }
                        _ => unimplemented!(),
                    }
                }

                mozjs::jsval::ObjectValue(obj)
            }
        }
    }
}

/// Front matter is defined by the block at the top of a document, separated by triple dashes "---"
pub fn extract_frontmatter(document: &str) -> (Option<&str>, &str) {
    let mut trimmed = document.trim();

    if !trimmed.starts_with("---\n") {
        return (None, document);
    }

    trimmed = trimmed[3..].trim();
    let mut was_newline = false;
    let mut dash_counter = 0;
    for (i, c) in trimmed.chars().enumerate() {
        if was_newline && c == '-' {
            dash_counter += 1;
        } else if c == '\n' {
            was_newline = true;
        } else {
            was_newline = false;
        }

        // println!("{}: {} - {}, {}", i, c, dash_counter, was_newline);
        if dash_counter == 3 && was_newline {
            return (Some(&trimmed[..i - 3]), &trimmed[i + 1..]);
        }
    }

    return (None, document);
}

pub fn infer_type(matter: &str) -> MatterType {
    // 1. trim whitespace.
    let trimmed = matter.trim();

    // 2. If trimmed starts with marker, advance and begin inference.
    if trimmed.starts_with("---\n") {
        if trimmed[3..].trim().starts_with("{") {
            MatterType::JSON
        } else {
            MatterType::YAML
        }
    } else {
        MatterType::NotFound
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_infer_json() {
        let doc = r###"
        ---
        {
            "title": "JSON Example"
        }
        ---"###;

        assert_eq!(
            crate::frontmatter::infer_type(&doc),
            crate::frontmatter::MatterType::JSON
        );
    }

    #[test]
    fn test_infer_yaml() {
        let doc = r###"---
        title: YAML Front Matter
        ---"###;

        assert_eq!(
            crate::frontmatter::infer_type(&doc),
            crate::frontmatter::MatterType::YAML
        );
    }

    #[test]
    fn test_infer_none() {
        let doc = r###"some article begins right here. nothing to see."###;

        assert_eq!(
            crate::frontmatter::infer_type(&doc),
            crate::frontmatter::MatterType::NotFound
        );
    }

    #[test]
    fn test_extract_matter() {
        let doc = r###"---
title: YAML Front Matter
description: Foo
other: Bar
---"###;

        assert_eq!(
            crate::frontmatter::extract_frontmatter(doc),
            (
                Some(
                    r###"title: YAML Front Matter
description: Foo
other: Bar"###
                ),
                ""
            )
        );
    }
}
