mod body;
mod names;
mod templates;
mod types;

use askama::Template;

use crate::model::{Class, Enumeration, Method, Module, Parameter, Record, StreamMethod};

pub use body::BodyRenderer;
pub use names::NamingConvention;
pub use templates::{ClassTemplate, CStyleEnumTemplate, DataEnumTemplate, RecordTemplate};
pub use types::TypeMapper;

pub struct Swift;

impl Swift {
    pub fn type_name(ty: &crate::model::Type) -> String {
        TypeMapper::map_type(ty)
    }

    pub fn method_name(name: &str) -> String {
        NamingConvention::method_name(name)
    }

    pub fn class_name(name: &str) -> String {
        NamingConvention::class_name(name)
    }

    pub fn param_name(name: &str) -> String {
        NamingConvention::param_name(name)
    }

    pub fn enum_case_name(name: &str) -> String {
        NamingConvention::enum_case_name(name)
    }

    pub fn property_name(name: &str) -> String {
        NamingConvention::property_name(name)
    }

    pub fn params_declaration(params: &[Parameter]) -> String {
        params
            .iter()
            .map(|param| {
                format!(
                    "{}: {}",
                    NamingConvention::param_name(&param.name),
                    TypeMapper::map_type(&param.param_type)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn method_signature(method: &Method) -> String {
        let params = Self::params_declaration(&method.inputs);
        let return_clause = method
            .output
            .as_ref()
            .filter(|ty| !ty.is_void())
            .map(|ty| format!(" -> {}", TypeMapper::map_type(ty)))
            .unwrap_or_default();

        let async_modifier = if method.is_async { " async" } else { "" };
        let throws_modifier = if method.throws() { " throws" } else { "" };

        format!(
            "({params}){async_modifier}{throws_modifier}{return_clause}"
        )
    }

    pub fn render_method_body(method: &Method, class: &Class, module: &Module) -> String {
        BodyRenderer::render_method(method, class, module)
    }

    pub fn render_stream_body(stream: &StreamMethod, class: &Class, module: &Module) -> String {
        BodyRenderer::render_stream(stream, class, module)
    }

    pub fn render_record(record: &Record) -> String {
        RecordTemplate::from_record(record)
            .render()
            .expect("record template failed")
    }

    pub fn render_enum(enumeration: &Enumeration) -> String {
        if enumeration.is_c_style() {
            CStyleEnumTemplate::from_enum(enumeration)
                .render()
                .expect("c-style enum template failed")
        } else {
            DataEnumTemplate::from_enum(enumeration)
                .render()
                .expect("data enum template failed")
        }
    }

    pub fn render_class(class: &Class, module: &Module) -> String {
        ClassTemplate::from_class(class, module)
            .render()
            .expect("class template failed")
    }
}
