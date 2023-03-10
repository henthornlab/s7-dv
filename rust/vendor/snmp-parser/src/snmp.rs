//! SNMP Parser (v1 and v2c)
//!
//! SNMP is defined in the following RFCs:
//!   - [RFC1157](https://tools.ietf.org/html/rfc1157): SNMP v1
//!   - [RFC1442](https://tools.ietf.org/html/rfc1442): Structure of Management Information for version 2 of the Simple Network Management Protocol (SNMPv2)
//!   - [RFC1902](https://tools.ietf.org/html/rfc1902): SNMP v2 SMI
//!   - [RFC2578](https://tools.ietf.org/html/rfc2578): Structure of Management Information Version 2 (SMIv2)
//!   - [RFC3416](https://tools.ietf.org/html/rfc3416): SNMP v2
//!   - [RFC2570](https://tools.ietf.org/html/rfc2570): Introduction to SNMP v3

use crate::error::SnmpError;
use der_parser::ber::*;
use der_parser::error::*;
use der_parser::oid::Oid;
use nom::bytes::streaming::take;
use nom::combinator::{map, map_res};
use nom::{Err, IResult};
use std::net::Ipv4Addr;
use std::slice::Iter;
use std::{fmt, str};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct PduType(pub u32);

#[allow(non_upper_case_globals)]
impl PduType {
    pub const GetRequest: PduType = PduType(0);
    pub const GetNextRequest: PduType = PduType(1);
    pub const Response: PduType = PduType(2);
    pub const SetRequest: PduType = PduType(3);
    pub const TrapV1: PduType = PduType(4); // Obsolete, was the old Trap-PDU in SNMPv1
    pub const GetBulkRequest: PduType = PduType(5);
    pub const InformRequest: PduType = PduType(6);
    pub const TrapV2: PduType = PduType(7);
    pub const Report: PduType = PduType(8);
}

impl fmt::Debug for PduType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            0 => f.write_str("GetRequest"),
            1 => f.write_str("GetNextRequest"),
            2 => f.write_str("Response"),
            3 => f.write_str("SetRequest"),
            4 => f.write_str("TrapV1"),
            5 => f.write_str("GetBulkRequest"),
            6 => f.write_str("InformRequest"),
            7 => f.write_str("TrapV2"),
            8 => f.write_str("Report"),
            n => f.debug_tuple("PduType").field(&n).finish(),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct TrapType(pub u8);

impl TrapType {
    pub const COLD_START: TrapType = TrapType(0);
    pub const WARM_START: TrapType = TrapType(1);
    pub const LINK_DOWN: TrapType = TrapType(2);
    pub const LINK_UP: TrapType = TrapType(3);
    pub const AUTHENTICATION_FAILURE: TrapType = TrapType(4);
    pub const EGP_NEIGHBOR_LOSS: TrapType = TrapType(5);
    pub const ENTERPRISE_SPECIFIC: TrapType = TrapType(6);
}

impl fmt::Debug for TrapType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            0 => f.write_str("coldStart"),
            1 => f.write_str("warmStart"),
            2 => f.write_str("linkDown"),
            3 => f.write_str("linkUp"),
            4 => f.write_str("authenticationFailure"),
            5 => f.write_str("egpNeighborLoss"),
            6 => f.write_str("enterpriseSpecific"),
            n => f.debug_tuple("TrapType").field(&n).finish(),
        }
    }
}

/// This CHOICE represents an address from one of possibly several
/// protocol families.  Currently, only one protocol family, the Internet
/// family, is present in this CHOICE.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum NetworkAddress {
    IPv4(Ipv4Addr),
}

/// This application-wide type represents a non-negative integer which
/// monotonically increases until it reaches a maximum value, when it
/// wraps around and starts increasing again from zero.  This memo
/// specifies a maximum value of 2^32-1 (4294967295 decimal) for
/// counters.
pub type Counter = u32;

/// This application-wide type represents a non-negative integer, which
/// may increase or decrease, but which latches at a maximum value.  This
/// memo specifies a maximum value of 2^32-1 (4294967295 decimal) for
/// gauges.
pub type Gauge = u32;

/// This application-wide type represents a non-negative integer which
/// counts the time in hundredths of a second since some epoch.  When
/// object types are defined in the MIB which use this ASN.1 type, the
/// description of the object type identifies the reference epoch.
pub type TimeTicks = u32;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ErrorStatus(pub u32);

#[allow(non_upper_case_globals)]
impl ErrorStatus {
    pub const NoError: ErrorStatus = ErrorStatus(0);
    pub const TooBig: ErrorStatus = ErrorStatus(1);
    pub const NoSuchName: ErrorStatus = ErrorStatus(2);
    pub const BadValue: ErrorStatus = ErrorStatus(3);
    pub const ReadOnly: ErrorStatus = ErrorStatus(4);
    pub const GenErr: ErrorStatus = ErrorStatus(5);
}

impl fmt::Debug for ErrorStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            0 => f.write_str("NoError"),
            1 => f.write_str("TooBig"),
            2 => f.write_str("NoSuchName"),
            3 => f.write_str("BadValue"),
            4 => f.write_str("ReadOnly"),
            5 => f.write_str("GenErr"),
            n => f.debug_tuple("ErrorStatus").field(&n).finish(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct SnmpGenericPdu<'a> {
    pub pdu_type: PduType,
    pub req_id: u32,
    pub err: ErrorStatus,
    pub err_index: u32,
    pub var: Vec<SnmpVariable<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct SnmpBulkPdu<'a> {
    pub req_id: u32,
    pub non_repeaters: u32,
    pub max_repetitions: u32,
    pub var: Vec<SnmpVariable<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct SnmpTrapPdu<'a> {
    pub enterprise: Oid<'a>,
    pub agent_addr: NetworkAddress,
    pub generic_trap: TrapType,
    pub specific_trap: u32,
    pub timestamp: TimeTicks,
    pub var: Vec<SnmpVariable<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum SnmpPdu<'a> {
    Generic(SnmpGenericPdu<'a>),
    Bulk(SnmpBulkPdu<'a>),
    TrapV1(SnmpTrapPdu<'a>),
}

/// An SNMPv1 or SNMPv2c message
#[derive(Debug, PartialEq)]
pub struct SnmpMessage<'a> {
    /// Version, as raw-encoded: 0 for SNMPv1, 1 for SNMPv2c
    pub version: u32,
    pub community: String,
    pub pdu: SnmpPdu<'a>,
}

impl<'a> SnmpGenericPdu<'a> {
    pub fn vars_iter(&'a self) -> Iter<SnmpVariable> {
        self.var.iter()
    }
}

impl<'a> SnmpTrapPdu<'a> {
    pub fn vars_iter(&'a self) -> Iter<SnmpVariable> {
        self.var.iter()
    }
}

impl<'a> SnmpPdu<'a> {
    pub fn pdu_type(&self) -> PduType {
        match *self {
            SnmpPdu::Generic(ref pdu) => pdu.pdu_type,
            SnmpPdu::Bulk(_) => PduType::GetBulkRequest,
            SnmpPdu::TrapV1(_) => PduType::TrapV1,
        }
    }

    pub fn vars_iter(&'a self) -> Iter<SnmpVariable> {
        match *self {
            SnmpPdu::Generic(ref pdu) => pdu.var.iter(),
            SnmpPdu::Bulk(ref pdu) => pdu.var.iter(),
            SnmpPdu::TrapV1(ref pdu) => pdu.var.iter(),
        }
    }
}

impl<'a> SnmpMessage<'a> {
    pub fn pdu_type(&self) -> PduType {
        self.pdu.pdu_type()
    }

    pub fn vars_iter(&'a self) -> Iter<SnmpVariable> {
        self.pdu.vars_iter()
    }
}

#[derive(Debug, PartialEq)]
pub struct SnmpVariable<'a> {
    pub oid: Oid<'a>,
    pub val: ObjectSyntax<'a>,
}

#[derive(Debug, PartialEq)]
pub enum ObjectSyntax<'a> {
    Number(BerObject<'a>),
    String(&'a [u8]),
    Object(Oid<'a>),
    BitString(u8, BitStringObject<'a>),
    Empty,
    UnknownSimple(BerObject<'a>),
    IpAddress(NetworkAddress),
    Counter32(Counter),
    Gauge32(Gauge),
    TimeTicks(TimeTicks),
    Opaque(&'a [u8]),
    NsapAddress(&'a [u8]),
    Counter64(u64),
    UInteger32(u32),
    UnknownApplication(u32, &'a [u8]),
}

pub(crate) fn parse_ber_octetstring_as_slice(i: &[u8]) -> IResult<&[u8], &[u8], BerError> {
    match parse_ber_octetstring(i) {
        Ok((rem, ref obj)) => match obj.content {
            BerObjectContent::OctetString(s) => Ok((rem, s)),
            _ => Err(Err::Error(BerError::InvalidTag)),
        },
        Err(e) => Err(e),
    }
}

// from RFC2578
// ObjectSyntax ::=
//     CHOICE {
//         simple
//             SimpleSyntax,
//
//           -- note that SEQUENCEs for conceptual tables and
//           -- rows are not mentioned here...
//
//         application-wide
//             ApplicationSyntax
//     }
//
// Defined in RFC1442 and RFC2578
fn parse_objectsyntax<'a>(i: &'a [u8]) -> IResult<&'a [u8], ObjectSyntax, BerError> {
    let (rem, hdr) = ber_read_element_header(i)?;
    if hdr.is_application() {
        match hdr.tag.0 {
            0 => {
                // IpAddress ::=
                //     [APPLICATION 0]
                //         IMPLICIT OCTET STRING (SIZE (4))
                let (rem, content) = ber_read_element_content_as(
                    rem,
                    BerTag::OctetString,
                    hdr.len,
                    hdr.is_constructed(),
                    0,
                )?;
                match content {
                    BerObjectContent::OctetString(s) if s.len() == 4 => {
                        let ipv4 = NetworkAddress::IPv4(Ipv4Addr::new(s[0], s[1], s[2], s[3]));
                        Ok((rem, ObjectSyntax::IpAddress(ipv4)))
                    }
                    _ => Err(Err::Error(BerError::InvalidTag)),
                }
            }
            1..=3 => {
                // -- this wraps
                // Counter32 ::=
                //     [APPLICATION 1]
                //         IMPLICIT INTEGER (0..4294967295)
                //
                // -- this doesn't wrap
                // Gauge32 ::=
                //     [APPLICATION 2]
                //         IMPLICIT INTEGER (0..4294967295)
                //
                // -- an unsigned 32-bit quantity
                // -- indistinguishable from Gauge32
                // Unsigned32 ::=
                //     [APPLICATION 2]
                //         IMPLICIT INTEGER (0..4294967295)
                //
                // -- hundredths of seconds since an epoch
                // TimeTicks ::=
                //     [APPLICATION 3]
                //         IMPLICIT INTEGER (0..4294967295)
                let (rem, content) = ber_read_element_content_as(
                    rem,
                    BerTag::Integer,
                    hdr.len,
                    hdr.is_constructed(),
                    0,
                )?;
                let x = content.as_u32()?;
                let obj = match hdr.tag.0 {
                    1 => ObjectSyntax::Counter32(x),
                    2 => ObjectSyntax::Gauge32(x),
                    3 => ObjectSyntax::TimeTicks(x),
                    _ => unreachable!(),
                };
                Ok((rem, obj))
            }
            4 => {
                let len = hdr.len.primitive()?;
                map(take(len), ObjectSyntax::Opaque)(rem)
            }
            5 => {
                let len = hdr.len.primitive()?;
                map(take(len), ObjectSyntax::NsapAddress)(rem)
            }
            6 => {
                let (rem, content) = ber_read_element_content_as(
                    rem,
                    BerTag::Integer,
                    hdr.len,
                    hdr.is_constructed(),
                    0,
                )?;
                let value = content.as_u64()?;
                Ok((rem, ObjectSyntax::Counter64(value)))
            }
            7 => {
                let (rem, content) = ber_read_element_content_as(
                    rem,
                    BerTag::Integer,
                    hdr.len,
                    hdr.is_constructed(),
                    0,
                )?;
                let value = content.as_u32()?;
                Ok((rem, ObjectSyntax::UInteger32(value)))
            }
            _ => {
                let len = hdr.len.primitive()?;
                map(take(len), |x| {
                    ObjectSyntax::UnknownApplication(hdr.tag.0, x)
                })(rem)
            }
        }
    } else {
        // SimpleSyntax ::=
        //     CHOICE {
        //         -- INTEGERs with a more restrictive range
        //         -- may also be used
        //         integer-value               -- includes Integer32
        //             INTEGER (-2147483648..2147483647),
        //
        //         -- OCTET STRINGs with a more restrictive size
        //         -- may also be used
        //         string-value
        //             OCTET STRING (SIZE (0..65535)),
        //
        //         objectID-value
        //             OBJECT IDENTIFIER
        //     }
        if hdr.len.is_null() {
            return Ok((rem, ObjectSyntax::Empty));
        }
        let (rem, content) =
            ber_read_element_content_as(rem, hdr.tag, hdr.len, hdr.is_constructed(), 0)?;
        let obj = match content {
            BerObjectContent::Integer(_) => ObjectSyntax::Number(BerObject::from_obj(content)),
            BerObjectContent::OctetString(s) => ObjectSyntax::String(s),
            BerObjectContent::OID(o) => ObjectSyntax::Object(o),
            BerObjectContent::BitString(a, s) => ObjectSyntax::BitString(a, s),
            BerObjectContent::Null => ObjectSyntax::Empty,
            _ => ObjectSyntax::UnknownSimple(BerObject::from_obj(content)),
        };
        Ok((rem, obj))
    }
}

fn parse_varbind<'a>(i: &'a [u8]) -> IResult<&'a [u8], SnmpVariable, BerError> {
    parse_ber_sequence_defined_g(|i, _| {
        let (i, oid) = map_res(parse_ber_oid, |x| x.as_oid_val())(i)?;
        let (i, val) = parse_objectsyntax(i)?;
        Ok((i, SnmpVariable { oid, val }))
    })(i)
}

fn parse_varbind_list(i: &[u8]) -> IResult<&[u8], Vec<SnmpVariable>, BerError> {
    parse_ber_sequence_of_v(parse_varbind)(i)
}

/// <pre>
///  NetworkAddress ::=
///      CHOICE {
///          internet
///              IpAddress
///      }
/// IpAddress ::=
///     [APPLICATION 0]          -- in network-byte order
///         IMPLICIT OCTET STRING (SIZE (4))
/// </pre>
fn parse_networkaddress(i: &[u8]) -> IResult<&[u8], NetworkAddress, BerError> {
    match parse_ber(i) {
        Ok((rem, obj)) => {
            if obj.header.tag != BerTag::EndOfContent || obj.header.class != BerClass::Application {
                return Err(Err::Error(BerError::InvalidTag));
            }
            match obj.content {
                BerObjectContent::Unknown(_, _, s) if s.len() == 4 => Ok((
                    rem,
                    NetworkAddress::IPv4(Ipv4Addr::new(s[0], s[1], s[2], s[3])),
                )),
                _ => Err(Err::Error(BerError::InvalidTag)),
            }
        }
        Err(e) => Err(e),
    }
}

/// <pre>
/// TimeTicks ::=
///     [APPLICATION 3]
///         IMPLICIT INTEGER (0..4294967295)
/// </pre>
fn parse_timeticks(i: &[u8]) -> IResult<&[u8], TimeTicks, BerError> {
    parse_ber_tagged_implicit_g(BerTag(3), |i, hdr, depth| {
        if !hdr.is_application() {
            return Err(Err::Error(BerError::InvalidTag));
        }
        let (i, content) =
            ber_read_element_content_as(i, BerTag::Integer, hdr.len, hdr.is_constructed(), depth)?;
        let value = content.as_u32()?;
        Ok((i, value))
    })(i)
}

fn parse_snmp_v1_generic_pdu(pdu: &[u8], tag: PduType) -> IResult<&[u8], SnmpPdu, BerError> {
    let (i, req_id) = parse_ber_u32(pdu)?;
    let (i, err) = map(parse_ber_u32, ErrorStatus)(i)?;
    let (i, err_index) = parse_ber_u32(i)?;
    let (i, var) = parse_varbind_list(i)?;
    let pdu = SnmpPdu::Generic(SnmpGenericPdu {
        pdu_type: tag,
        req_id,
        err,
        err_index,
        var,
    });
    Ok((i, pdu))
}

fn parse_snmp_v1_bulk_pdu(i: &[u8]) -> IResult<&[u8], SnmpPdu, BerError> {
    let (i, req_id) = parse_ber_u32(i)?;
    let (i, non_repeaters) = parse_ber_u32(i)?;
    let (i, max_repetitions) = parse_ber_u32(i)?;
    let (i, var) = parse_varbind_list(i)?;
    let pdu = SnmpBulkPdu {
        req_id,
        non_repeaters,
        max_repetitions,
        var,
    };
    Ok((i, SnmpPdu::Bulk(pdu)))
}

fn parse_snmp_v1_trap_pdu<'a>(i: &'a [u8]) -> IResult<&'a [u8], SnmpPdu, BerError> {
    let (i, enterprise) = map_res(parse_ber_oid, |x| x.as_oid_val())(i)?;
    let (i, agent_addr) = parse_networkaddress(i)?;
    let (i, generic_trap) = parse_ber_u32(i)?;
    let (i, specific_trap) = parse_ber_u32(i)?;
    let (i, timestamp) = parse_timeticks(i)?;
    let (i, var) = parse_varbind_list(i)?;
    let pdu = SnmpTrapPdu {
        enterprise,
        agent_addr,
        generic_trap: TrapType(generic_trap as u8),
        specific_trap,
        timestamp,
        var,
    };
    Ok((i, SnmpPdu::TrapV1(pdu)))
}

/// Parse a SNMP v1 message.
///
/// Top-level message
///
/// <pre>
/// Message ::=
///         SEQUENCE {
///             version          -- version-1 for this RFC
///                 INTEGER {
///                     version-1(0)
///                 },
///
///             community        -- community name
///                 OCTET STRING,
///
///             data             -- e.g., PDUs if trivial
///                 ANY          -- authentication is being used
///         }
/// </pre>
///
/// Example:
///
/// ```rust
/// use snmp_parser::parse_snmp_v1;
///
/// static SNMPV1_REQ: &[u8] = include_bytes!("../assets/snmpv1_req.bin");
///
/// # fn main() {
/// match parse_snmp_v1(&SNMPV1_REQ) {
///   Ok((_, ref r)) => {
///     assert!(r.version == 0);
///     assert!(r.community == String::from("public"));
///     assert!(r.vars_iter().count() == 1);
///   },
///   Err(e) => panic!("{}", e),
/// }
/// # }
/// ```
pub fn parse_snmp_v1(i: &[u8]) -> IResult<&[u8], SnmpMessage, SnmpError> {
    parse_ber_sequence_defined_g(|i, _| {
        let (i, version) = parse_ber_u32(i)?;
        if version != 0 {
            return Err(Err::Error(BerError::BerValueError));
        }
        let (i, community) = map_res(parse_ber_octetstring_as_slice, str::from_utf8)(i)?;
        let (i, pdu) = parse_snmp_v1_pdu(i)?;
        let msg = SnmpMessage {
            version,
            community: community.to_string(),
            pdu,
        };
        Ok((i, msg))
    })(i)
    .map_err(Err::convert)
}

pub(crate) fn parse_snmp_v1_pdu(i: &[u8]) -> IResult<&[u8], SnmpPdu, BerError> {
    match ber_read_element_header(i) {
        Ok((rem,hdr)) => {
            match PduType(hdr.tag.0) {
                PduType::GetRequest |
                PduType::GetNextRequest |
                PduType::Response |
                PduType::SetRequest     => parse_snmp_v1_generic_pdu(rem, PduType(hdr.tag.0)),
                PduType::TrapV1         => parse_snmp_v1_trap_pdu(rem),
                _                       => Err(Err::Error(BerError::BerValueError)),
                // _                       => { return IResult::Error(error_code!(ErrorKind::Custom(SnmpError::InvalidPdu))); },
            }
        },
        Err(e)        => Err(e)
        // IResult::Incomplete(i) => IResult::Incomplete(i),
        // IResult::Error(_)      => IResult::Error(error_code!(ErrorKind::Custom(129))),
        // // IResult::Error(_)      => IResult::Error(error_code!(ErrorKind::Custom(SnmpError::InvalidScopedPduData))),
    }
}

/// Parse a SNMP v2c message.
///
/// Top-level message
///
/// <pre>
/// Message ::=
///         SEQUENCE {
///             version
///                 INTEGER {
///                     version(1)  -- modified from RFC 1157
///                 },
///
///             community           -- community name
///                 OCTET STRING,
///
///             data                -- PDUs as defined in [4]
///                 ANY
///         }
/// </pre>
pub fn parse_snmp_v2c(i: &[u8]) -> IResult<&[u8], SnmpMessage, SnmpError> {
    parse_ber_sequence_defined_g(|i, _| {
        let (i, version) = parse_ber_u32(i)?;
        if version != 1 {
            return Err(Err::Error(BerError::BerValueError));
        }
        let (i, community) = map_res(parse_ber_octetstring_as_slice, str::from_utf8)(i)?;
        let (i, pdu) = parse_snmp_v2c_pdu(i)?;
        let msg = SnmpMessage {
            version,
            community: community.to_string(),
            pdu,
        };
        Ok((i, msg))
    })(i)
    .map_err(Err::convert)
}

pub(crate) fn parse_snmp_v2c_pdu(i: &[u8]) -> IResult<&[u8], SnmpPdu, BerError> {
    match ber_read_element_header(i) {
        Ok((rem,hdr)) => {
            match PduType(hdr.tag.0) {
                PduType::GetRequest |
                PduType::GetNextRequest |
                PduType::Response |
                PduType::SetRequest |
                PduType::InformRequest |
                PduType::TrapV2 |
                PduType::Report         => parse_snmp_v1_generic_pdu(rem, PduType(hdr.tag.0)),
                PduType::GetBulkRequest => parse_snmp_v1_bulk_pdu(rem),
                PduType::TrapV1         => parse_snmp_v1_trap_pdu(rem),
                _                       => Err(Err::Error(BerError::BerValueError)),
                // _                       => { return IResult::Error(error_code!(ErrorKind::Custom(SnmpError::InvalidPdu))); },
            }
        },
        Err(e)        => Err(e)
        // IResult::Incomplete(i) => IResult::Incomplete(i),
        // IResult::Error(_)      => IResult::Error(error_code!(ErrorKind::Custom(129))),
        // // IResult::Error(_)      => IResult::Error(error_code!(ErrorKind::Custom(SnmpError::InvalidScopedPduData))),
    }
}
