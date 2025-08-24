#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

// cf. facet-toml/facet-json for examples

mod serialize;
pub use serialize::{KdlSerializeError, KdlSerializer, to_string};

use std::{
    error::Error,
    fmt::{self, Display},
};

use facet_core::{Def, Facet, Type, UserType};
use facet_reflect::{Partial, ReflectError};
use kdl::{KdlDocument, KdlError as KdlParseError};

// QUESTION: Any interest in making something a bit like `strum` with `facet`? Always nice to have an easy way to get
// the names of enum variants as strings!

// DESIGN: Like `facet-toml`, this crate currently fully parses KDL into an AST before doing any deserialization. In the
// long-term, I think it's important that the code in `facet-kdl` stays as minimally complex and easy to maintain as
// possible — I'd like to get "free" KDL format / parsing updates from `kdl-rs`, and a "free" derive macro from `facet`.
// For this prototype then, I'm really going to try to avoid any premature optimisation — I'll try to take inspiration
// from `facet-toml` and split things into easy-to-understand functions that I can call recursively as I crawl down the
// KDL AST. After I'm happy with the API and have a really solid set of tests, we can look into making some more
// optimisations, like flattening this recursive structure into something more iterative / imparative (as in
// `facet-json`) or parsing things more incrementally by using `KdlNode::parse()` or `KdlEntry::parse`.

// TODO: Need to actually add some shared information here so it's not just a useless wrapper...

/// Error type for KDL deserialization.
#[derive(Debug)]
pub struct KdlError {
    kind: KdlErrorKind,
}

impl Display for KdlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        let kind = &self.kind;
        write!(f, "{kind}")
    }
}
impl Error for KdlError {}

// FIXME: Replace this with a proper constructor once there is other information to put into `KdlError`!
impl<K: Into<KdlErrorKind>> From<K> for KdlError {
    fn from(value: K) -> Self {
        let kind = value.into();
        KdlError { kind }
    }
}

#[derive(Debug)]
enum KdlErrorKind {
    InvalidDocumentShape(&'static Def),
    MissingNodes(Vec<String>),
    Parse(KdlParseError),
    Reflect(ReflectError),
}

impl Display for KdlErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KdlErrorKind::InvalidDocumentShape(def) => {
                write!(f, "invalid shape {def:#?} — needed... TODO")
            }
            KdlErrorKind::MissingNodes(expected) => write!(f, "failed to find node {expected:?}"),
            KdlErrorKind::Parse(kdl_error) => write!(f, "{kdl_error}"),
            KdlErrorKind::Reflect(reflect_error) => write!(f, "{reflect_error}"),
        }
    }
}

impl From<KdlParseError> for KdlErrorKind {
    fn from(value: KdlParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<ReflectError> for KdlErrorKind {
    fn from(value: ReflectError) -> Self {
        Self::Reflect(value)
    }
}

// FIXME: I'm not sure what to name this...
#[allow(dead_code)]
struct KdlDeserializer<'input> {
    // FIXME: Also no clue what fields it should have, if it should exist at all...
    kdl: &'input str,
}

type Result<T> = std::result::Result<T, KdlError>;

impl<'input, 'facet> KdlDeserializer<'input> {
    fn deserialize_value(
        &mut self,
        wip: &mut Partial<'facet>,
        value: &kdl::KdlValue,
    ) -> Result<()> {
        log::trace!("Deserializing value: {:?}", value);
        log::trace!("Current shape: {:?}", wip.shape());

        // Check if it's a scalar or undefined type
        match &wip.shape().def {
            facet_core::Def::Scalar => {
                // For scalar types, we need to handle them directly
                self.deserialize_scalar_value(wip, value)?;
            }
            facet_core::Def::Undefined => {
                // Undefined types like String need special handling
                if let kdl::KdlValue::String(s) = value {
                    log::trace!("Handling undefined type with value: {}", s);

                    // Check if this is a String type and try multiple approaches
                    if wip.shape().type_identifier == "String" {
                        // Try set_from_function to directly write to memory
                        log::trace!("Trying set_from_function for String");
                        let string_value = s.clone();
                        if wip
                            .set_from_function(move |ptr| {
                                unsafe {
                                    let string_ptr = ptr.as_mut_byte_ptr() as *mut String;
                                    core::ptr::write(string_ptr, string_value);
                                }
                                Ok(())
                            })
                            .is_ok()
                        {
                            log::trace!("set_from_function succeeded");
                            return Ok(());
                        }
                    }

                    // Try different approaches for other undefined types
                    // 1. Try direct set
                    if wip.set(s.clone()).is_ok() {
                        log::trace!("Direct set succeeded");
                        return Ok(());
                    }

                    // 2. Last resort: parse_from_str
                    log::trace!("Falling back to parse_from_str");
                    if wip.parse_from_str(s).is_ok() {
                        log::trace!("parse_from_str succeeded");
                        return Ok(());
                    }

                    log::error!("Failed to set undefined type value: {}", s);
                    return Err(KdlError::from(KdlErrorKind::InvalidDocumentShape(
                        &wip.shape().def,
                    )));
                } else {
                    log::warn!("Non-string value for undefined type: {:?}", value);
                    return Err(KdlError::from(KdlErrorKind::InvalidDocumentShape(
                        &wip.shape().def,
                    )));
                }
            }
            _ => {
                // For non-scalar types, we might need to handle them differently
                log::warn!("Non-scalar type encountered: {:?}", wip.shape().def);
                return Err(KdlError::from(KdlErrorKind::InvalidDocumentShape(
                    &wip.shape().def,
                )));
            }
        }

        Ok(())
    }

    fn deserialize_scalar_value(
        &mut self,
        wip: &mut Partial<'facet>,
        value: &kdl::KdlValue,
    ) -> Result<()> {
        log::trace!("Deserializing scalar value: {:?}", value);

        use facet_reflect::ScalarType;
        use std::borrow::Cow;

        // Get the scalar type from the shape
        let scalar_type = ScalarType::try_from_shape(wip.shape()).ok_or_else(|| {
            KdlError::from(KdlErrorKind::Reflect(
                facet_reflect::ReflectError::OperationFailed {
                    operation: "Not a scalar type",
                    shape: wip.shape(),
                },
            ))
        })?;

        match (scalar_type, value) {
            // String types
            (ScalarType::String, kdl::KdlValue::String(s)) => {
                wip.set(s.clone())?;
            }
            (ScalarType::CowStr, kdl::KdlValue::String(s)) => {
                wip.set(Cow::Owned::<str>(s.clone()))?;
            }

            // Boolean
            (ScalarType::Bool, kdl::KdlValue::Bool(b)) => {
                wip.set(*b)?;
            }

            // Integer types
            (ScalarType::I8, kdl::KdlValue::Integer(n))
                if *n >= i8::MIN as i128 && *n <= i8::MAX as i128 =>
            {
                wip.set(*n as i8)?;
            }
            (ScalarType::I16, kdl::KdlValue::Integer(n))
                if *n >= i16::MIN as i128 && *n <= i16::MAX as i128 =>
            {
                wip.set(*n as i16)?;
            }
            (ScalarType::I32, kdl::KdlValue::Integer(n))
                if *n >= i32::MIN as i128 && *n <= i32::MAX as i128 =>
            {
                wip.set(*n as i32)?;
            }
            (ScalarType::I64, kdl::KdlValue::Integer(n))
                if *n >= i64::MIN as i128 && *n <= i64::MAX as i128 =>
            {
                wip.set(*n as i64)?;
            }
            (ScalarType::I128, kdl::KdlValue::Integer(n)) => {
                wip.set(*n)?;
            }
            (ScalarType::ISize, kdl::KdlValue::Integer(n))
                if *n >= isize::MIN as i128 && *n <= isize::MAX as i128 =>
            {
                wip.set(*n as isize)?;
            }

            // Unsigned integer types
            (ScalarType::U8, kdl::KdlValue::Integer(n)) if *n >= 0 && *n <= u8::MAX as i128 => {
                wip.set(*n as u8)?;
            }
            (ScalarType::U16, kdl::KdlValue::Integer(n)) if *n >= 0 && *n <= u16::MAX as i128 => {
                wip.set(*n as u16)?;
            }
            (ScalarType::U32, kdl::KdlValue::Integer(n)) if *n >= 0 && *n <= u32::MAX as i128 => {
                wip.set(*n as u32)?;
            }
            (ScalarType::U64, kdl::KdlValue::Integer(n)) if *n >= 0 && *n <= u64::MAX as i128 => {
                wip.set(*n as u64)?;
            }
            (ScalarType::U128, kdl::KdlValue::Integer(n)) if *n >= 0 => {
                wip.set(*n as u128)?;
            }
            (ScalarType::USize, kdl::KdlValue::Integer(n))
                if *n >= 0 && *n <= usize::MAX as i128 =>
            {
                wip.set(*n as usize)?;
            }

            // Float types
            (ScalarType::F32, kdl::KdlValue::Float(f)) => {
                wip.set(*f as f32)?;
            }
            (ScalarType::F64, kdl::KdlValue::Float(f)) => {
                wip.set(*f)?;
            }

            // Also allow integers to be converted to floats
            (ScalarType::F32, kdl::KdlValue::Integer(n)) => {
                wip.set(*n as f32)?;
            }
            (ScalarType::F64, kdl::KdlValue::Integer(n)) => {
                wip.set(*n as f64)?;
            }

            // Char type
            (ScalarType::Char, kdl::KdlValue::String(s)) if s.len() == 1 => {
                wip.set(s.chars().next().unwrap())?;
            }

            // Handle null for any type (will use default)
            (_, kdl::KdlValue::Null) => {
                wip.set_default()?;
            }

            // For types that might implement FromStr
            (_, kdl::KdlValue::String(s)) => {
                // Try to parse from string as a fallback
                wip.parse_from_str(s)?;
            }

            _ => {
                return Err(KdlError::from(KdlErrorKind::Reflect(
                    facet_reflect::ReflectError::OperationFailed {
                        operation: "Type mismatch in scalar deserialization",
                        shape: wip.shape(),
                    },
                )));
            }
        }

        Ok(())
    }

    fn deserialize_property(
        &mut self,
        wip: &mut Partial<'facet>,
        name: &str,
        value: &kdl::KdlValue,
    ) -> Result<()> {
        log::trace!("Deserializing property '{}': {:?}", name, value);

        // Begin the field by name
        wip.begin_field(name)?;

        // Check if we're dealing with a String type that needs parse_from_str
        // String types are Scalar with is_from_str() true
        if wip.shape().type_identifier == "String" && wip.shape().is_from_str() {
            if let kdl::KdlValue::String(s) = value {
                wip.parse_from_str(s)?;
                // parse_from_str completes the field, so we don't need to call end()
                return Ok(());
            }
        }

        // For other types (including numbers), use the normal flow
        // Note: deserialize_value with scalar types will call wip.set() which automatically completes the frame
        self.deserialize_value(wip, value)?;

        // Don't call end() here - wip.set() in scalar types already completes the frame

        Ok(())
    }

    fn deserialize_children(
        &mut self,
        wip: &mut Partial<'facet>,
        children: &kdl::KdlDocument,
    ) -> Result<()> {
        log::trace!("Deserializing children nodes");

        for child_node in children.nodes() {
            log::trace!("Processing child node: {:#?}", child_node.name());

            // Process each child node recursively
            wip.begin_field(child_node.name().value())?;

            // Process the child node's entries
            let mut arg_index = 0;
            for entry in child_node.entries() {
                if entry.name().is_none() {
                    wip.begin_nth_field(arg_index)?;
                    self.deserialize_value(wip, entry.value())?;
                    wip.end()?;
                    arg_index += 1;
                } else {
                    self.deserialize_property(wip, entry.name().unwrap().value(), entry.value())?;
                }
            }

            // Process nested children if any
            if let Some(nested_children) = child_node.children() {
                self.deserialize_children(wip, nested_children)?;
            }

            wip.end()?;
        }

        Ok(())
    }

    fn from_str<T: Facet<'facet>>(kdl: &'input str) -> Result<T> {
        log::trace!("Entering `from_str` method");

        // PERF: This definitely isn't zero-copy, so it might be worth seeing if that's something that can be added to
        // `kdl-rs` at some point in the future?
        // PERF: Would be be better / quicker if I did this parsing incrementally? Using information from the `Partial` to
        // decide when to call `KdlNode::parse` and `KdlEntry::parse`? Probably would be if I'm only trying to parse
        // some of the KDL text, but I'm not so sure otherwise? Will need benchmarking...
        let document: KdlDocument = dbg!(kdl.parse()?);
        log::trace!("KDL parsed");

        let mut typed_partial = Partial::alloc::<T>().expect("failed to allocate");
        log::trace!(
            "Allocated WIP for type {}",
            typed_partial.inner_mut().shape()
        );

        {
            let wip = typed_partial.inner_mut();
            Self { kdl }.deserialize_document(wip, document)?;
        }

        let boxed_value = typed_partial.build()?;
        log::trace!("WIP fully built");
        log::trace!("Type of WIP unerased");

        Ok(*boxed_value)
    }

    fn deserialize_document(
        &mut self,
        wip: &mut Partial<'facet>,
        document: KdlDocument,
    ) -> Result<()> {
        log::trace!("Entering `deserialize_document` method");

        // First check the type system (Type)
        if let Type::User(UserType::Struct(struct_def)) = &wip.shape().ty {
            log::trace!("Document `Partial` is a struct: {struct_def:#?}");
            // A struct can be at the top level
            // We'll handle it as a valid document structure
            log::trace!("Processing struct at document level");
            return self.deserialize_node(wip, document);
        }

        // Fall back to the def system for backward compatibility
        let def = wip.shape().def;
        match def {
            // TODO: Valid if the list contains only enums with single fields that can be parsed as entries?
            Def::List(_list_def) => todo!(),
            _ => todo!(),
        }
    }

    fn deserialize_node(&mut self, wip: &mut Partial<'facet>, document: KdlDocument) -> Result<()> {
        log::trace!("Entering `deserialize_node` method");

        // Process all nodes in the document
        for node in document.nodes() {
            log::trace!("Processing node: {:#?}", node.name());

            // Check if this is a property (no children) or a child node
            if node.children().is_none() && !node.entries().is_empty() {
                // This looks like properties at the root level
                for entry in node.entries() {
                    if let Some(name) = entry.name() {
                        // Named property
                        self.deserialize_property(wip, name.value(), entry.value())?;
                    } else {
                        // Positional property (using node name as field name)
                        self.deserialize_property(wip, node.name().value(), entry.value())?;
                    }
                }
            } else {
                // Original logic for child nodes
                // Try to match the node name with a field
                wip.begin_field(node.name().value())?;
                log::trace!(
                    "Node matched expected child; New def: {:#?}",
                    wip.shape().def
                );

                // Process entries (arguments and properties)
                let mut arg_index = 0;
                for entry in node.entries() {
                    log::trace!("Processing entry: {entry:#?}");

                    if entry.name().is_none() {
                        // This is an argument - need to begin the field by index
                        wip.begin_nth_field(arg_index)?;
                        self.deserialize_value(wip, entry.value())?;
                        wip.end()?;
                        arg_index += 1;
                    } else {
                        // This is a property
                        self.deserialize_property(
                            wip,
                            entry.name().unwrap().value(),
                            entry.value(),
                        )?;
                    }
                }
            }

            // Process child nodes if any
            if let Some(children) = node.children() {
                self.deserialize_children(wip, children)?;
            }

            // Finish processing this field
            wip.end()?;
        }

        Ok(())
    }
}

/// Deserialize a value of type `T` from a KDL string.
///
/// Returns a [`KdlError`] if the input KDL is invalid or doesn't match `T`.
///
/// # Example
/// ```ignore
/// let kdl = r#"
/// my_struct {
///     field1 "value"
///     field2 42
/// }
/// "#;
/// let val: MyStruct = from_str(kdl)?;
/// ```
pub fn from_str<'input, 'facet: 'shape, 'shape, T>(kdl: &'input str) -> Result<T>
where
    T: Facet<'facet>,
    'input: 'facet,
{
    log::trace!("Entering `from_str` function");

    KdlDeserializer::from_str(kdl)
}
