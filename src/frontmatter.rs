use mozjs::jsval::UndefinedValue;

use mozjs::conversions::ToJSValConvertible;
use mozjs::jsval::JSVal;
use serde_yaml::Value as YAMLValue;
use mozjs::jsapi::Value as JSValue;
use mozjs::jsapi::JSContext;
use mozjs::rust::MutableHandleValue;
use mozjs::rust::SIMPLE_GLOBAL_CLASS;

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
    NotFound
}

type Alias = YAMLValue;


trait EasyToJSVal {
    unsafe fn convert_to_jsval(&self, cx: *mut JSContext) -> JSValue;
}

impl EasyToJSVal for ToJSValConvertible {
    unsafe fn convert_to_jsval(&self, cx: *mut JSContext) -> JSValue {
        rooted!(in(cx) let mut val = UndefinedValue());
        self.to_jsval(cx, val.handle_mut());
        val.get()
    }
}

impl ToJSValConvertible for EasyToJSVal {
    unsafe fn to_jsval(&self, cx: *mut JSContext, mut rval: MutableHandleValue) {
        let val = self.convert_to_jsval(cx);
        rval.set(val);
    }
}

impl<T: ToJSValConvertible> EasyToJSVal for &Vec<T> {
    unsafe fn convert_to_jsval(&self, cx: *mut JSContext) -> JSValue {
        rooted!(in(cx) let mut val = UndefinedValue());
        self.to_jsval(cx, val.handle_mut());
        val.get()
    }
}

// impl <T: EasyToJSVal> EasyToJSVal for Vec<T> {
//     unsafe fn convert_to_jsval(&self, cx: *mut JSContext) -> JSValue {
//         rooted!(in(cx) let mut val = UndefinedValue());
//         self.to_jsval(cx, val.handle_mut());
//         val.get()
//     }
// }

impl EasyToJSVal for YAMLValue {
    unsafe fn convert_to_jsval(&self, cx: *mut JSContext) -> JSValue {
        match self {
            YAMLValue::Null => {
                mozjs::jsval::NullValue()
            },
            YAMLValue::String(string) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                string.to_jsval(cx, val.handle_mut());
                val.get()
            },
            YAMLValue::Bool(boolean) => {
                rooted!(in(cx) let mut val = UndefinedValue());
                boolean.to_jsval(cx, val.handle_mut());
                val.get()
            },
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
            },
            YAMLValue::Sequence(vector) => {
                let items = vector.iter().map(|item| {
                    item.convert_to_jsval(cx)
                }).collect::<Vec<JSValue>>();

                rooted!(in(cx) let mut val = UndefinedValue());
                items.to_jsval(cx, val.handle_mut());
                val.get()
            },
            YAMLValue::Mapping(mapping) => {
                let obj = mozjs::jsapi::JS_NewObject(cx, &SIMPLE_GLOBAL_CLASS);
                for (k,v) in mapping.iter() {
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
                                value.handle()
                            );
                        },
                        _ => unimplemented!()
                    }
                }

                mozjs::jsval::ObjectValue(obj)
            }
        }
    }
}

/// Front matter is defined by the block at the top of a document, separated by triple dashes "---"
pub fn extract_frontmatter(document: &str) ->  impl ToJSValConvertible {
    unimplemented!()
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
    fn test_infer_json () {
        let doc = r###"---
        {
            "title": "JSON Example"
        }
        ---"###;

        assert_eq!(crate::frontmatter::infer_type(&doc), crate::frontmatter::MatterType::JSON);
    }

        #[test]
    fn test_infer_yaml () {
        let doc = r###"---
        title: YAML Front Matter
        ---"###;

        assert_eq!(crate::frontmatter::infer_type(&doc), crate::frontmatter::MatterType::YAML);
    }

    #[test]
    fn test_infer_none () {
        let doc = r###"some article begins right here. nothing to see."###;

        assert_eq!(crate::frontmatter::infer_type(&doc), crate::frontmatter::MatterType::NotFound);
    }
}