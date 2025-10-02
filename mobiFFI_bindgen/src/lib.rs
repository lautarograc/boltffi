pub mod model;
pub mod kotlin;
pub mod swift;

pub use model::{
    Class, Constructor, ConstructorParam, Deprecation, Enumeration, Function, Method, Module,
    Parameter, Primitive, Receiver, Record, RecordField, StreamMethod, Type, Variant,
};

pub use kotlin::Kotlin;
pub use swift::Swift;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_module() -> Module {
        let sensor_class = Class::new("Sensor")
            .with_doc("A hardware sensor interface")
            .with_constructor(Constructor::new())
            .with_method(
                Method::new("get_reading", Receiver::Ref)
                    .with_output(Type::Primitive(Primitive::F64)),
            )
            .with_method(
                Method::new("predict_next", Receiver::Ref)
                    .with_param(Parameter::new("samples", Type::Primitive(Primitive::U32)))
                    .with_output(Type::vec(Type::Primitive(Primitive::F64)))
                    .make_async(),
            )
            .with_stream(StreamMethod::new(
                "readings",
                Type::Primitive(Primitive::F64),
            ));

        let reading_record = Record::new("SensorReading")
            .with_field(RecordField::new("timestamp", Type::Primitive(Primitive::U64)))
            .with_field(RecordField::new("value", Type::Primitive(Primitive::F64)))
            .with_field(RecordField::new("unit", Type::String));

        let status_enum = Enumeration::new("SensorStatus")
            .with_variant(Variant::new("idle").with_discriminant(0))
            .with_variant(Variant::new("active").with_discriminant(1))
            .with_variant(Variant::new("error").with_discriminant(2));

        Module::new("sensors")
            .with_class(sensor_class)
            .with_record(reading_record)
            .with_enum(status_enum)
    }

    #[test]
    fn test_module_ffi_prefix() {
        let module = create_test_module();
        assert_eq!(module.ffi_prefix(), "mffi_sensors");
    }

    #[test]
    fn test_class_ffi_names() {
        let module = create_test_module();
        let class = module.find_class("Sensor").unwrap();
        let prefix = module.ffi_prefix();

        assert_eq!(class.ffi_new(&prefix), "mffi_sensors_sensor_new");
        assert_eq!(class.ffi_free(&prefix), "mffi_sensors_sensor_free");
    }

    #[test]
    fn test_method_ffi_names() {
        let module = create_test_module();
        let class = module.find_class("Sensor").unwrap();
        let class_prefix = class.ffi_prefix(&module.ffi_prefix());
        let method = class.methods.iter().find(|m| m.name == "predict_next").unwrap();

        assert_eq!(method.ffi_name(&class_prefix), "mffi_sensors_sensor_predict_next");
        assert_eq!(method.ffi_poll(&class_prefix), "mffi_sensors_sensor_predict_next_poll");
    }

    #[test]
    fn test_swift_type_mapping() {
        assert_eq!(Swift::type_name(&Type::Primitive(Primitive::I32)), "Int32");
        assert_eq!(Swift::type_name(&Type::Primitive(Primitive::Bool)), "Bool");
        assert_eq!(Swift::type_name(&Type::String), "String");
        assert_eq!(Swift::type_name(&Type::Bytes), "Data");
        assert_eq!(Swift::type_name(&Type::vec(Type::Primitive(Primitive::F64))), "[Double]");
        assert_eq!(Swift::type_name(&Type::option(Type::String)), "String?");
    }

    #[test]
    fn test_swift_naming_convention() {
        assert_eq!(Swift::class_name("sensor_manager"), "SensorManager");
        assert_eq!(Swift::method_name("get_current_reading"), "getCurrentReading");
        assert_eq!(Swift::param_name("sample_count"), "sampleCount");
        assert_eq!(Swift::enum_case_name("NOT_FOUND"), "notFound");
    }

    #[test]
    fn test_swift_record_generation() {
        let module = create_test_module();
        let record = module.find_record("SensorReading").unwrap();
        let output = Swift::render_record(record);

        assert!(output.contains("public struct SensorReading"));
        assert!(output.contains("public var timestamp: UInt64"));
        assert!(output.contains("public var value: Double"));
        assert!(output.contains("public var unit: String"));
    }

    #[test]
    fn test_swift_enum_generation() {
        let module = create_test_module();
        let enumeration = module.find_enum("SensorStatus").unwrap();
        let output = Swift::render_enum(enumeration);

        assert!(output.contains("public enum SensorStatus: Int32"));
        assert!(output.contains("case idle = 0"));
        assert!(output.contains("case active = 1"));
        assert!(output.contains("case error = 2"));
    }

    #[test]
    fn test_enum_detection() {
        let c_style = Enumeration::new("Status")
            .with_variant(Variant::new("ok"))
            .with_variant(Variant::new("error"));

        let data_enum = Enumeration::new("Result")
            .with_variant(Variant::new("success").with_field(RecordField::new("value", Type::Primitive(Primitive::I32))))
            .with_variant(Variant::new("failure"));

        assert!(c_style.is_c_style());
        assert!(!data_enum.is_c_style());
        assert!(data_enum.is_data_enum());
    }

    #[test]
    fn test_swift_params_declaration() {
        let params = vec![
            Parameter::new("sample_count", Type::Primitive(Primitive::U32)),
            Parameter::new("threshold", Type::Primitive(Primitive::F64)),
        ];

        let output = Swift::params_declaration(&params);
        assert_eq!(output, "sampleCount: UInt32, threshold: Double");
    }
}
