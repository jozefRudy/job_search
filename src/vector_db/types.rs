use arrow_schema::{DataType, Field};
use std::sync::Arc;

pub const EMBEDDING_DIM: i32 = 384;

pub(crate) fn embedding_field(name: &str) -> Field {
    Field::new(
        name,
        DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            EMBEDDING_DIM,
        ),
        false,
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct VectorQuery {
    pub embedding: Vec<f32>,
    pub top_n: usize,
}

impl VectorQuery {
    pub fn new(embedding: Vec<f32>) -> Self {
        Self {
            embedding,
            top_n: 500,
        }
    }
}
