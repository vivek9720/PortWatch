use crate::error::{ParseError, ParseResult};

#[derive(Clone, Copy, Debug)]
pub struct ByteCursor<'a> {
    data: &'a [u8],
    pos: usize,
    base: usize,
}

impl<'a> ByteCursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            base: 0,
        }
    }

    pub fn with_base(data: &'a [u8], base: usize) -> Self {
        Self { data, pos: 0, base }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn absolute_position(&self) -> usize {
        self.base.saturating_add(self.pos)
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn as_slice(&self) -> &'a [u8] {
        self.data
    }

    pub fn tail(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    pub fn peek_u8(&self) -> ParseResult<u8> {
        self.data
            .get(self.pos)
            .copied()
            .ok_or_else(|| ParseError::eof(self.absolute_position(), "u8"))
    }

    pub fn read_u8(&mut self) -> ParseResult<u8> {
        let value = self.peek_u8()?;
        self.pos += 1;
        Ok(value)
    }

    pub fn read_i8(&mut self) -> ParseResult<i8> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_bool(&mut self) -> ParseResult<bool> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ParseError::invalid_value(
                self.absolute_position().saturating_sub(1),
                "bool",
            )),
        }
    }

    pub fn read_u16(&mut self) -> ParseResult<u16> {
        let bytes = self.read_array::<2>("u16")?;
        Ok(u16::from_le_bytes(bytes))
    }

    pub fn read_be_u16(&mut self) -> ParseResult<u16> {
        let bytes = self.read_array::<2>("be_u16")?;
        Ok(u16::from_be_bytes(bytes))
    }

    pub fn read_i16(&mut self) -> ParseResult<i16> {
        let bytes = self.read_array::<2>("i16")?;
        Ok(i16::from_le_bytes(bytes))
    }

    pub fn read_u24(&mut self) -> ParseResult<u32> {
        let b0 = self.read_u8()? as u32;
        let b1 = self.read_u8()? as u32;
        let b2 = self.read_u8()? as u32;
        Ok(b0 | (b1 << 8) | (b2 << 16))
    }

    pub fn read_u32(&mut self) -> ParseResult<u32> {
        let bytes = self.read_array::<4>("u32")?;
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn read_be_u32(&mut self) -> ParseResult<u32> {
        let bytes = self.read_array::<4>("be_u32")?;
        Ok(u32::from_be_bytes(bytes))
    }

    pub fn read_i32(&mut self) -> ParseResult<i32> {
        let bytes = self.read_array::<4>("i32")?;
        Ok(i32::from_le_bytes(bytes))
    }

    pub fn read_u64(&mut self) -> ParseResult<u64> {
        let bytes = self.read_array::<8>("u64")?;
        Ok(u64::from_le_bytes(bytes))
    }

    pub fn read_i64(&mut self) -> ParseResult<i64> {
        let bytes = self.read_array::<8>("i64")?;
        Ok(i64::from_le_bytes(bytes))
    }

    pub fn read_f32_scaled(&mut self, scale: f32) -> ParseResult<f32> {
        Ok(self.read_i32()? as f32 / scale)
    }

    pub fn read_slice(&mut self, len: usize, context: &'static str) -> ParseResult<&'a [u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| ParseError::invalid_length(self.absolute_position(), context))?;
        if end > self.data.len() {
            return Err(ParseError::eof(self.absolute_position(), context));
        }
        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    pub fn read_array<const N: usize>(&mut self, context: &'static str) -> ParseResult<[u8; N]> {
        let slice = self.read_slice(N, context)?;
        let mut out = [0u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    pub fn read_length_prefixed(&mut self, context: &'static str) -> ParseResult<&'a [u8]> {
        let len = self.read_u16()? as usize;
        self.read_slice(len, context)
    }

    pub fn read_varint(&mut self, context: &'static str) -> ParseResult<u64> {
        let mut result = 0u64;
        let mut shift = 0u32;
        for _ in 0..10 {
            let byte = self.read_u8()?;
            result |= ((byte & 0x7f) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
        }
        Err(ParseError::invalid_length(self.absolute_position(), context))
    }

    pub fn read_zigzag_i64(&mut self, context: &'static str) -> ParseResult<i64> {
        let value = self.read_varint(context)?;
        Ok(((value >> 1) as i64) ^ (-((value & 1) as i64)))
    }

    pub fn read_utf8(&mut self, context: &'static str) -> ParseResult<&'a str> {
        let start = self.absolute_position();
        let bytes = self.read_length_prefixed(context)?;
        std::str::from_utf8(bytes).map_err(|_| ParseError::invalid_utf8(start, context))
    }

    pub fn skip(&mut self, len: usize, context: &'static str) -> ParseResult<()> {
        self.read_slice(len, context)?;
        Ok(())
    }

    pub fn subcursor(&mut self, len: usize, context: &'static str) -> ParseResult<ByteCursor<'a>> {
        let start = self.absolute_position();
        let slice = self.read_slice(len, context)?;
        Ok(ByteCursor::with_base(slice, start))
    }

    pub fn take_remaining(&mut self) -> &'a [u8] {
        let out = &self.data[self.pos..];
        self.pos = self.data.len();
        out
    }

    pub fn expect_magic(&mut self, magic: &[u8], context: &'static str) -> ParseResult<()> {
        let start = self.absolute_position();
        let bytes = self.read_slice(magic.len(), context)?;
        if bytes == magic {
            Ok(())
        } else {
            Err(ParseError::invalid_magic(start, context))
        }
    }

    pub fn seek(&mut self, pos: usize, context: &'static str) -> ParseResult<()> {
        if pos <= self.data.len() {
            self.pos = pos;
            Ok(())
        } else {
            Err(ParseError::invalid_length(self.base.saturating_add(pos), context))
        }
    }

    pub fn align(&mut self, modulus: usize, context: &'static str) -> ParseResult<()> {
        if modulus == 0 {
            return Err(ParseError::invalid_value(self.absolute_position(), context));
        }
        let rem = self.pos % modulus;
        if rem == 0 {
            return Ok(());
        }
        self.skip(modulus - rem, context)
    }

    pub fn fork(&self) -> Self {
        *self
    }
}
