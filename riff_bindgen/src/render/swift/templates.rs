use askama::Template;

use super::plan::{
    SwiftCallMode, SwiftCallback, SwiftClass, SwiftEnum, SwiftField, SwiftFunction, SwiftRecord,
    SwiftStreamMode, SwiftVariant,
};

#[derive(Template)]
#[template(path = "preamble.txt", escape = "none")]
pub struct PreambleTemplate<'a> {
    pub prefix: &'a str,
    pub ffi_module_name: Option<&'a str>,
    pub has_async: bool,
    pub has_streams: bool,
}

impl<'a> PreambleTemplate<'a> {
    pub fn new(
        prefix: &'a str,
        ffi_module_name: Option<&'a str>,
        has_async: bool,
        has_streams: bool,
    ) -> Self {
        Self {
            prefix,
            ffi_module_name,
            has_async,
            has_streams,
        }
    }
}

pub fn render_preamble(
    prefix: &str,
    ffi_module_name: Option<&str>,
    has_async: bool,
    has_streams: bool,
) -> String {
    PreambleTemplate::new(prefix, ffi_module_name, has_async, has_streams)
        .render()
        .unwrap()
}

#[derive(Template)]
#[template(path = "record.txt", escape = "none")]
pub struct RecordTemplate<'a> {
    pub class_name: &'a str,
    pub fields: &'a [SwiftField],
    pub is_blittable: bool,
    pub blittable_size: Option<usize>,
}

impl<'a> RecordTemplate<'a> {
    pub fn from_record(record: &'a SwiftRecord) -> Self {
        Self {
            class_name: &record.class_name,
            fields: &record.fields,
            is_blittable: record.is_blittable,
            blittable_size: record.blittable_size,
        }
    }
}

#[derive(Template)]
#[template(path = "enum_c_style.txt", escape = "none")]
pub struct EnumCStyleTemplate<'a> {
    pub class_name: &'a str,
    pub variants: &'a [SwiftVariant],
    pub is_error: bool,
}

impl<'a> EnumCStyleTemplate<'a> {
    pub fn from_enum(e: &'a SwiftEnum) -> Self {
        Self {
            class_name: &e.name,
            variants: &e.variants,
            is_error: e.is_error,
        }
    }
}

#[derive(Template)]
#[template(path = "enum_data.txt", escape = "none")]
pub struct EnumDataTemplate<'a> {
    pub class_name: &'a str,
    pub variants: &'a [SwiftVariant],
    pub is_error: bool,
}

impl<'a> EnumDataTemplate<'a> {
    pub fn from_enum(e: &'a SwiftEnum) -> Self {
        Self {
            class_name: &e.name,
            variants: &e.variants,
            is_error: e.is_error,
        }
    }
}

pub fn render_record(record: &SwiftRecord) -> String {
    RecordTemplate::from_record(record).render().unwrap()
}

pub fn render_enum(e: &SwiftEnum) -> String {
    if e.is_c_style {
        EnumCStyleTemplate::from_enum(e).render().unwrap()
    } else {
        EnumDataTemplate::from_enum(e).render().unwrap()
    }
}

#[derive(Template)]
#[template(path = "callback_trait.txt", escape = "none")]
pub struct CallbackTemplate<'a> {
    pub callback: &'a SwiftCallback,
}

impl<'a> CallbackTemplate<'a> {
    pub fn new(callback: &'a SwiftCallback) -> Self {
        Self { callback }
    }
}

pub fn render_callback(callback: &SwiftCallback) -> String {
    CallbackTemplate::new(callback).render().unwrap()
}

#[derive(Template)]
#[template(path = "function.txt", escape = "none")]
pub struct FunctionTemplate<'a> {
    pub func: &'a SwiftFunction,
    pub prefix: &'a str,
}

impl<'a> FunctionTemplate<'a> {
    pub fn new(func: &'a SwiftFunction, prefix: &'a str) -> Self {
        Self { func, prefix }
    }
}

pub fn render_function(func: &SwiftFunction, prefix: &str) -> String {
    FunctionTemplate::new(func, prefix).render().unwrap()
}

#[derive(Template)]
#[template(path = "class.txt", escape = "none")]
pub struct ClassTemplate<'a> {
    pub cls: &'a SwiftClass,
    pub prefix: &'a str,
}

impl<'a> ClassTemplate<'a> {
    pub fn new(cls: &'a SwiftClass, prefix: &'a str) -> Self {
        Self { cls, prefix }
    }
}

pub fn render_class(cls: &SwiftClass, prefix: &str) -> String {
    ClassTemplate::new(cls, prefix).render().unwrap()
}

use super::plan::SwiftModule;

pub struct SwiftEmitter {
    prefix: String,
    ffi_module_name: Option<String>,
}

impl SwiftEmitter {
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
            ffi_module_name: None,
        }
    }

    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            ffi_module_name: None,
        }
    }

    pub fn with_ffi_module(mut self, ffi_module: impl Into<String>) -> Self {
        self.ffi_module_name = Some(ffi_module.into());
        self
    }

    pub fn emit(&self, module: &SwiftModule) -> String {
        let mut output = String::new();

        output.push_str(&render_preamble(
            &self.prefix,
            self.ffi_module_name.as_deref(),
            module.has_async(),
            module.has_streams(),
        ));
        output.push_str("\n\n");

        for record in &module.records {
            output.push_str(&render_record(record));
            output.push_str("\n\n");
        }

        for e in &module.enums {
            output.push_str(&render_enum(e));
            output.push_str("\n\n");
        }

        for callback in &module.callbacks {
            output.push_str(&render_callback(callback));
            output.push_str("\n\n");
        }

        for func in &module.functions {
            output.push_str(&render_function(func, &self.prefix));
            output.push_str("\n\n");
        }

        for cls in &module.classes {
            output.push_str(&render_class(cls, &self.prefix));
            output.push_str("\n\n");
        }

        output
    }
}

impl Default for SwiftEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::codec::CodecPlan;
    use crate::ir::types::PrimitiveType;
    use crate::render::swift::plan::{
        SwiftAsyncResult, SwiftCallbackParam, SwiftStream, SwiftStreamMode,
    };
    use crate::render::swift::{
        SwiftCallback, SwiftCallbackMethod, SwiftClass, SwiftConstructor, SwiftConversion,
        SwiftFunction, SwiftMethod, SwiftParam, SwiftReturn, SwiftVariantPayload,
    };

    #[test]
    fn snapshot_blittable_point() {
        let record = SwiftRecord {
            class_name: "Point".to_string(),
            fields: vec![
                SwiftField {
                    swift_name: "x".to_string(),
                    swift_type: "Double".to_string(),
                    default_expr: None,
                    codec: CodecPlan::Primitive(PrimitiveType::F64),
                    c_offset: Some(0),
                },
                SwiftField {
                    swift_name: "y".to_string(),
                    swift_type: "Double".to_string(),
                    default_expr: None,
                    codec: CodecPlan::Primitive(PrimitiveType::F64),
                    c_offset: Some(8),
                },
            ],
            is_blittable: true,
            blittable_size: Some(16),
        };
        insta::assert_snapshot!(render_record(&record));
    }

    #[test]
    fn snapshot_blittable_with_alignment_padding() {
        let record = SwiftRecord {
            class_name: "Padded".to_string(),
            fields: vec![
                SwiftField {
                    swift_name: "a".to_string(),
                    swift_type: "UInt8".to_string(),
                    default_expr: None,
                    codec: CodecPlan::Primitive(PrimitiveType::U8),
                    c_offset: Some(0),
                },
                SwiftField {
                    swift_name: "b".to_string(),
                    swift_type: "UInt32".to_string(),
                    default_expr: None,
                    codec: CodecPlan::Primitive(PrimitiveType::U32),
                    c_offset: Some(4),
                },
                SwiftField {
                    swift_name: "c".to_string(),
                    swift_type: "UInt8".to_string(),
                    default_expr: None,
                    codec: CodecPlan::Primitive(PrimitiveType::U8),
                    c_offset: Some(8),
                },
            ],
            is_blittable: true,
            blittable_size: Some(12),
        };
        insta::assert_snapshot!(render_record(&record));
    }

    #[test]
    fn snapshot_encoded_record_with_string() {
        let record = SwiftRecord {
            class_name: "User".to_string(),
            fields: vec![
                SwiftField {
                    swift_name: "id".to_string(),
                    swift_type: "Int64".to_string(),
                    default_expr: None,
                    codec: CodecPlan::Primitive(PrimitiveType::I64),
                    c_offset: None,
                },
                SwiftField {
                    swift_name: "name".to_string(),
                    swift_type: "String".to_string(),
                    default_expr: None,
                    codec: CodecPlan::String,
                    c_offset: None,
                },
            ],
            is_blittable: false,
            blittable_size: None,
        };
        insta::assert_snapshot!(render_record(&record));
    }

    #[test]
    fn snapshot_record_with_default_value() {
        let record = SwiftRecord {
            class_name: "Config".to_string(),
            fields: vec![
                SwiftField {
                    swift_name: "timeout".to_string(),
                    swift_type: "Double".to_string(),
                    default_expr: Some("30.0".to_string()),
                    codec: CodecPlan::Primitive(PrimitiveType::F64),
                    c_offset: Some(0),
                },
                SwiftField {
                    swift_name: "retries".to_string(),
                    swift_type: "Int32".to_string(),
                    default_expr: Some("3".to_string()),
                    codec: CodecPlan::Primitive(PrimitiveType::I32),
                    c_offset: Some(8),
                },
            ],
            is_blittable: true,
            blittable_size: Some(12),
        };
        insta::assert_snapshot!(render_record(&record));
    }

    #[test]
    fn snapshot_c_style_enum() {
        let e = SwiftEnum {
            name: "Status".to_string(),
            is_c_style: true,
            is_error: false,
            variants: vec![
                SwiftVariant {
                    swift_name: "active".to_string(),
                    discriminant: 0,
                    payload: SwiftVariantPayload::Unit,
                },
                SwiftVariant {
                    swift_name: "inactive".to_string(),
                    discriminant: 1,
                    payload: SwiftVariantPayload::Unit,
                },
                SwiftVariant {
                    swift_name: "pending".to_string(),
                    discriminant: 2,
                    payload: SwiftVariantPayload::Unit,
                },
            ],
            doc: None,
        };
        insta::assert_snapshot!(render_enum(&e));
    }

    #[test]
    fn snapshot_c_style_error_enum() {
        let e = SwiftEnum {
            name: "ApiError".to_string(),
            is_c_style: true,
            is_error: true,
            variants: vec![
                SwiftVariant {
                    swift_name: "notFound".to_string(),
                    discriminant: 0,
                    payload: SwiftVariantPayload::Unit,
                },
                SwiftVariant {
                    swift_name: "unauthorized".to_string(),
                    discriminant: 1,
                    payload: SwiftVariantPayload::Unit,
                },
                SwiftVariant {
                    swift_name: "serverError".to_string(),
                    discriminant: 2,
                    payload: SwiftVariantPayload::Unit,
                },
            ],
            doc: None,
        };
        insta::assert_snapshot!(render_enum(&e));
    }

    #[test]
    fn snapshot_data_enum_with_payloads() {
        let e = SwiftEnum {
            name: "Message".to_string(),
            is_c_style: false,
            is_error: false,
            variants: vec![
                SwiftVariant {
                    swift_name: "empty".to_string(),
                    discriminant: 0,
                    payload: SwiftVariantPayload::Unit,
                },
                SwiftVariant {
                    swift_name: "text".to_string(),
                    discriminant: 1,
                    payload: SwiftVariantPayload::Tuple(vec![SwiftField {
                        swift_name: "value".to_string(),
                        swift_type: "String".to_string(),
                        default_expr: None,
                        codec: CodecPlan::String,
                        c_offset: None,
                    }]),
                },
                SwiftVariant {
                    swift_name: "number".to_string(),
                    discriminant: 2,
                    payload: SwiftVariantPayload::Tuple(vec![SwiftField {
                        swift_name: "value".to_string(),
                        swift_type: "Int64".to_string(),
                        default_expr: None,
                        codec: CodecPlan::Primitive(PrimitiveType::I64),
                        c_offset: None,
                    }]),
                },
            ],
            doc: None,
        };
        insta::assert_snapshot!(render_enum(&e));
    }

    #[test]
    fn snapshot_data_enum_with_struct_payload() {
        let e = SwiftEnum {
            name: "Event".to_string(),
            is_c_style: false,
            is_error: false,
            variants: vec![
                SwiftVariant {
                    swift_name: "click".to_string(),
                    discriminant: 0,
                    payload: SwiftVariantPayload::Struct(vec![
                        SwiftField {
                            swift_name: "x".to_string(),
                            swift_type: "Int32".to_string(),
                            default_expr: None,
                            codec: CodecPlan::Primitive(PrimitiveType::I32),
                            c_offset: None,
                        },
                        SwiftField {
                            swift_name: "y".to_string(),
                            swift_type: "Int32".to_string(),
                            default_expr: None,
                            codec: CodecPlan::Primitive(PrimitiveType::I32),
                            c_offset: None,
                        },
                    ]),
                },
                SwiftVariant {
                    swift_name: "keyPress".to_string(),
                    discriminant: 1,
                    payload: SwiftVariantPayload::Struct(vec![SwiftField {
                        swift_name: "code".to_string(),
                        swift_type: "UInt32".to_string(),
                        default_expr: None,
                        codec: CodecPlan::Primitive(PrimitiveType::U32),
                        c_offset: None,
                    }]),
                },
            ],
            doc: None,
        };
        insta::assert_snapshot!(render_enum(&e));
    }

    #[test]
    fn snapshot_sync_function_returning_primitive() {
        let func = SwiftFunction {
            name: "add".to_string(),
            mode: SwiftCallMode::Sync {
                symbol: "riff_add".to_string(),
            },
            params: vec![
                SwiftParam {
                    label: None,
                    name: "a".to_string(),
                    swift_type: "Int32".to_string(),
                    conversion: SwiftConversion::Direct,
                },
                SwiftParam {
                    label: None,
                    name: "b".to_string(),
                    swift_type: "Int32".to_string(),
                    conversion: SwiftConversion::Direct,
                },
            ],
            returns: SwiftReturn::Direct {
                swift_type: "Int32".to_string(),
            },
            doc: None,
        };
        insta::assert_snapshot!(render_function(&func, "riff"));
    }

    #[test]
    fn snapshot_sync_function_with_string_param() {
        let func = SwiftFunction {
            name: "greet".to_string(),
            mode: SwiftCallMode::Sync {
                symbol: "riff_greet".to_string(),
            },
            params: vec![SwiftParam {
                label: None,
                name: "name".to_string(),
                swift_type: "String".to_string(),
                conversion: SwiftConversion::ToString,
            }],
            returns: SwiftReturn::FromWireBuffer {
                swift_type: "String".to_string(),
                codec: CodecPlan::String,
            },
            doc: None,
        };
        insta::assert_snapshot!(render_function(&func, "riff"));
    }

    #[test]
    fn snapshot_sync_function_with_record_param() {
        let func = SwiftFunction {
            name: "processPoint".to_string(),
            mode: SwiftCallMode::Sync {
                symbol: "riff_process_point".to_string(),
            },
            params: vec![SwiftParam {
                label: None,
                name: "point".to_string(),
                swift_type: "Point".to_string(),
                conversion: SwiftConversion::ToWireBuffer {
                    codec: CodecPlan::Record {
                        id: crate::ir::ids::RecordId::new("Point"),
                        layout: crate::ir::codec::RecordLayout::Blittable {
                            size: 16,
                            fields: vec![],
                        },
                    },
                },
            }],
            returns: SwiftReturn::FromWireBuffer {
                swift_type: "Point".to_string(),
                codec: CodecPlan::Record {
                    id: crate::ir::ids::RecordId::new("Point"),
                    layout: crate::ir::codec::RecordLayout::Blittable {
                        size: 16,
                        fields: vec![],
                    },
                },
            },
            doc: None,
        };
        insta::assert_snapshot!(render_function(&func, "riff"));
    }

    #[test]
    fn snapshot_async_function_returning_string() {
        let func = SwiftFunction {
            name: "fetchData".to_string(),
            mode: SwiftCallMode::Async {
                start: "riff_fetch_data_start".to_string(),
                poll: "riff_fetch_data_poll".to_string(),
                complete: "riff_fetch_data_complete".to_string(),
                cancel: "riff_fetch_data_cancel".to_string(),
                free: "riff_fetch_data_free".to_string(),
                result: Box::new(SwiftAsyncResult::Encoded {
                    swift_type: "String".to_string(),
                    ok_type: None,
                    codec: CodecPlan::String,
                    throws: false,
                }),
            },
            params: vec![SwiftParam {
                label: None,
                name: "url".to_string(),
                swift_type: "String".to_string(),
                conversion: SwiftConversion::ToString,
            }],
            returns: SwiftReturn::Void,
            doc: None,
        };
        insta::assert_snapshot!(render_function(&func, "riff"));
    }

    #[test]
    fn snapshot_callback_trait_simple() {
        let callback = SwiftCallback {
            protocol_name: "DataHandler".to_string(),
            wrapper_class: "DataHandlerWrapper".to_string(),
            vtable_var: "dataHandlerVtable".to_string(),
            vtable_type: "DataHandlerVtable".to_string(),
            bridge_name: "DataHandlerBridge".to_string(),
            register_fn: "riff_register_data_handler".to_string(),
            create_fn: "riff_create_data_handler".to_string(),
            methods: vec![SwiftCallbackMethod {
                swift_name: "onData".to_string(),
                ffi_name: "on_data".to_string(),
                params: vec![SwiftCallbackParam {
                    label: "data".to_string(),
                    swift_type: "Data".to_string(),
                    call_arg: "data".to_string(),
                    ffi_args: vec!["dataPtr".to_string(), "dataLen".to_string()],
                    decode_prelude: Some(
                        "let data = Data(bytes: dataPtr!, count: Int(dataLen))".to_string(),
                    ),
                }],
                returns: SwiftReturn::Void,
                is_async: false,
                has_out_param: false,
            }],
            doc: None,
        };
        insta::assert_snapshot!(render_callback(&callback));
    }

    #[test]
    fn snapshot_callback_trait_with_return() {
        let callback = SwiftCallback {
            protocol_name: "Validator".to_string(),
            wrapper_class: "ValidatorWrapper".to_string(),
            vtable_var: "validatorVtable".to_string(),
            vtable_type: "ValidatorVtable".to_string(),
            bridge_name: "ValidatorBridge".to_string(),
            register_fn: "riff_register_validator".to_string(),
            create_fn: "riff_create_validator".to_string(),
            methods: vec![SwiftCallbackMethod {
                swift_name: "validate".to_string(),
                ffi_name: "validate".to_string(),
                params: vec![SwiftCallbackParam {
                    label: "input".to_string(),
                    swift_type: "String".to_string(),
                    call_arg: "input".to_string(),
                    ffi_args: vec!["inputPtr".to_string(), "inputLen".to_string()],
                    decode_prelude: Some(
                        "let input = String(decoding: UnsafeBufferPointer(start: inputPtr, count: Int(inputLen)), as: UTF8.self)".to_string(),
                    ),
                }],
                returns: SwiftReturn::Direct {
                    swift_type: "Bool".to_string(),
                },
                is_async: false,
                has_out_param: true,
            }],
            doc: None,
        };
        insta::assert_snapshot!(render_callback(&callback));
    }

    #[test]
    fn snapshot_class_with_constructor_and_method() {
        let cls = SwiftClass {
            name: "Database".to_string(),
            ffi_free: "riff_database_free".to_string(),
            constructors: vec![SwiftConstructor::Designated {
                ffi_symbol: "riff_database_open".to_string(),
                params: vec![SwiftParam {
                    label: None,
                    name: "path".to_string(),
                    swift_type: "String".to_string(),
                    conversion: SwiftConversion::ToString,
                }],
                is_fallible: false,
                doc: None,
            }],
            methods: vec![SwiftMethod {
                name: "query".to_string(),
                mode: SwiftCallMode::Sync {
                    symbol: "riff_database_query".to_string(),
                },
                params: vec![SwiftParam {
                    label: None,
                    name: "sql".to_string(),
                    swift_type: "String".to_string(),
                    conversion: SwiftConversion::ToString,
                }],
                returns: SwiftReturn::FromWireBuffer {
                    swift_type: "String".to_string(),
                    codec: CodecPlan::String,
                },
                is_static: false,
                doc: None,
            }],
            streams: vec![],
            doc: None,
        };
        insta::assert_snapshot!(render_class(&cls, "riff"));
    }

    #[test]
    fn snapshot_class_with_stream() {
        let cls = SwiftClass {
            name: "EventSource".to_string(),
            ffi_free: "riff_event_source_free".to_string(),
            constructors: vec![],
            methods: vec![],
            streams: vec![SwiftStream {
                name: "events".to_string(),
                mode: SwiftStreamMode::Async,
                item_type: "String".to_string(),
                item_decode_expr: "wire.readString(at: 0).value".to_string(),
                subscribe: "riff_event_source_events_subscribe".to_string(),
                poll: "riff_event_source_events_poll".to_string(),
                pop_batch: "riff_event_source_events_pop_batch".to_string(),
                wait: "riff_event_source_events_wait".to_string(),
                unsubscribe: "riff_event_source_events_unsubscribe".to_string(),
                free: "riff_event_source_events_free".to_string(),
                free_buf: "riff_free_buf_u8".to_string(),
                atomic_cas: "riff_atomic_u8_cas".to_string(),
            }],
            doc: None,
        };
        insta::assert_snapshot!(render_class(&cls, "riff"));
    }

    #[test]
    fn snapshot_class_with_async_method() {
        let cls = SwiftClass {
            name: "HttpClient".to_string(),
            ffi_free: "riff_http_client_free".to_string(),
            constructors: vec![],
            methods: vec![SwiftMethod {
                name: "fetch".to_string(),
                mode: SwiftCallMode::Async {
                    start: "riff_http_client_fetch_start".to_string(),
                    poll: "riff_http_client_fetch_poll".to_string(),
                    complete: "riff_http_client_fetch_complete".to_string(),
                    cancel: "riff_http_client_fetch_cancel".to_string(),
                    free: "riff_http_client_fetch_free".to_string(),
                    result: Box::new(SwiftAsyncResult::Encoded {
                        swift_type: "Data".to_string(),
                        ok_type: None,
                        codec: CodecPlan::Bytes,
                        throws: false,
                    }),
                },
                params: vec![SwiftParam {
                    label: None,
                    name: "url".to_string(),
                    swift_type: "String".to_string(),
                    conversion: SwiftConversion::ToString,
                }],
                returns: SwiftReturn::Void,
                is_static: false,
                doc: None,
            }],
            streams: vec![],
            doc: None,
        };
        insta::assert_snapshot!(render_class(&cls, "riff"));
    }
}
