use std::collections::HashMap;
use std::sync::Arc;
use crate::pva_message::typ::PvaType;

pub struct PvaTypeRegistry {
    pub types: HashMap<i16, Arc<PvaType>>,
}

impl PvaTypeRegistry {
    pub fn new() -> Self {
        PvaTypeRegistry {
            types: HashMap::new(),
        }
    }

    pub fn add(self: &mut Self, id: i16, typ: Arc<PvaType>) {
        self.types_mut().insert(id, typ);
    }

    pub fn remove(self: &mut Self, id: i16) {
        self.types_mut().remove(&id);
    }

    pub fn types_mut(self: &mut Self) -> &mut HashMap<i16, Arc<PvaType>> {
        &mut self.types
    }

    pub fn types(self: &Self) -> &HashMap<i16, Arc<PvaType>> {
        &self.types
    }

    pub fn typ(self: &Self, id: i16) -> Option<Arc<PvaType>> {
        self.types().get(&id).cloned()
    }
}
