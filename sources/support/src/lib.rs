pub mod bytes_ext;
pub mod descriptor;
pub mod encoding;
pub mod types;
pub mod jar;

#[cfg(test)]
mod tests {
    use crate::descriptor::{BaseType, FieldType, MethodType, ObjectType};
    use anyhow::Result;

    #[test]
    fn it_parses_simple_descriptors() -> Result<()> {
        let descriptor = FieldType::parse("Z".to_string())?;
        let descriptor = descriptor.into_base().unwrap();

        assert!(descriptor.is_boolean());

        Ok(())
    }

    #[test]
    fn it_parses_array_descriptors() -> Result<()> {
        let descriptor = FieldType::parse("[D".to_string())?;
        let descriptor = descriptor.into_array().unwrap();

        let field = descriptor.field_type;
        let field = field.into_base().unwrap();

        assert!(field.is_double());

        Ok(())
    }

    #[test]
    fn it_parses_deep_array_descriptors() -> Result<()> {
        let descriptor = FieldType::parse("[[[D".to_string())?;
        let descriptor = descriptor.into_array().unwrap();

        let field = descriptor.field_type;
        let field = field.into_base().unwrap();

        assert_eq!(descriptor.dimensions, 3);
        assert!(field.is_double());

        Ok(())
    }

    #[test]
    fn it_parses_class_descriptors() -> Result<()> {
        let descriptor = FieldType::parse("Ljava/lang/Object;".to_string())?;
        let descriptor = descriptor.into_object().unwrap();

        let class_name = descriptor.class_name;
        assert_eq!(class_name, "java/lang/Object");

        Ok(())
    }

    #[test]
    fn it_parses_method_descriptors() -> Result<()> {
        let descriptor = MethodType::parse("(IDLjava/lang/Thread;)Ljava/lang/Object;".to_string())?;
        assert_eq!(
            descriptor.parameters,
            vec![
                FieldType::Base(BaseType::Int),
                FieldType::Base(BaseType::Double),
                FieldType::Object(ObjectType {
                    class_name: "java/lang/Thread".to_string()
                })
            ]
        );

        assert_eq!(
            descriptor.return_type,
            FieldType::Object(ObjectType {
                class_name: "java/lang/Object".to_string()
            })
        );

        Ok(())
    }

    #[test]
    fn it_unparses_simple_descriptors() -> Result<()> {
        let descriptor = FieldType::parse("Z".to_string())?;
        let unparsed = descriptor.to_string();
        assert_eq!(unparsed, "Z");

        Ok(())
    }

    #[test]
    fn it_unparses_array_descriptors() -> Result<()> {
        let descriptor = FieldType::parse("[D".to_string())?;
        let unparsed = descriptor.to_string();

        assert_eq!(unparsed, "[D");

        Ok(())
    }

    #[test]
    fn it_unparses_deep_array_descriptors() -> Result<()> {
        let descriptor = FieldType::parse("[[[D".to_string())?;
        let unparsed = descriptor.to_string();

        assert_eq!(unparsed, "[[[D");

        Ok(())
    }

    #[test]
    fn it_unparses_method_descriptors() -> Result<()> {
        let descriptor = MethodType::parse("(IDLjava/lang/Thread;)Ljava/lang/Object;".to_string())?;
        let unparsed = descriptor.to_string();

        assert_eq!(unparsed, "(IDLjava/lang/Thread;)Ljava/lang/Object;");

        Ok(())
    }

    #[test]
    fn it_unparses_arraycopy_descriptors() -> Result<()> {
        let descriptor =
            MethodType::parse("(Ljava/lang/Object;ILjava/lang/Object;II)V".to_string())?;
        let unparsed = descriptor.to_string();

        assert_eq!(unparsed, "(Ljava/lang/Object;ILjava/lang/Object;II)V");

        Ok(())
    }
}

#[cfg(test)]
mod jar_tests {
    use std::io::{BufReader, Read};
    use std::path::Path;
    use std::fs::File;
    use crate::jar::{JarFile};
    use anyhow::Result;
    

    fn read_bytes(p: impl AsRef<Path>) -> Result<Vec<u8>> {
        let f = File::open(p)?;
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::with_capacity(1024 * 100);

        // Read file into vector.
        reader.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    #[test]
    fn it_opens_jars() -> Result<()> {
        // PWD = src/support
        let bytes = read_bytes("../../std.jar").unwrap();
        let jar = JarFile::from_bytes(bytes).unwrap();
        let jlc = jar.locate_class("java/lang/String.class");

        Ok(())
    }
}
