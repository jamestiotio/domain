//! Basic resource data handling.
//!
//! DNS resource records consist of some common start defining the domain
//! name they pertain to, their type and class, and finally record data
//! the format of which depends on the specific record type. As there are
//! currently more than eighty record types, having a giant enum for record
//! data seemed like a bad idea. Instead, resource records are generic over
//! the `RecordData` trait which is being implemented by all concrete
//! record data types—these are defined in module `domain::rdata`.
//!
//! In order to walk over all resource records in a message or work with
//! unknown record types, this module also defines the `GenericRecordData`
//! type that can deal with all record types.

use std::fmt;
use std::io;
use iana::RRType;
use super::compose::ComposeBytes;
use super::error::{ComposeResult, ParseResult};
use super::nest::Nest;
use super::parse::ParseBytes;
use ::bits::bytes::BytesBuf;
use ::master;


//----------- RecordData ------------------------------------------------

/// A trait for parsing and composing record data.
pub trait RecordData<'a>: Sized {
    /// Returns the record type for this record data instance.
    fn rtype(&self) -> RRType;

    /// Appends the record data to the end of compose target.
    fn compose<C: ComposeBytes>(&self, target: &mut C) -> ComposeResult<()>;

    /// Parse the record data from a cursor if the type is right.
    ///
    /// If this record data type does not feel responsible for records of
    /// type `rtype`, it should return `None` an leave the parser untouche.
    /// Otherwise it should return a value or an error if parsing fails.
    fn parse<P>(rtype: RRType, parser: &mut P) -> Option<ParseResult<Self>>
             where P: ParseBytes<'a>;
}


//------------ GenericRecordData --------------------------------------------

/// A type for any type of record data.
///
/// This type accepts any record type and stores the plain binary data in
/// form of a `Nest`. This way, it can provide a parser for converting the
/// data into into concrete record data type if necessary.
///
/// Since values may be built from messages, the data may contain compressed
/// domain names. When composing a new message, this may lead to corrupt
/// messages when simply pushing the data as is. However, the type follows
/// RFC 3597, ‘Handling of Unknown DNS Resource Record (RR) Types,’ and
/// assumes that compressed domain names only occur in record types defined
/// in RFC 1035. When composing, it treats those values specially ensuring
/// that compressed names are handled correctly. This may still lead to
/// corrupt messages, however, if the generic record data is obtained from
/// a source not complying with RFC 3597. In general, be wary when
/// re-composing parsed messages unseen.
#[derive(Clone, Debug)]
pub struct GenericRecordData<'a> {
    rtype: RRType,
    data: Nest<'a>,
}

impl<'a> GenericRecordData<'a> {
    /// Creates a generic record data value from its components.
    pub fn new(rtype: RRType, data: Nest<'a>) -> Self {
        GenericRecordData { rtype: rtype, data: data }
    }

    /// Returns the record type of the generic record data value.
    pub fn rtype(&self) -> RRType { self.rtype }

    /// Returns a reference to the value’s data.
    pub fn data(&self) -> &Nest { &self.data }

    /// Tries to re-parse the value for the concrete type `R`.
    ///
    /// Returns `None` if `R` does not want to parse the value.
    pub fn concrete<'b, R: RecordData<'b>>(&'b self) -> Option<ParseResult<R>> {
        let mut parser = self.data.parser();
        R::parse(self.rtype, &mut parser)
    }

    /// Scan generic master format record data into a bytes buf.
    ///
    /// This function *only* scans the generic record data format defined
    /// in [RFC 3597]. Use [domain::rdata::scan_into()] for a function that
    /// tries to also scan the specific record data format for record type
    /// `rtype`.
    ///
    /// [RFC 3597]: https:://tools.ietf.org/html/rfc3597
    /// [domain::rdata::scan_into()]: ../../rdata/fn.scan_into.html
    pub fn scan_into<R, B>(stream: &mut master::Stream<R>, target: &mut B)
                           -> master::Result<()>
                     where R: io::Read, B: BytesBuf {
        try!(stream.skip_literal(b"\\#"));
        let mut len = try!(stream.scan_u16());
        target.reserve(len as usize);
        while len > 0 {
            try!(stream.scan_hex_word(|v| {
                if len == 0 { Err(master::SyntaxError::LongGenericData) }
                else {
                    target.push_u8(v);
                    len -= 1;
                    Ok(())
                }
            }))
        }
        Ok(())
    }

    /// Formats the record data as if it were of concrete type `R`.
    pub fn fmt<'b: 'a, R>(&'b self, f: &mut fmt::Formatter) -> fmt::Result
               where R: RecordData<'a> + fmt::Display {
        let mut parser = self.data.parser();
        match R::parse(self.rtype, &mut parser) {
            Some(Ok(data)) => data.fmt(f),
            Some(Err(..)) => Ok(()),
            None => Ok(())
        }
    }
}


impl<'a> RecordData<'a> for GenericRecordData<'a> {
    fn rtype(&self) -> RRType {
        self.rtype
    }

    fn compose<C: ComposeBytes>(&self, target: &mut C) -> ComposeResult<()> {
        self.data.compose(target)
    }

    fn parse<P>(rtype: RRType, parser: &mut P) -> Option<ParseResult<Self>>
             where P: ParseBytes<'a> {
        let len = parser.left();
        match parser.parse_nest(len) {
            Err(err) => Some(Err(err)),
            Ok(nest) => Some(Ok(GenericRecordData::new(rtype, nest)))
        }
    }
}


impl<'a> fmt::Display for GenericRecordData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use rdata::*;

        match self.rtype {
            // RFC 1035
            RRType::A => self.fmt::<A>(f),
            RRType::Cname => self.fmt::<Cname>(f),
            RRType::Hinfo => self.fmt::<Hinfo>(f),
            RRType::Mb => self.fmt::<Mb>(f),
            RRType::Md => self.fmt::<Md>(f),
            RRType::Mf => self.fmt::<Mf>(f),
            RRType::Mg => self.fmt::<Mg>(f),
            RRType::Minfo => self.fmt::<Minfo>(f),
            RRType::Mr => self.fmt::<Mr>(f),
            RRType::Mx => self.fmt::<Mx>(f),
            RRType::Ns => self.fmt::<Ns>(f),
            RRType::Null => self.fmt::<Null>(f),
            RRType::Ptr => self.fmt::<Ptr>(f),
            RRType::Soa => self.fmt::<Soa>(f),
            RRType::Txt => self.fmt::<Txt>(f),
            RRType::Wks => self.fmt::<Wks>(f),

            // RFC 3596
            RRType::Aaaa => self.fmt::<Aaaa>(f),

            // Unknown
            _ => "...".fmt(f)
        }
    }
}


impl<'a> PartialEq for GenericRecordData<'a> {
    /// Compares two generic record data values for equality.
    ///
    /// Almost all record types can be compared bitwise. However, record
    /// types from RFC 1035 may employ name compression if they contain
    /// domain names. For these we need to actually check.
    fn eq(&self, other: &Self) -> bool {
        if self.rtype != other.rtype { false }
        else {
            use rdata::rfc1035::*;

            match self.rtype {
                RRType::Cname => rdata_eq::<Cname>(self, other),
                RRType::Mb => rdata_eq::<Mb>(self, other),
                RRType::Md => rdata_eq::<Md>(self, other),
                RRType::Mf => rdata_eq::<Mf>(self, other),
                RRType::Mg => rdata_eq::<Mg>(self, other),
                RRType::Minfo => rdata_eq::<Minfo>(self, other),
                RRType::Mr => rdata_eq::<Mr>(self, other),
                RRType::Mx => rdata_eq::<Mx>(self, other),
                RRType::Ns => rdata_eq::<Ns>(self, other),
                RRType::Ptr => rdata_eq::<Ptr>(self, other),
                RRType::Soa => rdata_eq::<Soa>(self, other),
                RRType::Txt => rdata_eq::<Txt>(self, other),
                _ => self.data.as_bytes() == other.data.as_bytes()
            }
        }
    }
}

/// Parse and then compare with concrete type.
fn rdata_eq<'a, D>(left: &'a GenericRecordData<'a>,
                   right: &'a GenericRecordData<'a>) -> bool
            where D: RecordData<'a> + PartialEq {
    D::parse(left.rtype, &mut left.data.parser())
        == D::parse(right.rtype, &mut right.data.parser())
}

