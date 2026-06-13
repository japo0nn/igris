use crate::models::metadata::ModuleType::{Ephemeral, Persistent};

#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub name: String,
    pub version: String,
    pub _type: ModuleType,
    pub description: String,
    pub author: Option<String>,
}

impl ModuleMetadata {
    pub fn display(&self) {
        println!("Skill name: {}", self.name);
        println!("Version: {}", self.version);
        println!("Description: {}", self.description);

        match self._type {
            Persistent => {
                println!("Type: Persistent");
            }
            Ephemeral => {
                println!("Type: Ephemeral");
            }
        };

        match &self.author {
            Some(value) => {
                println!("Author: {}", value);
            }
            None => {
                println!("Author: Unknown");
            }
        };
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ModuleType {
    Persistent,
    Ephemeral,
}
