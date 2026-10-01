//! Run configuration parameters: named, typed, immutable key/value pairs.

use crate::codec::{write_i64, write_string, Canonical, Decoder};
use crate::error::{ErrorKind, MathError};
use crate::value::ValidFloat;

/// A single typed configuration value.
///
/// Filled by test-only capture: plain metadata, never secret material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConfigValue {
    /// A finite floating-point value.
    Float(ValidFloat),
    /// A signed integer value.
    Integer(i64),
    /// A UTF-8 text value.
    Text(String),
    /// A boolean value.
    Logical(bool),
}

impl Canonical for ConfigValue {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        match self {
            ConfigValue::Float(value) => {
                out.push(0);
                value.encode(out)?;
            }
            ConfigValue::Integer(value) => {
                out.push(1);
                write_i64(*value, out);
            }
            ConfigValue::Text(value) => {
                out.push(2);
                write_string(value, out)?;
            }
            ConfigValue::Logical(value) => {
                out.push(3);
                out.push(u8::from(*value));
            }
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => Ok(ConfigValue::Float(dec.read_valid_float()?)),
            1 => Ok(ConfigValue::Integer(dec.read_i64()?)),
            2 => Ok(ConfigValue::Text(dec.read_string()?)),
            3 => match dec.read_u8()? {
                0 => Ok(ConfigValue::Logical(false)),
                1 => Ok(ConfigValue::Logical(true)),
                _ => Err(MathError::new(
                    ErrorKind::Reserved,
                    "unknown logical payload byte",
                )),
            },
            _ => Err(MathError::new(
                ErrorKind::Reserved,
                "unknown ConfigValue tag",
            )),
        }
    }
}

/// A named configuration parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConfigParam {
    name: String,
    value: ConfigValue,
}

impl ConfigParam {
    /// Build a configuration parameter.
    ///
    /// # Errors
    /// [`ErrorKind::EmptyCollection`] if `name` is empty.
    pub fn new(name: String, value: ConfigValue) -> Result<Self, MathError> {
        if name.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "config parameter name must not be empty",
            ));
        }
        Ok(Self { name, value })
    }

    /// The parameter name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The parameter value.
    #[must_use]
    pub fn value(&self) -> &ConfigValue {
        &self.value
    }
}

impl Canonical for ConfigParam {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_string(&self.name, out)?;
        self.value.encode(out)?;
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let name = dec.read_string()?;
        let value = ConfigValue::decode(dec)?;
        ConfigParam::new(name, value)
    }
}
