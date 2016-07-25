//! Dealing with bytes slices and vec.

use std::mem;
use ::bits::error::{ParseError, ParseResult};


//------------ BytesSlice ---------------------------------------------------

/// A trait extending a bytes slice for reading of DNS data.
///
pub trait BytesSlice {
    fn split_u8(&self) -> ParseResult<(u8, &Self)>;
    fn split_u16(&self) -> ParseResult<(u16, &Self)>;
    fn split_u32(&self) -> ParseResult<(u32, &Self)>;
    fn split_bytes(&self, at: usize) -> ParseResult<(&[u8], &Self)>;
    fn tail(&self, start: usize) -> ParseResult<&Self>;
    fn check_len(&self, len: usize) -> ParseResult<()>;
}

impl BytesSlice for [u8] {
    fn split_u8(&self) -> ParseResult<(u8, &[u8])> {
        self.split_first().map(|(l,r)| (*l, r)).ok_or(ParseError::UnexpectedEnd)
    }

    fn split_u16(&self) -> ParseResult<(u16, &[u8])> {
        try!(self.check_len(2));
        let (l, r) = self.split_at(2);
        let l: &[u8; 2] = unsafe { mem::transmute(l.as_ptr()) };
        let l = unsafe { mem::transmute(*l) };
        Ok((u16::from_be(l), r))
    }

    fn split_u32(&self) -> ParseResult<(u32, &[u8])> {
        try!(self.check_len(4));
        if self.len() < 4 { return Err(ParseError::UnexpectedEnd) }
        let (l, r) = self.split_at(4);
        let l: &[u8; 4] = unsafe { mem::transmute(l.as_ptr()) };
        let l = unsafe { mem::transmute(*l) };
        Ok((u32::from_be(l), r))
    }

    fn split_bytes(&self, at: usize) -> ParseResult<(&[u8], &[u8])> {
        try!(self.check_len(at));
        Ok(self.split_at(at))
    }

    fn tail(&self, start: usize) -> ParseResult<&[u8]> {
        try!(self.check_len(start));
        if self.len() < start { return Err(ParseError::UnexpectedEnd) }
        Ok(&self[start..])
    }

    fn check_len(&self, len: usize) -> ParseResult<()> {
        if len > self.len() { Err(ParseError::UnexpectedEnd) }
        else { Ok(()) }
    }
}


//------------ BytesBuf -----------------------------------------------------

/// A trait for something that can have raw DNS data pushed to its end.
pub trait BytesBuf {
    fn reserve(&mut self, additional: usize);

    fn push_bytes(&mut self, data: &[u8]);

    fn push_u8(&mut self, data: u8) {
        let bytes: [u8; 1] = unsafe { mem::transmute(data) };
        self.push_bytes(&bytes);
    }

    fn push_u16(&mut self, data: u16) {
        let data = data.to_be();
        let bytes: [u8; 2] = unsafe { mem::transmute(data) };
        self.push_bytes(&bytes);
    }

    fn push_u32(&mut self, data: u32) {
        let data = data.to_be();
        let bytes: [u8; 4] = unsafe { mem::transmute(data) };
        self.push_bytes(&bytes);
    }
}

impl BytesBuf for Vec<u8> {
    fn reserve(&mut self, additional: usize) { self.reserve(additional) }
    fn push_bytes(&mut self, data: &[u8]) {
        self.extend(data)
    }
}

