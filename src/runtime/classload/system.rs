use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::{anyhow, Result};
use tracing::debug;

use crate::runtime::classload::loader::{ClassDefinition, ClassLoader, PackageDefinition};
use crate::stdlib::VISITORS;
use crate::structs::loaded::package::Package;
use crate::{ClassFileParser, LoadedClassFile};

pub struct SystemClassLoader {
    classes: HashMap<String, Arc<LoadedClassFile>>,
}

impl SystemClassLoader {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
        }
    }
}

impl ClassLoader<SystemClassLoader> for SystemClassLoader {
    fn parent(&self) -> Option<Arc<RwLock<SystemClassLoader>>> {
        None
    }

    fn find_class(&self, internal_name: &str) -> Result<ClassDefinition> {
        let bytes = ClassFileParser::bytes(internal_name.to_owned())?;
        Ok(ClassDefinition {
            internal_name: Some(internal_name.to_owned()),
            bytes,
            protection_domain: None,
        })
    }

    fn find_loaded_class(&self, internal_name: &str) -> Option<Arc<LoadedClassFile>> {
        self.classes.get(internal_name).map(Arc::clone)
    }

    fn get_package(&self, _internal_name: &str) -> Result<Arc<Package>> {
        todo!()
    }

    fn get_packages(&self) -> Result<Vec<Arc<Package>>> {
        todo!()
    }

    fn define_class(&mut self, data: ClassDefinition) -> Result<Arc<LoadedClassFile>> {
        let name = data.internal_name;
        let name = name.ok_or_else(|| {
            anyhow!("name was not present, and anonymous classes are not supported yet")
        })?;

        let res =
            LoadedClassFile::from_raw(ClassFileParser::from_bytes(name, data.bytes).parse()?)?;

        let res = Arc::new(res);

        self.classes
            .insert(res.this_class.name.str.clone(), Arc::clone(&res));

        let visitor = VISITORS.get(&res.this_class.name.str);

        if let Some(func) = visitor {
            debug!("calling visitor for classfile");
            func(Arc::clone(&res));
        }

        Ok(res)
    }

    fn define_package(&self, _data: PackageDefinition) -> Result<Arc<Package>> {
        todo!()
    }
}
