use std::{
    error::Error,
    fmt::{self, Display},
};

use facet_serialize::{Serialize, Serializer};
use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

/// Error type for KDL serialization.
#[derive(Debug)]
pub struct KdlSerializeError {
    message: String,
}

impl Display for KdlSerializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KDL serialization error: {}", self.message)
    }
}

impl Error for KdlSerializeError {}

impl KdlSerializeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Serializer for KDL documents.
pub struct KdlSerializer {
    pub document: KdlDocument,
    pub current_node: Option<KdlNode>,
    pub node_stack: Vec<KdlNode>,
    pub current_key: Option<String>,
}

impl KdlSerializer {
    /// Create a new KDL serializer.
    pub fn new() -> Self {
        Self {
            document: KdlDocument::new(),
            current_node: None,
            node_stack: Vec::new(),
            current_key: None,
        }
    }

    /// Get the output serialized KDL document.
    pub fn into_document(self) -> KdlDocument {
        self.document
    }

    /// Get the output serialized KDL string.
    pub fn into_string(self) -> String {
        self.document.to_string()
    }
}

impl Serializer for KdlSerializer {
    type Error = KdlSerializeError;

    fn serialize_bool(&mut self, v: bool) -> Result<(), Self::Error> {
        log::trace!("Serializing bool: {}", v);
        if let Some(ref mut node) = self.current_node {
            if let Some(key) = self.current_key.take() {
                node.push(KdlEntry::new_prop(key, KdlValue::Bool(v)));
            } else {
                node.push(KdlEntry::new(KdlValue::Bool(v)));
            }
        }
        Ok(())
    }

    fn serialize_i8(&mut self, v: i8) -> Result<(), Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(&mut self, v: i16) -> Result<(), Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(&mut self, v: i32) -> Result<(), Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(&mut self, v: i64) -> Result<(), Self::Error> {
        log::trace!("Serializing i64: {}", v);
        if let Some(ref mut node) = self.current_node {
            if let Some(key) = self.current_key.take() {
                node.push(KdlEntry::new_prop(key, KdlValue::Integer(v as i128)));
            } else {
                node.push(KdlEntry::new(KdlValue::Integer(v as i128)));
            }
        }
        Ok(())
    }

    fn serialize_i128(&mut self, v: i128) -> Result<(), Self::Error> {
        log::trace!("Serializing i128: {}", v);
        if let Some(ref mut node) = self.current_node {
            if let Some(key) = self.current_key.take() {
                node.push(KdlEntry::new_prop(key, KdlValue::Integer(v)));
            } else {
                node.push(KdlEntry::new(KdlValue::Integer(v)));
            }
        }
        Ok(())
    }

    fn serialize_u8(&mut self, v: u8) -> Result<(), Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(&mut self, v: u16) -> Result<(), Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(&mut self, v: u32) -> Result<(), Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(&mut self, v: u64) -> Result<(), Self::Error> {
        if v > i128::MAX as u64 {
            return Err(KdlSerializeError::new(format!(
                "u64 value {} is too large for KDL",
                v
            )));
        }
        self.serialize_i128(v as i128)
    }

    fn serialize_u128(&mut self, v: u128) -> Result<(), Self::Error> {
        if v > i128::MAX as u128 {
            return Err(KdlSerializeError::new(format!(
                "u128 value {} is too large for KDL",
                v
            )));
        }
        self.serialize_i128(v as i128)
    }

    fn serialize_f32(&mut self, v: f32) -> Result<(), Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(&mut self, v: f64) -> Result<(), Self::Error> {
        log::trace!("Serializing f64: {}", v);
        if let Some(ref mut node) = self.current_node {
            if let Some(key) = self.current_key.take() {
                node.push(KdlEntry::new_prop(key, KdlValue::Float(v)));
            } else {
                node.push(KdlEntry::new(KdlValue::Float(v)));
            }
        }
        Ok(())
    }

    fn serialize_char(&mut self, v: char) -> Result<(), Self::Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(&mut self, v: &str) -> Result<(), Self::Error> {
        log::trace!("Serializing string: {}", v);
        if let Some(ref mut node) = self.current_node {
            if let Some(key) = self.current_key.take() {
                node.push(KdlEntry::new_prop(key, KdlValue::String(v.to_string())));
            } else {
                node.push(KdlEntry::new(KdlValue::String(v.to_string())));
            }
        }
        Ok(())
    }

    fn serialize_bytes(&mut self, _v: &[u8]) -> Result<(), Self::Error> {
        // KDL doesn't have native byte array support
        Err(KdlSerializeError::new("Byte arrays not supported in KDL"))
    }

    fn serialize_none(&mut self) -> Result<(), Self::Error> {
        log::trace!("Serializing None");
        if let Some(ref mut node) = self.current_node {
            if let Some(key) = self.current_key.take() {
                node.push(KdlEntry::new_prop(key, KdlValue::Null));
            } else {
                node.push(KdlEntry::new(KdlValue::Null));
            }
        }
        Ok(())
    }

    fn start_some(&mut self) -> Result<(), Self::Error> {
        log::trace!("Starting Some");
        // For Option<T>, we just serialize the inner value
        Ok(())
    }

    fn serialize_unit(&mut self) -> Result<(), Self::Error> {
        log::trace!("Serializing unit");
        Ok(())
    }

    fn serialize_unit_variant(
        &mut self,
        _variant_index: usize,
        variant: &'static str,
    ) -> Result<(), Self::Error> {
        log::trace!("Serializing unit variant: {}", variant);
        self.serialize_str(variant)
    }

    fn start_object(&mut self, _len: Option<usize>) -> Result<(), Self::Error> {
        log::trace!("Starting object");
        // Objects in KDL are represented as nodes with children
        Ok(())
    }

    fn serialize_field_name(&mut self, name: &'static str) -> Result<(), Self::Error> {
        log::trace!("Serializing field name: {}", name);
        // Store the field name for the next value
        self.current_key = Some(name.to_string());
        Ok(())
    }

    fn start_array(&mut self, _len: Option<usize>) -> Result<(), Self::Error> {
        log::trace!("Starting array");
        // Arrays in KDL are represented as multiple arguments
        Ok(())
    }

    fn start_map(&mut self, _len: Option<usize>) -> Result<(), Self::Error> {
        log::trace!("Starting map");
        // Maps in KDL are represented as properties
        Ok(())
    }
}

/// Serialize a value to a KDL string using facet-serialize.
pub fn to_string<'a, T>(value: &'a T) -> Result<String, KdlSerializeError>
where
    T: Serialize<'a>,
{
    let mut serializer = KdlSerializer::new();
    // For now, we'll create a root node for the serialization
    serializer.current_node = Some(KdlNode::new("root"));
    value.serialize(&mut serializer)?;

    // Add the root node to the document
    if let Some(node) = serializer.current_node.take() {
        serializer.document.nodes_mut().push(node);
    }

    Ok(serializer.into_string())
}
