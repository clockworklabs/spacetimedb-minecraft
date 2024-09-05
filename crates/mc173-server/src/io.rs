//! This module provides read and write extension traits for Java types.

use byteorder::{ReadBytesExt, WriteBytesExt, BE};
use std::io::{self, Read, Write};


/// Extension trait with Minecraft-specific packet read methods.
pub trait ReadJavaExt: Read {

    #[inline]
    fn read_java_byte(&mut self) -> io::Result<i8> {
        ReadBytesExt::read_i8(self)
    }

    #[inline]
    fn read_java_short(&mut self) -> io::Result<i16> {
        ReadBytesExt::read_i16::<BE>(self)
    }

    #[inline]
    fn read_java_int(&mut self) -> io::Result<i32> {
        ReadBytesExt::read_i32::<BE>(self)
    }

    #[inline]
    fn read_java_long(&mut self) -> io::Result<i64> {
        ReadBytesExt::read_i64::<BE>(self)
    }

    #[inline]
    fn read_java_float(&mut self) -> io::Result<f32> {
        ReadBytesExt::read_f32::<BE>(self)
    }

    #[inline]
    fn read_java_double(&mut self) -> io::Result<f64> {
        ReadBytesExt::read_f64::<BE>(self)
    }

    #[inline]
    fn read_java_boolean(&mut self) -> io::Result<bool> {
        Ok(self.read_java_byte()? != 0)
    }

    fn read_java_string16(&mut self, max_len: usize) -> io::Result<String> {
        
        let len = self.read_java_short()?;
        if len < 0 {
            return Err(new_invalid_data_err("negative length string"));
        }

        if len as usize > max_len {
            return Err(new_invalid_data_err("exceeded max string length"));
        }

        let mut raw = Vec::with_capacity(len as usize);
        for _ in 0..len {
            raw.push(ReadBytesExt::read_u16::<BE>(self)?);
        }

        let ret = char::decode_utf16(raw)
            .map(|res| res.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect::<String>();

        Ok(ret)

    }

    fn read_java_string8(&mut self) -> io::Result<String> {

        let len = self.read_u16::<BE>()?;
        let mut buf = vec![0u8; len as usize];
        self.read_exact(&mut buf)?;

        String::from_utf8(buf).map_err(|_| new_invalid_data_err("invalid utf-8 string"))

    }

}

/// Extension trait with Minecraft-specific packet write methods.
pub trait WriteJavaExt: Write {

    #[inline]
    fn write_java_byte(&mut self, b: i8) -> io::Result<()> {
        WriteBytesExt::write_i8(self, b)
    }

    #[inline]
    fn write_java_short(&mut self, s: i16) -> io::Result<()> {
        WriteBytesExt::write_i16::<BE>(self, s)
    }

    #[inline]
    fn write_java_int(&mut self, i: i32) -> io::Result<()> {
        WriteBytesExt::write_i32::<BE>(self, i)
    }

    #[inline]
    fn write_java_long(&mut self, l: i64) -> io::Result<()> {
        WriteBytesExt::write_i64::<BE>(self, l)
    }

    #[inline]
    fn write_java_float(&mut self, f: f32) -> io::Result<()> {
        WriteBytesExt::write_f32::<BE>(self, f)
    }

    #[inline]
    fn write_java_double(&mut self, d: f64) -> io::Result<()> {
        WriteBytesExt::write_f64::<BE>(self, d)
    }

    #[inline]
    fn write_java_boolean(&mut self, b: bool) -> io::Result<()> {
        self.write_java_byte(b as i8)
    }

    fn write_java_string16(&mut self, s: &str) -> io::Result<()> {
        
        // Count the number of UTF-16 java character.
        let len = s.chars().map(|c| c.len_utf16()).sum::<usize>();
        if len > i16::MAX as usize {
            return Err(new_invalid_data_err("string too big"));
        }
        
        self.write_java_short(len as i16)?;
        for code in s.encode_utf16() {
            WriteBytesExt::write_u16::<BE>(self, code)?;
        }

        Ok(())

    }

    fn write_java_string8(&mut self, s: &str) -> io::Result<()> {

        if s.len() > u16::MAX as usize {
            return Err(new_invalid_data_err("string too big"));
        }

        self.write_u16::<BE>(s.len() as u16)?;
        self.write_all(s.as_bytes())
        
    }

}

impl<R: Read> ReadJavaExt for R {}
impl<W: Write> WriteJavaExt for W {}


/// Return an invalid data io error with specific message.
fn new_invalid_data_err(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
