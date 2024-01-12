//! NBT format serialization and deserialization.

use std::io::{self, Read, Write};
use std::collections::BTreeMap;
use std::fmt;

use crate::io::{ReadJavaExt, WriteJavaExt};


const NBT_BYTE       : i8 = 1;
const NBT_SHORT      : i8 = 2;
const NBT_INT        : i8 = 3;
const NBT_LONG       : i8 = 4;
const NBT_FLOAT      : i8 = 5;
const NBT_DOUBLE     : i8 = 6;
const NBT_BYTE_ARRAY : i8 = 7;
const NBT_STRING     : i8 = 8;
const NBT_LIST       : i8 = 9;
const NBT_COMPOUND   : i8 = 10;


/// A generic NBT tag, this structure has a size of 32 bytes. 
#[derive(Clone, PartialEq)]
pub enum Nbt {
    // Primitive tags.
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    // Container tags.
    List(Vec<Nbt>),
    Compound(NbtCompound),
}

/// An abstract NBT compound type that hides the internal implementation of the mapping.
#[derive(Clone, PartialEq)]
pub struct NbtCompound {
    inner: BTreeMap<String, Nbt>,
}

/// This macro is used to generate from/into implementations from inner types to
/// NBT variant instance.
macro_rules! impl_nbt_from {
    ( $variant:ident ( $type:ty ) slice ) => {
        impl_nbt_from!( $variant ( $type ) );
        impl<'a> From<&'a [$type]> for Nbt {
            fn from(value: &'a [$type]) -> Self {
                Nbt::List(value.iter().map(|&value| Nbt::$variant(value as _)).collect())
            }
        }
    };
    ( $variant:ident ( $type:ty ) ) => {
        impl From<$type> for Nbt {
            fn from(value: $type) -> Self {
                Nbt::$variant(value as _)
            }
        }
    }
}

impl_nbt_from!(Byte(bool) slice);
impl_nbt_from!(Byte(i8) slice);
impl_nbt_from!(Byte(u8) slice);
impl_nbt_from!(Short(i16) slice);
impl_nbt_from!(Short(u16) slice);
impl_nbt_from!(Int(i32) slice);
impl_nbt_from!(Int(u32) slice);
impl_nbt_from!(Long(i64) slice);
impl_nbt_from!(Long(u64) slice);
impl_nbt_from!(Float(f32) slice);
impl_nbt_from!(Double(f64) slice);
impl_nbt_from!(ByteArray(Vec<u8>));
impl_nbt_from!(String(String));
impl_nbt_from!(List(Vec<Nbt>));
impl_nbt_from!(Compound(NbtCompound));

impl<'a> From<&'a str> for Nbt {
    fn from(value: &'a str) -> Self {
        Nbt::String(value.to_string())
    }
}

/// Basic methods to interpret a tag as its inner type if possible.
impl Nbt {

    #[inline]
    pub fn as_boolean(&self) -> Option<bool> {
        self.as_byte().map(|b| b != 0)
    }

    #[inline]
    pub fn as_byte(&self) -> Option<i8> {
        match *self {
            Self::Byte(n) => Some(n),
            _ => None
        }
    }

    #[inline]
    pub fn as_short(&self) -> Option<i16> {
        match *self {
            Self::Short(n) => Some(n),
            _ => None
        }
    }

    #[inline]
    pub fn as_int(&self) -> Option<i32> {
        match *self {
            Self::Int(n) => Some(n),
            _ => None
        }
    }

    #[inline]
    pub fn as_long(&self) -> Option<i64> {
        match *self {
            Self::Long(n) => Some(n),
            _ => None
        }
    }

    #[inline]
    pub fn as_float(&self) -> Option<f32> {
        match *self {
            Self::Float(n) => Some(n),
            _ => None
        }
    }

    #[inline]
    pub fn as_double(&self) -> Option<f64> {
        match *self {
            Self::Double(n) => Some(n),
            _ => None
        }
    }

    #[inline]
    pub fn as_byte_array(&self) -> Option<&[u8]> {
        match self {
            Self::ByteArray(buf) => Some(&buf[..]),
            _ => None
        }
    }

    #[inline]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(string) => Some(string.as_str()),
            _ => None
        }
    }

    #[inline]
    pub fn as_list(&self) -> Option<&[Nbt]> {
        match self {
            Self::List(list) => Some(&list[..]),
            _ => None
        }
    }

    #[inline]
    pub fn as_compound(&self) -> Option<&NbtCompound> {
        match self {
            Self::Compound(comp) => Some(comp),
            _ => None
        }
    }

    pub fn parse(&self) -> NbtParse<'_> {
        NbtParse { inner: self, path: String::new() }
    }

}

/// Basic methods to create and manage keys in a compound.
impl NbtCompound {

    pub const fn new() -> Self {
        Self { inner: BTreeMap::new() }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    pub fn insert(&mut self, key: impl Into<String>, tag: impl Into<Nbt>) {
        self.inner.insert(key.into(), tag.into());
    }

    #[inline]
    pub fn get(&self, key: &str) -> Option<&Nbt> {
        self.inner.get(key)
    }

    #[inline]
    pub fn get_boolean(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Nbt::as_boolean)
    }

    #[inline]
    pub fn get_byte(&self, key: &str) -> Option<i8> {
        self.get(key).and_then(Nbt::as_byte)
    }

    #[inline]
    pub fn get_short(&self, key: &str) -> Option<i16> {
        self.get(key).and_then(Nbt::as_short)
    }

    #[inline]
    pub fn get_int(&self, key: &str) -> Option<i32> {
        self.get(key).and_then(Nbt::as_int)
    }

    #[inline]
    pub fn get_long(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Nbt::as_long)
    }

    #[inline]
    pub fn get_float(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(Nbt::as_float)
    }

    #[inline]
    pub fn get_double(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(Nbt::as_double)
    }

    #[inline]
    pub fn get_byte_array(&self, key: &str) -> Option<&[u8]> {
        self.get(key).and_then(Nbt::as_byte_array)
    }

    #[inline]
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Nbt::as_string)
    }

    #[inline]
    pub fn get_list(&self, key: &str) -> Option<&[Nbt]> {
        self.get(key).and_then(Nbt::as_list)
    }

    #[inline]
    pub fn get_compound(&self, key: &str) -> Option<&NbtCompound> {
        self.get(key).and_then(Nbt::as_compound)
    }

}

/// Manual debug implement to shrink the potential huge byte arrays.
impl fmt::Debug for Nbt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Byte(n) => f.debug_tuple("Byte").field(n).finish(),
            Self::Short(n) => f.debug_tuple("Short").field(n).finish(),
            Self::Int(n) => f.debug_tuple("Int").field(n).finish(),
            Self::Long(n) => f.debug_tuple("Long").field(n).finish(),
            Self::Float(n) => f.debug_tuple("Float").field(n).finish(),
            Self::Double(n) => f.debug_tuple("Double").field(n).finish(),
            Self::ByteArray(buf) => {
                f.debug_tuple("ByteArray")
                    .field(&format_args!("({}) {:X?}...", buf.len(), &buf[..buf.len().min(10)]))
                    .finish()
            }
            Self::String(string) => f.debug_tuple("String").field(string).finish(),
            Self::List(list) => f.debug_tuple("List").field(list).finish(),
            Self::Compound(compound) => f.debug_tuple("Compound").field(&compound.inner).finish(),
        }
    }
}

/// Deserialize a NBT tag from a reader.
pub fn from_reader(mut reader: impl Read) -> Result<Nbt, NbtError> {

    let type_id = reader.read_java_byte()?;
    if type_id == 0 {
        // We should not get a end tag directly.
        return Err(NbtError::IllegalTagType);
    }

    let _key = reader.read_java_string8()?;
    from_reader_with_type(&mut reader, type_id)
    
}

/// Internal function to read a NBT tag of a specific type.
fn from_reader_with_type(reader: &mut impl Read, type_id: i8) -> Result<Nbt, NbtError> {
    Ok(match type_id {
        NBT_BYTE => Nbt::Byte(reader.read_java_byte()?),
        NBT_SHORT => Nbt::Short(reader.read_java_short()?),
        NBT_INT => Nbt::Int(reader.read_java_int()?),
        NBT_LONG => Nbt::Long(reader.read_java_long()?),
        NBT_FLOAT => Nbt::Float(reader.read_java_float()?),
        NBT_DOUBLE => Nbt::Double(reader.read_java_double()?),
        NBT_BYTE_ARRAY => {
            
            let len: usize = reader.read_java_int()?.try_into().map_err(|_| NbtError::IllegalLength)?;
            let mut buf = vec![0u8; len as usize];
            reader.read_exact(&mut buf)?;
            Nbt::ByteArray(buf)

        }
        NBT_STRING => Nbt::String(reader.read_java_string8()?),
        NBT_LIST => {

            // NOTE: A list can contain a single type.
            let type_id = reader.read_java_byte()?;
            let len: usize = reader.read_java_int()?.try_into().map_err(|_| NbtError::IllegalLength)?;

            let mut list = Vec::with_capacity(len as usize);
            for _ in 0..len {
                list.push(from_reader_with_type(reader, type_id)?);
            }

            Nbt::List(list)

        }
        NBT_COMPOUND => {

            let mut map = BTreeMap::new();

            loop {

                let type_id = reader.read_java_byte()?;
                if type_id == 0 {
                    break Nbt::Compound(NbtCompound { inner: map });  // End tag.
                }

                let key = reader.read_java_string8()?;
                map.insert(key, from_reader_with_type(reader, type_id)?);

            }

        }
        _ => return Err(NbtError::IllegalTagType),
    })
}

/// Serialize a NBT tag into a writer.
pub fn to_writer(mut writer: impl Write, tag: &Nbt) -> Result<(), NbtError> {
    writer.write_java_byte(get_nbt_type_id(tag))?;
    writer.write_java_string8("")?; // Root tag has empty key.
    to_writer_raw(&mut writer, tag)
}

/// Internal function to write a NBT tag content.
fn to_writer_raw(writer: &mut impl Write, tag: &Nbt) -> Result<(), NbtError> {

    match *tag {
        Nbt::Byte(n) => writer.write_java_byte(n)?,
        Nbt::Short(n) => writer.write_java_short(n)?,
        Nbt::Int(n) => writer.write_java_int(n)?,
        Nbt::Long(n) => writer.write_java_long(n)?,
        Nbt::Float(n) => writer.write_java_float(n)?,
        Nbt::Double(n) => writer.write_java_double(n)?,
        Nbt::ByteArray(ref buf) => {
            
            let len: i32 = buf.len().try_into().map_err(|_| NbtError::IllegalLength)?;
            writer.write_java_int(len)?;
            writer.write_all(&buf)?;

        }
        Nbt::String(ref string) => writer.write_java_string8(&string)?,
        Nbt::List(ref list) => {

            let len: i32 = list.len().try_into().map_err(|_| NbtError::IllegalLength)?;
            let type_id = list.first().map(get_nbt_type_id).unwrap_or(NBT_BYTE);
            writer.write_java_byte(type_id)?;
            writer.write_java_int(len)?;

            for item in list {
                if get_nbt_type_id(item) != type_id {
                    return Err(NbtError::IllegalTagType);
                }
                to_writer_raw(writer, item)?;
            }

        }
        Nbt::Compound(ref compound) => {
            
            for (key, tag) in &compound.inner {
                writer.write_java_byte(get_nbt_type_id(tag))?;
                writer.write_java_string8(&key)?;
                to_writer_raw(writer, tag)?;
            }
        
            writer.write_java_byte(0)?;

        }
    }

    Ok(())

}

/// Internal function to get the NBT type id of a tag.
fn get_nbt_type_id(tag: &Nbt) -> i8 {
    match tag {
        Nbt::Byte(_) => NBT_BYTE,
        Nbt::Short(_) => NBT_SHORT,
        Nbt::Int(_) => NBT_INT,
        Nbt::Long(_) => NBT_LONG,
        Nbt::Float(_) => NBT_FLOAT,
        Nbt::Double(_) => NBT_DOUBLE,
        Nbt::ByteArray(_) => NBT_BYTE_ARRAY,
        Nbt::String(_) => NBT_STRING,
        Nbt::List(_) => NBT_LIST,
        Nbt::Compound(_) => NBT_COMPOUND,
    }
}

/// Error type used together with `RegionResult` for every call on region file methods.
#[derive(thiserror::Error, Debug)]
pub enum NbtError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("illegal tag type")]
    IllegalTagType,
    #[error("illegal decoded length")]
    IllegalLength,
}


/// Parsing utility structure for anonymous NBT.
#[derive(Clone)]
pub struct NbtParse<'nbt> {
    /// Reference to the anonymous NBT.
    inner: &'nbt Nbt,
    /// Current path being parsed, used to return relevant errors.
    path: String,
}

impl<'nbt> NbtParse<'nbt> {
    
    #[inline]
    fn make_error(self, expected: &'static str) -> NbtParseError {
        NbtParseError::new(self.path, expected)
    }

    // #[inline]
    // pub fn as_custom<T, F>(self, func: F, expected: &'static str) -> Result<T, NbtParseError>
    // where
    //     F: FnOnce() -> Option<T>,
    // {
    //     func()
    // }

    #[inline]
    pub fn as_boolean(self) -> Result<bool, NbtParseError> {
        self.as_byte().map(|b| b != 0)
    }

    #[inline]
    pub fn as_byte(self) -> Result<i8, NbtParseError> {
        self.inner.as_byte().ok_or_else(|| self.make_error("byte"))
    }

    #[inline]
    pub fn as_short(self) -> Result<i16, NbtParseError> {
        self.inner.as_short().ok_or_else(|| self.make_error("short"))
    }

    #[inline]
    pub fn as_int(self) -> Result<i32, NbtParseError> {
        self.inner.as_int().ok_or_else(|| self.make_error("int"))
    }

    #[inline]
    pub fn as_long(self) -> Result<i64, NbtParseError> {
        self.inner.as_long().ok_or_else(|| self.make_error("long"))
    }

    #[inline]
    pub fn as_float(self) -> Result<f32, NbtParseError> {
        self.inner.as_float().ok_or_else(|| self.make_error("float"))
    }

    #[inline]
    pub fn as_double(self) -> Result<f64, NbtParseError> {
        self.inner.as_double().ok_or_else(|| self.make_error("double"))
    }

    #[inline]
    pub fn as_byte_array(self) -> Result<&'nbt [u8], NbtParseError> {
        self.inner.as_byte_array().ok_or_else(|| self.make_error("byte array"))
    }

    #[inline]
    pub fn as_string(self) -> Result<&'nbt str, NbtParseError> {
        self.inner.as_string().ok_or_else(|| self.make_error("string"))
    }

    #[inline]
    pub fn as_list(self) -> Result<NbtListParse<'nbt>, NbtParseError> {
        match self.inner.as_list() {
            Some(inner) => Ok(NbtListParse { inner, path: self.path }),
            None => Err(self.make_error("list"))
        }
    }

    #[inline]
    pub fn as_compound(self) -> Result<NbtCompoundParse<'nbt>, NbtParseError> {
        // If successful we wrap the compound into a parse structure to keep the path.
        match self.inner.as_compound() {
            Some(inner) => Ok(NbtCompoundParse { inner, path: self.path }),
            None => Err(self.make_error("compound"))
        }
    }

    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[inline]
    pub fn inner(&self) -> &'nbt Nbt {
        self.inner
    }

}

/// Parsing utility structure for NBT list.
#[derive(Clone)]
pub struct NbtListParse<'nbt> {
    /// Reference to the parsed NBT data.
    inner: &'nbt [Nbt],
    /// Current path being parsed, used to return relevant errors.
    path: String,
}

impl<'nbt> NbtListParse<'nbt> {

    /// Get an item from its index in this list.
    /// An expected item error is returned if not found.
    pub fn get(&self, index: usize) -> Result<NbtParse<'nbt>, NbtParseError> {
        let path = format!("{}/{index}", self.path);
        match self.inner.get(index) {
            Some(inner) => Ok(NbtParse { inner, path }),
            None => Err(NbtParseError::new(path, "list value"))
        }
    }
    
    #[inline]
    pub fn get_boolean(&self, index: usize) -> Result<bool, NbtParseError> {
        self.get(index).and_then(NbtParse::as_boolean)
    }

    #[inline]
    pub fn get_byte(&self, index: usize) -> Result<i8, NbtParseError> {
        self.get(index).and_then(NbtParse::as_byte)
    }

    #[inline]
    pub fn get_short(&self, index: usize) -> Result<i16, NbtParseError> {
        self.get(index).and_then(NbtParse::as_short)
    }

    #[inline]
    pub fn get_int(&self, index: usize) -> Result<i32, NbtParseError> {
        self.get(index).and_then(NbtParse::as_int)
    }

    #[inline]
    pub fn get_long(&self, index: usize) -> Result<i64, NbtParseError> {
        self.get(index).and_then(NbtParse::as_long)
    }

    #[inline]
    pub fn get_float(&self, index: usize) -> Result<f32, NbtParseError> {
        self.get(index).and_then(NbtParse::as_float)
    }

    #[inline]
    pub fn get_double(&self, index: usize) -> Result<f64, NbtParseError> {
        self.get(index).and_then(NbtParse::as_double)
    }

    #[inline]
    pub fn get_byte_array(&self, index: usize) -> Result<&'nbt [u8], NbtParseError> {
        self.get(index).and_then(NbtParse::as_byte_array)
    }

    #[inline]
    pub fn get_string(&self, index: usize) -> Result<&'nbt str, NbtParseError> {
        self.get(index).and_then(NbtParse::as_string)
    }

    #[inline]
    pub fn get_list(&self, index: usize) -> Result<NbtListParse<'nbt>, NbtParseError> {
        self.get(index).and_then(NbtParse::as_list)
    }

    #[inline]
    pub fn get_compound(&self, index: usize) -> Result<NbtCompoundParse<'nbt>, NbtParseError> {
        self.get(index).and_then(NbtParse::as_compound)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[inline]
    pub fn inner(&self) -> &'nbt [Nbt] {
        self.inner
    }

    pub fn iter(&self) -> impl Iterator<Item = NbtParse<'_>> + '_ {
        self.inner.iter()
            .enumerate()
            .map(|(i, inner)| {
                let path = format!("{}/{i}", self.path);
                NbtParse { inner, path }
            })
    }

}

/// Parsing utility structure for a NBT compound.
#[derive(Clone)]
pub struct NbtCompoundParse<'nbt> {
    /// Reference to the NBT compound.
    inner: &'nbt NbtCompound,
    /// Current path being parsed, used to return relevant errors.
    path: String,
}

impl<'nbt> NbtCompoundParse<'nbt> {

    /// Get a item from its key in this compound. 
    /// An expected item error is returned if not found.
    pub fn get(&self, key: &str) -> Result<NbtParse<'nbt>, NbtParseError> {
        let path = format!("{}/{key}", self.path);
        match self.inner.get(key) {
            Some(inner) => Ok(NbtParse { inner, path }),
            None => Err(NbtParseError::new(path, "compound value"))
        }
    }

    #[inline]
    pub fn get_boolean(&self, key: &str) -> Result<bool, NbtParseError> {
        self.get(key).and_then(NbtParse::as_boolean)
    }

    #[inline]
    pub fn get_byte(&self, key: &str) -> Result<i8, NbtParseError> {
        self.get(key).and_then(NbtParse::as_byte)
    }

    #[inline]
    pub fn get_short(&self, key: &str) -> Result<i16, NbtParseError> {
        self.get(key).and_then(NbtParse::as_short)
    }

    #[inline]
    pub fn get_int(&self, key: &str) -> Result<i32, NbtParseError> {
        self.get(key).and_then(NbtParse::as_int)
    }

    #[inline]
    pub fn get_long(&self, key: &str) -> Result<i64, NbtParseError> {
        self.get(key).and_then(NbtParse::as_long)
    }

    #[inline]
    pub fn get_float(&self, key: &str) -> Result<f32, NbtParseError> {
        self.get(key).and_then(NbtParse::as_float)
    }

    #[inline]
    pub fn get_double(&self, key: &str) -> Result<f64, NbtParseError> {
        self.get(key).and_then(NbtParse::as_double)
    }

    #[inline]
    pub fn get_byte_array(&self, key: &str) -> Result<&'nbt [u8], NbtParseError> {
        self.get(key).and_then(NbtParse::as_byte_array)
    }

    #[inline]
    pub fn get_string(&self, key: &str) -> Result<&'nbt str, NbtParseError> {
        self.get(key).and_then(NbtParse::as_string)
    }

    #[inline]
    pub fn get_list(&self, key: &str) -> Result<NbtListParse<'nbt>, NbtParseError> {
        self.get(key).and_then(NbtParse::as_list)
    }

    #[inline]
    pub fn get_compound(&self, key: &str) -> Result<NbtCompoundParse<'nbt>, NbtParseError> {
        self.get(key).and_then(NbtParse::as_compound)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[inline]
    pub fn inner(&self) -> &'nbt NbtCompound {
        self.inner
    }

}


/// A parsing error as returned by [`NbtParse`] and [`NbtCompoundParse`] wrappers.
#[derive(thiserror::Error, Debug)]
#[error("{}: expected {}", self.path(), self.expected())]
pub struct NbtParseError(Box<NbtParseErrorInner>);

#[derive(Debug)]
struct NbtParseErrorInner {
    /// The path of the tag that caused an error.
    path: String,
    /// The type of item expected at the path.
    expected: &'static str,
}

impl NbtParseError {

    /// Create a new parse error.
    pub fn new(path: String, expected: &'static str) -> Self {
        Self(Box::new(NbtParseErrorInner { path, expected }))
    }

    pub fn path(&self) -> &str {
        &self.0.path
    }

    pub fn expected(&self) -> &'static str {
        self.0.expected
    }

}


#[cfg(test)]
mod tests {

    use std::io::Cursor;
    use super::*;

    fn test_value(tag: impl Into<Nbt>, bytes: &[u8]) {
        
        let tag = tag.into();

        let mut data = Vec::new();
        to_writer(&mut data, &tag).expect("failed to write");
        assert_eq!(data, bytes, "invalid written tag");

        let mut cursor = Cursor::new(bytes);
        let read_tag = from_reader(&mut cursor).expect("failed to read");
        assert_eq!(tag, read_tag, "invalid read tag");
        assert_eq!(cursor.position(), bytes.len() as u64, "not all data has been read");

    }

    #[test]
    fn primitives() {
        test_value(0x12u8,                  &[NBT_BYTE as u8,   0, 0, 0x12]);
        test_value(0x1234u16,               &[NBT_SHORT as u8,  0, 0, 0x12, 0x34]);
        test_value(0x12345678u32,           &[NBT_INT as u8,    0, 0, 0x12, 0x34, 0x56, 0x78]);
        test_value(0x123456789ABCDEF0u64,   &[NBT_LONG as u8,   0, 0, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
        test_value(3141592.5f32,            &[NBT_FLOAT as u8,  0, 0, 0x4A, 0x3F, 0xBF, 0x62]);
        test_value(3141592.5f64,            &[NBT_DOUBLE as u8, 0, 0, 0x41, 0x47, 0xF7, 0xEC, 0x40, 0x00, 0x00, 0x00]);
        test_value(format!("hello"),        &[NBT_STRING as u8, 0, 0, 0, 5, 0x68, 0x65, 0x6C, 0x6C, 0x6F]);
    }

    #[test]
    fn lists() {

        const V0: Nbt = Nbt::Byte(0);
        const V1: Nbt = Nbt::Byte(0x12u8 as _);
        const V2: Nbt = Nbt::Short(0x1234u16 as _);

        test_value(vec![V0; 0], &[NBT_LIST as u8,   0, 0, NBT_BYTE as u8,   0, 0, 0, 0]);
        test_value(vec![V1; 3], &[NBT_LIST as u8,   0, 0, NBT_BYTE as u8,   0, 0, 0, 3, 0x12, 0x12, 0x12]);
        test_value(vec![V2; 2], &[NBT_LIST as u8,   0, 0, NBT_SHORT as u8,  0, 0, 0, 2, 0x12, 0x34, 0x12, 0x34]);

        let mut compound = NbtCompound::new();
        compound.insert("key0", true);
        let compound = Nbt::Compound(compound);

        test_value(vec![compound.clone(), compound], &[
            NBT_LIST as u8,     0, 0, NBT_COMPOUND as u8, 0, 0, 0, 2, // List header
            NBT_BYTE as u8,     0, 4, 0x6B, 0x65, 0x79, 0x30, 0x01, 0, // key0 header + value + terminating byte
            NBT_BYTE as u8,     0, 4, 0x6B, 0x65, 0x79, 0x30, 0x01, 0, // key0 header + value + terminating byte
        ]);

    }

    #[test]
    #[should_panic]
    fn lists_err() {
        
        const V1: Nbt = Nbt::Byte(0x12u8 as _);
        const V2: Nbt = Nbt::Short(0x1234u16 as _);

        test_value(vec![V1, V2], &[]);

    }

    #[test]
    fn compounds() {

        test_value(NbtCompound::new(),  &[NBT_COMPOUND as u8, 0, 0, 0]);

        let mut comp = NbtCompound::new();
        comp.insert("key0", "hello");
        comp.insert("key1", true);
        comp.insert("key2", 3141592.5f32);

        test_value(comp, &[
            NBT_COMPOUND as u8, 0, 0, // Compound header
            NBT_STRING as u8,   0, 4, 0x6B, 0x65, 0x79, 0x30, 0, 5, 0x68, 0x65, 0x6C, 0x6C, 0x6F, // key0 header + value
            NBT_BYTE as u8,     0, 4, 0x6B, 0x65, 0x79, 0x31, 0x01, // key1 header + value
            NBT_FLOAT as u8,    0, 4, 0x6B, 0x65, 0x79, 0x32, 0x4A, 0x3F, 0xBF, 0x62, // key2 header + value
            0 // terminating byte
        ]);

    }
    
}
