use std::{alloc::Layout, cell::RefCell, collections::HashMap, fs, path::PathBuf};

use crate::{
    error::Throwable,
    internal, internalise,
    native::{lang::LangObject, NativeModule},
};

use super::{
    builtins::{Class, Object},
    layout::{full_layout, types, ClassFileLayout},
    mem::{JavaObject, RefTo},
};
use parking_lot::RwLock;
use parse::{classfile::Resolvable, parser::Parser};
use support::descriptor::FieldType;
use tracing::debug;

pub fn base_layout() -> Layout {
    Layout::new::<Object>()
}

pub struct ClassLoader {
    class_path: Vec<PathBuf>,
    classes: RwLock<HashMap<FieldType, RefTo<Class>>>,
    meta_class: RefTo<Class>,
}

pub struct BootstrappedClasses {
    pub java_lang_class: RefTo<Class>,
    pub java_lang_object: RefTo<Class>,
    pub java_lang_string: RefTo<Class>,
    pub byte_array_ty: RefTo<Class>,
}

impl ClassLoader {
    pub fn new() -> Self {
        Self {
            class_path: vec![],
            classes: RwLock::new(HashMap::new()),
            meta_class: RefTo::null(),
        }
    }

    pub fn for_bytes(
        &self,
        field_type: FieldType,
        bytes: &[u8],
    ) -> Result<RefTo<Class>, Throwable> {
        let mut parser = Parser::new(bytes);
        let class_file = parser.parse()?;

        let mut super_class: Option<RefTo<Class>> = None;
        if let Some(ref cls) = class_file.super_class {
            let super_class_name = cls.resolve().name.resolve().string();
            let super_class_name = FieldType::parse(format!("L{};", super_class_name))?;
            super_class = Some(self.for_name(super_class_name)?);
        }

        /*
            In order for inheritance to work properly, and to allow future caching of offsets, we must layout from top to bottom
            in terms of inheritance. E.g:

            class A {
                int x;
                int y;
            }

            class B {
                int z;
            }

            would get the layout (only representing order):
            {
                x: 0,
                y: 4,
                z: 8
            }
        */

        // Start with an empty layout
        let mut layout = {
            // Setup the base at the start
            let mut cfl = ClassFileLayout::empty();
            cfl.layout = base_layout();

            cfl
        };

        // Figure out all of our superclasses so we can work backwards later
        let mut super_classes = vec![];
        {
            let mut _super = super_class.clone();
            while let Some(sup) = &_super {
                super_classes.push(sup.clone());

                let next_super = sup.unwrap_ref().super_class();
                if !next_super.is_null() {
                    _super = Some(next_super);
                } else {
                    _super = None;
                }
            }
        }

        // Iterate all our parents, in reverse, and layout each of them
        for sup in super_classes.iter().rev() {
            // Get the superclass layout
            let super_classfile = sup.unwrap_ref().class_file();

            // We need to construct our own instead of just using the one stored by the class
            // because that includes the header, which, if included, would mess up inheritance.
            let mut super_layout = full_layout(super_classfile, Layout::new::<()>())?;

            // Extend our layout based on it
            let (mut our_new_layout, offset_from_base) = layout
                .layout()
                .extend(super_layout.layout())
                .map_err(internalise!())?;

            // Align the new layout
            our_new_layout = our_new_layout.pad_to_align();

            // Adjust the offset of the superclass fields to be based on the new offsets
            super_layout.field_info.iter_mut().for_each(|(_, field)| {
                field.location.offset += offset_from_base;
            });

            // Push superclass fields into our layout
            layout.field_info.extend(super_layout.field_info);

            // Assign our layout to the newly computed one
            layout.layout = our_new_layout;
        }

        // Add our layout to the end of the layout
        // This should be integrated into the main loop above at some point, but for now we
        // keep it separate so we can apply the statics here. Really just need to cleanup
        // the layout APIs so that they can support this sort of usecase
        {
            let mut super_layout = full_layout(&class_file, Layout::new::<()>())?;

            // Extend our layout based on it
            let (mut our_new_layout, offset_from_base) = layout
                .layout()
                .extend(super_layout.layout())
                .map_err(internalise!())?;

            // Align the new layout
            our_new_layout = our_new_layout.pad_to_align();

            // Adjust the offset of the superclass fields to be based on the new offsets
            super_layout.field_info.iter_mut().for_each(|(_, field)| {
                field.location.offset += offset_from_base;
            });

            // Push superclass fields into our layout
            layout.field_info.extend(super_layout.field_info);

            // FIXME: Make a better full/basic API so that we can move the statics out instead of having to clone them here
            layout
                .statics
                .write()
                .extend(super_layout.statics.read().clone());

            // Assign our layout to the newly computed one
            layout.layout = our_new_layout;
        }

        let name = field_type.name();
        let cls = Class::new(
            Object::new(
                self.meta_class.clone(),
                super_class.unwrap_or(RefTo::null()),
            ),
            name,
            class_file,
            layout,
        );

        let object = RefTo::new(cls);

        {
            let mut classes = self.classes.write();
            classes.insert(field_type, object.clone());
        }

        Ok(object)
    }

    pub fn for_name(&self, field_type: FieldType) -> Result<RefTo<Class>, Throwable> {
        if let Some(object) = self.classes.read().get(&field_type) {
            debug!("Fast path: {}", field_type.name());
            assert!(object.is_not_null());
            return Ok(object.clone());
        }

        if let Some(array) = field_type.as_array() {
            let component_ty = self.for_name(*array.field_type.clone())?;
            let mut cls = Class::new_array(
                Object::new(RefTo::null(), RefTo::null()),
                component_ty,
                ClassFileLayout::from_java_type(types::ARRAY_BASE),
            );

            // Set the array classfile to the jlo classfile
            // Kinda hacky but not really sure how else to get methods onto arrays
            cls.set_class_file(
                self.meta_class
                    .unwrap_ref()
                    .super_class()
                    .unwrap_ref()
                    .class_file()
                    .clone(),
            );

            // Similarly, give all the native methods that object has and put them onto arrays
            let mut module = LangObject::new();
            module.init();
            cls.set_native_module(Box::new(RefCell::new(module)));

            let cls = RefTo::new(cls);
            {
                let mut classes = self.classes.write();
                classes.insert(field_type, cls.clone());
            }
            return Ok(cls);
        }

        let formatted_name = format!("{}.class", field_type.name());
        debug!("Slow path: {}", &formatted_name);

        let found_path = self.resolve_name(formatted_name.clone());
        if let Some(path) = found_path {
            let bytes = fs::read(path).map_err(internalise!())?;
            return self.for_bytes(field_type, &bytes);
        }

        Err(internal!(
            "Could not locate classfile {} ({:#?})",
            formatted_name,
            field_type
        ))
    }

    fn resolve_name(&self, name: String) -> Option<PathBuf> {
        let mut found_path: Option<PathBuf> = None;

        for root in self.class_path.iter() {
            let path = root.join::<PathBuf>(name.clone().into());
            if path.exists() {
                found_path = Some(path);
                break;
            }
        }

        found_path
    }

    pub fn classes(&self) -> &RwLock<HashMap<FieldType, RefTo<Class>>> {
        &self.classes
    }

    pub fn add_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.class_path.push(path.into());
        self
    }

    pub fn bootstrap(&mut self) -> Result<BootstrappedClasses, Throwable> {
        let jlc = self.for_name("Ljava/lang/Class;".into())?;
        self.meta_class = jlc.clone();

        let jlo = self.for_name("Ljava/lang/Object;".into())?;
        let jls = self.for_name("Ljava/lang/String;".into())?;

        macro_rules! primitive {
            ($layout_ty: ident, $name: expr) => {{
                let prim = RefTo::new(Class::new_primitive(
                    Object::new(jlc.clone(), jlo.clone()),
                    $name.to_string(),
                    ClassFileLayout::from_java_type(types::$layout_ty),
                ));

                let array = {
                    let mut cls = Class::new_array(
                        Object::new(RefTo::null(), RefTo::null()),
                        prim.clone(),
                        ClassFileLayout::from_java_type(types::ARRAY_BASE),
                    );

                    // Set the array classfile to the jlo classfile
                    // Kinda hacky but not really sure how else to get methods onto arrays
                    cls.set_class_file(
                        self.meta_class
                            .unwrap_ref()
                            .super_class()
                            .unwrap_ref()
                            .class_file()
                            .clone(),
                    );

                    // Similarly, give all the native methods that object has and put them onto arrays
                    let mut module = LangObject::new();
                    module.init();
                    cls.set_native_module(Box::new(RefCell::new(module)));

                    RefTo::new(cls)
                };

                (prim, array)
            }};
        }

        macro_rules! insert {
            ($tup: expr) => {
                {
                    let mut classes = self.classes.write();
                    classes.insert($tup.0.unwrap_ref().name().clone().into(), $tup.0);
                    classes.insert($tup.1.unwrap_ref().name().clone().into(), $tup.1);
                }
            };
        }

        // Primitives
        let byte = primitive!(BYTE, "B");
        insert!(byte.clone());

        insert!(primitive!(BOOL, "Z"));
        insert!(primitive!(SHORT, "S"));
        insert!(primitive!(CHAR, "C"));
        insert!(primitive!(INT, "I"));
        insert!(primitive!(LONG, "J"));
        insert!(primitive!(FLOAT, "F"));
        insert!(primitive!(DOUBLE, "D"));

        {
            let mut classes = self.classes.write();
            classes.iter_mut().for_each(|(_, value)| {
                value.with_lock(|value| {
                    value.header_mut().class = self.meta_class.clone();
                });
            });
        }

        Ok(BootstrappedClasses {
            java_lang_class: jlc,
            java_lang_object: jlo,
            java_lang_string: jls,
            byte_array_ty: byte.1,
        })
    }
}
