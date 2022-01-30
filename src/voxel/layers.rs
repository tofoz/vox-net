use std::collections::HashMap;

use packed_struct::prelude::*;

// add way for layer to tell if its empty or not ??

pub struct Volume {
    pub type_id: AttributeLayer<u16>,
}

impl Volume {
    pub fn new(size: usize) -> Self {
        let type_id = AttributeLayer {
            layer: vec![0; size],
        };
        Self { type_id }
    }
}

pub struct AttributeLayer<T> {
    pub layer: Vec<T>,
}

// ---------------------------------------------------------------------------------------------------

impl<T> AttributeLayer<T> {
    // todo add xyz getters and xyz setters
}

enum ListDensity<T> {
    Dense(DenseList<T>),
    // todo
    Sparse,
}

// useful for when layer property's are dense such as ids or color, however to tell when to convert to a sparse list requires a type id system.
struct DenseList<T> {
    val: Vec<T>,
}

// -----------------------------------------------

//  however if the layer gets to dense with them you may want to switch to denies list
struct SparseList<T> {
    val: Option<HashMap<u16, Option<T>>>,
}

impl<T> SparseList<T> {
    // fn new(val: Option<HashMap<u16, Option<T>>>) -> Self { Self { val } }
    // todo: add a get, set, inset, remove
}
