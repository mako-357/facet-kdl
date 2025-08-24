#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

// cf. facet-toml/facet-json for examples

use std::{
    error::Error,
    fmt::{self, Display},
};

use facet_core::{Def, Facet, FieldFlags, Type, UserType};
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

        // Handle scalar types
        if let facet_core::Def::Scalar = wip.shape().def {
            // For scalar types, we need to handle them directly
            // The actual scalar type information is in the shape
            self.deserialize_scalar_value(wip, value)?;
        } else {
            // For non-scalar types, we might need to handle them differently
            log::warn!("Non-scalar type encountered: {:?}", wip.shape().def);
            return Err(KdlError::from(KdlErrorKind::InvalidDocumentShape(
                &wip.shape().def,
            )));
        }

        Ok(())
    }

    fn deserialize_scalar_value(
        &mut self,
        wip: &mut Partial<'facet>,
        value: &kdl::KdlValue,
    ) -> Result<()> {
        log::trace!("Deserializing scalar value: {:?}", value);

        // For scalars, we need to check the actual type and deserialize accordingly
        // This is a simplified version - in reality we'd need to inspect wip.shape() to get the exact scalar type
        match value {
            kdl::KdlValue::String(s) => {
                // Set string value
                let value = s.clone();
                wip.set(value)?;
            }
            kdl::KdlValue::Bool(b) => {
                // Set boolean value
                wip.set(*b)?;
            }
            kdl::KdlValue::Integer(n) => {
                // For integers, we try to set based on the size
                // This is simplified - we'd need to check the actual type
                let value = *n as i64;
                wip.set(value)?;
            }
            kdl::KdlValue::Float(f) => {
                // Set float value
                wip.set(*f)?;
            }
            kdl::KdlValue::Null => {
                // Handle null values - this would typically be for Option types
                wip.set_default()?;
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

        // Deserialize the value
        self.deserialize_value(wip, value)?;

        // End the field
        wip.end()?;

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
            // QUESTION: Would be be possible, once we allow custom types, to make all attributes arbitrary? With
            // the sort of general tool that `facet` is, I think it might actually be best if we didn't try to
            // "bake-in" anything like sensitive, default, skip, etc...
            let is_valid_toplevel = struct_def
                .fields
                .iter()
                .all(|field| field.flags.contains(FieldFlags::CHILD));
            log::trace!("WIP represents a valid top-level: {is_valid_toplevel}");

            if is_valid_toplevel {
                // FIXME: At this point I'm really not sure where function boundaries should be... It's a messy disaster
                // whilst I try to work that out...
                // FIXME: For example, this feels like maybe it should take a `KdlNode` and not a `KdlDocument`?
                return self.deserialize_node(wip, document);
            } else {
                return Err(KdlErrorKind::InvalidDocumentShape(&wip.shape().def).into());
            }
        }

        // Fall back to the def system for backward compatibility
        let def = wip.shape().def;
        match def {
            // TODO: Valid if the list contains only enums with single fields that can be parsed as entries?
            Def::List(_list_def) => todo!(),
            _ => todo!(),
        }
    }

    fn deserialize_node(
        &mut self,
        wip: &mut Partial<'facet>,
        mut document: KdlDocument,
    ) -> Result<()> {
        log::trace!("Entering `deserialize_node` method");

        // Process all nodes in the document
        for node in document.nodes() {
            log::trace!("Processing node: {:#?}", node.name());

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
                    self.deserialize_property(wip, entry.name().unwrap().value(), entry.value())?;
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
