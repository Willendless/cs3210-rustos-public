use core::fmt;

use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub date: Date,
    pub time: Time,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    // FIXME: Fill me in.
    pub attributes: Attributes,
    pub created_timestamp: Timestamp,
    pub accessed_timestamp: Timestamp,
    pub modified_timestamp: Timestamp,
}

const DAY_MASK: u16 = 0x1F;
const DAY_OFF: u16 = 0;
const MONTH_MASK: u16 = 0x1E0;
const MONTH_OFF: u16 = 5;
const YEAR_MASK: u16 = 0xFE00;
const YEAR_OFF: u16 = 9;
const SECONDS_MASK: u16 = 0x1F;
const SECONDS_OFF: u16 = 0;
const MINUTES_MASK: u16 = 0x7E0;
const MINUTES_OFF: u16 = 5;
const HOURS_MASK: u16 = 0xF800;
const HOURS_OFF: u16 = 11;

// FIXME: Implement `traits::Timestamp` for `Timestamp`.
impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        ((self.date.0 & YEAR_MASK) >> YEAR_OFF) as usize + 1980
    }

    fn month(&self) -> u8 {
        ((self.date.0 & MONTH_MASK) >> MONTH_OFF) as u8
    }

    fn day(&self) -> u8 {
        ((self.date.0 & DAY_MASK) >> DAY_OFF) as u8
    }

    fn hour(&self) -> u8 {
        ((self.time.0 & HOURS_MASK) >> HOURS_OFF) as u8
    }

    fn minute(&self) -> u8 {
        ((self.time.0 & MINUTES_MASK) >> MINUTES_OFF) as u8
    }

    fn second(&self) -> u8 {
        (((self.time.0 & SECONDS_MASK) >> SECONDS_OFF) as u8) * 2
    }
}

const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN: u8 = 0x02;

// FIXME: Implement `traits::Metadata` for `Metadata`.
impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    fn read_only(&self) -> bool {
        (self.attributes.0 & ATTR_READ_ONLY) > 0
    }

    fn hidden(&self) -> bool {
        (self.attributes.0 & ATTR_HIDDEN) > 0
    }

    fn created(&self) -> Self::Timestamp {
        self.created_timestamp
    }

    fn accessed(&self) -> Self::Timestamp {
        self.accessed_timestamp
    }

    fn modified(&self) -> Self::Timestamp {
        self.modified_timestamp
    }
}

use crate::traits::Timestamp as _;

// FIXME: Implement `fmt::Display` (to your liking) for `Metadata`.
impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let created = self.created_timestamp;
        let accessed = self.accessed_timestamp;
        let modified = self.modified_timestamp;
        write!(f, "{:0<2}/{:0<2}/{:0<2} {:0<2}:{:0<2}:{:0<2}    ", created.year(), created.month(), created.day(), created.hour(), created.minute(), created.second())?;
        write!(f, "{:0<2}/{:0<2}/{:0<2} {:0<2}:{:0<2}:{:0<2}    ", modified.year(), modified.month(), modified.day(), modified.hour(), modified.minute(), modified.second())
    }
}


impl From<u16> for Date {
    fn from(v: u16) -> Date {
        Date(v)
    }
}

impl From<u16> for Time {
    fn from(v: u16) -> Time {
        Time(v)
    }
}

impl From<u8> for Attributes {
    fn from(v: u8) -> Attributes {
        Attributes(v)
    }
}
