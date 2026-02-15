use crate::ir::{
    AbiCall, AbiParam, AbiType, AsyncCall, CallbackId, CallbackStyle, ClassId, InputShape,
    Mutability, OutputShape as AbiOutputShape, ParamName, ReadOp, ReadSeq, RecordId, ValueShape,
    WriteOp, WriteSeq,
};

#[derive(Debug, Clone)]
pub struct InputParam {
    pub name: ParamName,
    pub ffi_type: AbiType,
    pub abi: InputBinding,
}

impl InputParam {
    pub fn from_abi_param(param: &AbiParam) -> Option<Self> {
        let abi = match ParamBinding::from_abi_param(param) {
            ParamBinding::Input(abi) => abi,
            ParamBinding::Hidden(_) | ParamBinding::UnsupportedValue => return None,
        };
        Some(Self {
            name: param.name.clone(),
            ffi_type: param.ffi_type,
            abi,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ParamBinding {
    Input(InputBinding),
    Hidden(HiddenInputBinding),
    UnsupportedValue,
}

impl ParamBinding {
    pub fn from_abi_param(param: &AbiParam) -> Self {
        match &param.input_shape {
            InputShape::Value(ValueShape::Scalar(_)) => Self::Input(InputBinding::Scalar),
            InputShape::Utf8Slice { len_param } => Self::Input(InputBinding::Utf8Slice {
                len_param: len_param.clone(),
            }),
            InputShape::PrimitiveSlice {
                len_param,
                mutability,
                element_abi,
            } => Self::Input(InputBinding::PrimitiveSlice {
                len_param: len_param.clone(),
                mutability: *mutability,
                element_abi: *element_abi,
            }),
            InputShape::WirePacket { len_param, value } => Self::Input(InputBinding::WirePacket {
                len_param: len_param.clone(),
                decode_ops: value
                    .read_ops()
                    .unwrap_or_else(|| {
                        panic!(
                            "wire packet input shape missing decode ops for param {}",
                            param.name.as_str()
                        )
                    })
                    .clone(),
                encode_ops: value
                    .write_ops()
                    .unwrap_or_else(|| {
                        panic!(
                            "wire packet input shape missing encode ops for param {}",
                            param.name.as_str()
                        )
                    })
                    .clone(),
            }),
            InputShape::OutputBuffer { len_param, value } => {
                Self::Input(InputBinding::OutputBuffer {
                    len_param: len_param.clone(),
                    decode_ops: value
                        .read_ops()
                        .unwrap_or_else(|| {
                            panic!(
                                "output buffer input shape missing decode ops for param {}",
                                param.name.as_str()
                            )
                        })
                        .clone(),
                })
            }
            InputShape::Handle { class_id, nullable } => Self::Input(InputBinding::Handle {
                class_id: class_id.clone(),
                nullable: *nullable,
            }),
            InputShape::Callback {
                callback_id,
                nullable,
                style,
            } => Self::Input(InputBinding::CallbackHandle {
                callback_id: callback_id.clone(),
                nullable: *nullable,
                style: *style,
            }),
            InputShape::HiddenSyntheticLen { for_param } => {
                Self::Hidden(HiddenInputBinding::SyntheticLen {
                    for_param: for_param.clone(),
                })
            }
            InputShape::HiddenOutLen { for_param } => Self::Hidden(HiddenInputBinding::OutLen {
                for_param: for_param.clone(),
            }),
            InputShape::HiddenOutDirect => Self::Hidden(HiddenInputBinding::OutDirect),
            InputShape::HiddenStatusOut => Self::Hidden(HiddenInputBinding::StatusOut),
            InputShape::Value(_) => Self::UnsupportedValue,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InputBinding {
    Scalar,
    Utf8Slice {
        len_param: ParamName,
    },
    PrimitiveSlice {
        len_param: ParamName,
        mutability: Mutability,
        element_abi: AbiType,
    },
    WirePacket {
        len_param: ParamName,
        decode_ops: ReadSeq,
        encode_ops: WriteSeq,
    },
    OutputBuffer {
        len_param: ParamName,
        decode_ops: ReadSeq,
    },
    Handle {
        class_id: ClassId,
        nullable: bool,
    },
    CallbackHandle {
        callback_id: CallbackId,
        nullable: bool,
        style: CallbackStyle,
    },
}

impl InputBinding {
    pub fn from_abi_param(param: &AbiParam) -> Option<Self> {
        match ParamBinding::from_abi_param(param) {
            ParamBinding::Input(abi) => Some(abi),
            ParamBinding::Hidden(_) | ParamBinding::UnsupportedValue => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum HiddenInputBinding {
    SyntheticLen { for_param: ParamName },
    OutLen { for_param: ParamName },
    OutDirect,
    StatusOut,
}

impl HiddenInputBinding {
    pub fn from_abi_param(param: &AbiParam) -> Option<Self> {
        match ParamBinding::from_abi_param(param) {
            ParamBinding::Hidden(hidden) => Some(hidden),
            ParamBinding::Input(_) | ParamBinding::UnsupportedValue => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FastOutputBinding {
    Scalar {
        abi_type: AbiType,
    },
    OptionScalar {
        abi_type: AbiType,
        decode_ops: ReadSeq,
        encode_ops: WriteSeq,
    },
    ResultScalar {
        ok_abi: AbiType,
        err_abi: AbiType,
        decode_ops: ReadSeq,
        encode_ops: WriteSeq,
    },
    PrimitiveVec {
        element_abi: AbiType,
        decode_ops: ReadSeq,
        encode_ops: WriteSeq,
    },
    BlittableRecord {
        record_id: RecordId,
        size: u32,
        decode_ops: ReadSeq,
        encode_ops: WriteSeq,
    },
}

impl FastOutputBinding {
    pub fn decode_ops(&self) -> Option<&ReadSeq> {
        match self {
            Self::Scalar { .. } => None,
            Self::OptionScalar { decode_ops, .. }
            | Self::ResultScalar { decode_ops, .. }
            | Self::PrimitiveVec { decode_ops, .. }
            | Self::BlittableRecord { decode_ops, .. } => Some(decode_ops),
        }
    }

    pub fn encode_ops(&self) -> Option<&WriteSeq> {
        match self {
            Self::Scalar { .. } => None,
            Self::OptionScalar { encode_ops, .. }
            | Self::ResultScalar { encode_ops, .. }
            | Self::PrimitiveVec { encode_ops, .. }
            | Self::BlittableRecord { encode_ops, .. } => Some(encode_ops),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireOutputKind {
    Utf8String,
    Encoded,
}

#[derive(Debug, Clone)]
pub struct WireOutputBinding {
    pub decode_ops: ReadSeq,
    pub encode_ops: WriteSeq,
    pub wire_shape: WireOutputKind,
}

#[derive(Debug, Clone)]
pub enum OutputBinding {
    Unit,
    Fast(FastOutputBinding),
    Wire(WireOutputBinding),
    Handle {
        class_id: ClassId,
        nullable: bool,
    },
    CallbackHandle {
        callback_id: CallbackId,
        nullable: bool,
    },
}

impl OutputBinding {
    pub fn from_abi_call(call: &AbiCall) -> Self {
        Self::from_output_shape(&call.output_shape)
    }

    pub fn from_async_call(async_call: &AsyncCall) -> Self {
        Self::from_result_shape(&async_call.result_shape)
    }

    pub fn from_result_shape(result_shape: &AbiOutputShape) -> Self {
        Self::from_output_shape(result_shape)
    }

    pub fn from_output_shape(output_shape: &AbiOutputShape) -> Self {
        match output_shape {
            AbiOutputShape::Unit => Self::Unit,
            AbiOutputShape::Value(ValueShape::Scalar(abi_type)) => {
                Self::Fast(FastOutputBinding::Scalar {
                    abi_type: *abi_type,
                })
            }
            AbiOutputShape::Value(ValueShape::OptionScalar { abi, read, write }) => {
                Self::Fast(FastOutputBinding::OptionScalar {
                    abi_type: *abi,
                    decode_ops: read.clone(),
                    encode_ops: write.clone(),
                })
            }
            AbiOutputShape::Value(ValueShape::ResultScalar {
                ok,
                err,
                read,
                write,
            }) => Self::Fast(FastOutputBinding::ResultScalar {
                ok_abi: *ok,
                err_abi: *err,
                decode_ops: read.clone(),
                encode_ops: write.clone(),
            }),
            AbiOutputShape::Value(ValueShape::PrimitiveVec {
                element_abi,
                read,
                write,
            }) => Self::Fast(FastOutputBinding::PrimitiveVec {
                element_abi: *element_abi,
                decode_ops: read.clone(),
                encode_ops: write.clone(),
            }),
            AbiOutputShape::Value(ValueShape::BlittableRecord {
                id,
                size,
                read,
                write,
            }) => Self::Fast(FastOutputBinding::BlittableRecord {
                record_id: id.clone(),
                size: *size,
                decode_ops: read.clone(),
                encode_ops: write.clone(),
            }),
            AbiOutputShape::Handle { class_id, nullable } => Self::Handle {
                class_id: class_id.clone(),
                nullable: *nullable,
            },
            AbiOutputShape::Callback {
                callback_id,
                nullable,
            } => Self::CallbackHandle {
                callback_id: callback_id.clone(),
                nullable: *nullable,
            },
            AbiOutputShape::Value(ValueShape::WireEncoded { .. }) => {
                let decode_ops = output_shape_value_read_ops(output_shape);
                let encode_ops = output_shape_value_write_ops(output_shape);
                let wire_shape = classify_wire_shape(&decode_ops, &encode_ops);
                Self::Wire(WireOutputBinding {
                    decode_ops,
                    encode_ops,
                    wire_shape,
                })
            }
        }
    }

    pub fn decode_ops(&self) -> Option<&ReadSeq> {
        match self {
            Self::Fast(fast) => fast.decode_ops(),
            Self::Wire(wire) => Some(&wire.decode_ops),
            Self::Unit | Self::Handle { .. } | Self::CallbackHandle { .. } => None,
        }
    }

    pub fn encode_ops(&self) -> Option<&WriteSeq> {
        match self {
            Self::Fast(fast) => fast.encode_ops(),
            Self::Wire(wire) => Some(&wire.encode_ops),
            Self::Unit | Self::Handle { .. } | Self::CallbackHandle { .. } => None,
        }
    }
}

fn output_shape_value_read_ops(output_shape: &AbiOutputShape) -> ReadSeq {
    match output_shape {
        AbiOutputShape::Value(value_shape) => value_shape
            .read_ops()
            .unwrap_or_else(|| panic!("encoded output shape missing decode ops"))
            .clone(),
        _ => panic!("expected AbiOutputShape::Value"),
    }
}

fn output_shape_value_write_ops(output_shape: &AbiOutputShape) -> WriteSeq {
    match output_shape {
        AbiOutputShape::Value(value_shape) => value_shape
            .write_ops()
            .unwrap_or_else(|| panic!("encoded output shape missing encode ops"))
            .clone(),
        _ => panic!("expected AbiOutputShape::Value"),
    }
}

fn classify_wire_shape(decode_ops: &ReadSeq, encode_ops: &WriteSeq) -> WireOutputKind {
    match (decode_ops.ops.first(), encode_ops.ops.first()) {
        (Some(ReadOp::String { .. }), Some(WriteOp::String { .. })) => WireOutputKind::Utf8String,
        _ => WireOutputKind::Encoded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ops::{SizeExpr, WireShape};
    use crate::ir::{
        AbiParam, AbiType, CallbackId, CallbackStyle, ClassId, InputShape, Mutability, OutputShape,
        ParamName, RecordId, ValueShape,
    };

    fn empty_read_seq() -> ReadSeq {
        ReadSeq {
            size: SizeExpr::Fixed(0),
            ops: Vec::new(),
            shape: WireShape::Value,
        }
    }

    fn empty_write_seq() -> WriteSeq {
        WriteSeq {
            size: SizeExpr::Fixed(0),
            ops: Vec::new(),
            shape: WireShape::Value,
        }
    }

    #[test]
    fn input_abi_maps_supported_shapes() {
        let len_param = ParamName::new("payload_len");
        let class_id = ClassId::new("Connection");
        let callback_id = CallbackId::new("OnEvent");

        assert!(matches!(
            InputBinding::from_abi_param(&AbiParam {
                name: ParamName::new("value"),
                ffi_type: AbiType::I32,
                input_shape: InputShape::Value(ValueShape::Scalar(AbiType::I32)),
            }),
            Some(InputBinding::Scalar)
        ));
        assert!(matches!(
            InputBinding::from_abi_param(&AbiParam {
                name: ParamName::new("value"),
                ffi_type: AbiType::Pointer,
                input_shape: InputShape::Utf8Slice {
                    len_param: len_param.clone(),
                },
            }),
            Some(InputBinding::Utf8Slice { .. })
        ));
        assert!(matches!(
            InputBinding::from_abi_param(&AbiParam {
                name: ParamName::new("value"),
                ffi_type: AbiType::Pointer,
                input_shape: InputShape::PrimitiveSlice {
                    len_param: len_param.clone(),
                    mutability: Mutability::Shared,
                    element_abi: AbiType::I32,
                },
            }),
            Some(InputBinding::PrimitiveSlice { .. })
        ));
        assert!(matches!(
            InputBinding::from_abi_param(&AbiParam {
                name: ParamName::new("value"),
                ffi_type: AbiType::Pointer,
                input_shape: InputShape::WirePacket {
                    len_param: len_param.clone(),
                    value: ValueShape::WireEncoded {
                        read: empty_read_seq(),
                        write: empty_write_seq(),
                    },
                },
            }),
            Some(InputBinding::WirePacket { .. })
        ));
        assert!(matches!(
            InputBinding::from_abi_param(&AbiParam {
                name: ParamName::new("value"),
                ffi_type: AbiType::Pointer,
                input_shape: InputShape::OutputBuffer {
                    len_param: len_param.clone(),
                    value: ValueShape::WireEncoded {
                        read: empty_read_seq(),
                        write: empty_write_seq(),
                    },
                },
            }),
            Some(InputBinding::OutputBuffer { .. })
        ));
        assert!(matches!(
            InputBinding::from_abi_param(&AbiParam {
                name: ParamName::new("value"),
                ffi_type: AbiType::Pointer,
                input_shape: InputShape::Handle {
                    class_id: class_id.clone(),
                    nullable: true,
                },
            }),
            Some(InputBinding::Handle { .. })
        ));
        assert!(matches!(
            InputBinding::from_abi_param(&AbiParam {
                name: ParamName::new("value"),
                ffi_type: AbiType::Pointer,
                input_shape: InputShape::Callback {
                    callback_id: callback_id.clone(),
                    nullable: false,
                    style: CallbackStyle::BoxedDyn,
                },
            }),
            Some(InputBinding::CallbackHandle { .. })
        ));
    }

    #[test]
    fn output_abi_maps_shapes() {
        assert!(matches!(
            OutputBinding::from_output_shape(&OutputShape::Unit),
            OutputBinding::Unit
        ));
        assert!(matches!(
            OutputBinding::from_output_shape(&OutputShape::Value(ValueShape::Scalar(AbiType::I64))),
            OutputBinding::Fast(FastOutputBinding::Scalar { .. })
        ));
        assert!(matches!(
            OutputBinding::from_output_shape(&OutputShape::Value(ValueShape::OptionScalar {
                abi: AbiType::I32,
                read: empty_read_seq(),
                write: empty_write_seq()
            })),
            OutputBinding::Fast(FastOutputBinding::OptionScalar {
                abi_type: AbiType::I32,
                ..
            })
        ));
        assert!(matches!(
            OutputBinding::from_output_shape(&OutputShape::Value(ValueShape::ResultScalar {
                ok: AbiType::I32,
                err: AbiType::U32,
                read: empty_read_seq(),
                write: empty_write_seq()
            })),
            OutputBinding::Fast(FastOutputBinding::ResultScalar {
                ok_abi: AbiType::I32,
                err_abi: AbiType::U32,
                ..
            })
        ));
        assert!(matches!(
            OutputBinding::from_output_shape(&OutputShape::Value(ValueShape::PrimitiveVec {
                element_abi: AbiType::U32,
                read: empty_read_seq(),
                write: empty_write_seq()
            })),
            OutputBinding::Fast(FastOutputBinding::PrimitiveVec {
                element_abi: AbiType::U32,
                ..
            })
        ));
        assert!(matches!(
            OutputBinding::from_output_shape(&OutputShape::Value(ValueShape::BlittableRecord {
                id: RecordId::new("Point"),
                size: 16,
                read: empty_read_seq(),
                write: empty_write_seq()
            })),
            OutputBinding::Fast(FastOutputBinding::BlittableRecord { size: 16, .. })
        ));
    }

    #[test]
    fn render_backends_do_not_use_removed_route_constructors() {
        let swift_lower = include_str!("../render/swift/lower.rs");
        let kotlin_lower = include_str!("../render/kotlin/lower.rs");
        let typescript_lower = include_str!("../render/typescript/lower.rs");

        [swift_lower, kotlin_lower, typescript_lower]
            .into_iter()
            .for_each(|source| {
                assert!(!source.contains("from_param_role("));
                assert!(!source.contains("from_return_transport("));
                assert!(!source.contains("from_async_result_transport("));
                assert!(!source.contains("InputShape::"));
                assert!(!source.contains("OutputShape::"));
            });
    }
}
