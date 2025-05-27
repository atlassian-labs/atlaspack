use protobuf::descriptor::field_descriptor_proto::Type;
use protobuf_parse::Parser;

fn main() {
  let schema_dir = "packages/core/core/src/protobuf/";
  let mut parser = Parser::new();

  let mut output_decls = vec![];

  for entry in std::fs::read_dir(schema_dir).unwrap() {
    let entry = entry.unwrap();
    let path = entry.path();
    println!("{}", path.display());

    parser.input(path);
    parser.include(schema_dir);

    let file_descriptor_set = parser.file_descriptor_set().unwrap();
    for file_descriptor in file_descriptor_set.file {
      for message in file_descriptor.message_type {
        // type $MESSAGE = { ...$FIELDS }
        output_decls.push(format!(
          "export type {} = {{|\n{}\n|}};\n",
          message.name.unwrap(),
          message
            .field
            .iter()
            .map(|field| {
              format!(
                "  {}{}: {}",
                field.name.as_ref().unwrap(),
                if field.proto3_optional.unwrap_or(false) {
                  "?"
                } else {
                  ""
                },
                field_type_to_javascript_type(field)
              )
            })
            .collect::<Vec<String>>()
            .join(",\n")
        ));
      }

      for enum_ in file_descriptor.enum_type {
        output_decls.push(format!(
          "export type {} = {};\n",
          enum_.name.as_ref().unwrap(),
          enum_
            .value
            .iter()
            .map(|value| format!(
              "'{}'",
              convert_enum_to_flow_string(
                enum_.name.as_ref().unwrap(),
                value.name.as_ref().unwrap()
              )
            ))
            .collect::<Vec<String>>()
            .join(" | ")
        ));
      }
    }
  }

  let output_file = format!("// @flow strict-local\n\n{}", output_decls.join("\n"));
  println!("{}", output_file);
}

fn field_type_to_javascript_type(field: &protobuf::descriptor::FieldDescriptorProto) -> String {
  let type_ = field.type_();
  match type_ {
    Type::TYPE_STRING => "string".to_string(),
    Type::TYPE_INT32 => "number".to_string(),
    Type::TYPE_BOOL => "boolean".to_string(),
    Type::TYPE_FLOAT => "number".to_string(),
    Type::TYPE_DOUBLE => "number".to_string(),
    Type::TYPE_BYTES => "string".to_string(),
    Type::TYPE_INT64 => "number".to_string(),
    Type::TYPE_UINT64 => "number".to_string(),
    Type::TYPE_FIXED64 => "number".to_string(),
    Type::TYPE_FIXED32 => "number".to_string(),
    Type::TYPE_UINT32 => "number".to_string(),
    Type::TYPE_SFIXED32 => "number".to_string(),
    Type::TYPE_SFIXED64 => "number".to_string(),
    Type::TYPE_SINT32 => "number".to_string(),
    Type::TYPE_SINT64 => "number".to_string(),

    Type::TYPE_GROUP => todo!(),
    Type::TYPE_MESSAGE => {
      let message_name = field.type_name.as_ref().unwrap();
      format!("{}", message_name.split(".").last().unwrap())
    }
    Type::TYPE_ENUM => {
      let enum_name = field.type_name.as_ref().unwrap();
      format!("{}", enum_name.split(".").last().unwrap())
    }
  }
}

fn convert_enum_to_flow_string(enum_name: &str, value_name: &str) -> String {
  // if enum is OutputFormat
  // and value is OUTPUT_FORMAT_ESMODULE
  // then return 'esmodule'

  // convert UpperCamelCase to snake_case
  let to_snake_case = |s: &str| {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
      if i > 0 && c.is_uppercase() {
        result.push('_');
      }
      result.push(c.to_ascii_lowercase());
    }
    result
  };

  let to_kebab_case = |s: &str| s.to_lowercase().replace("_", "-");

  let enum_name = to_snake_case(enum_name);
  let value_name = value_name.to_lowercase();

  format!(
    "{}",
    to_kebab_case(&value_name.replace(&format!("{}_", enum_name), ""))
  )
}
