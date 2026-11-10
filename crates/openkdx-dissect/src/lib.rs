#![forbid(unsafe_code)]
//! Decode KDX connections in pcap and pcapng captures.
//!
//! [`connection::dissect_capture`] turns a capture into decoded connections
//! and tracker registrations; [`audit::audit`] checks that each payload
//! re-encodes byte for byte; [`text`] renders records for people.

pub mod audit;
pub mod capture;
pub mod connection;
pub mod text;
