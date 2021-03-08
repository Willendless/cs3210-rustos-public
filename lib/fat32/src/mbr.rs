use core::{fmt, mem};
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;
use core::slice::Iter;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    // FIXME: Fill me in.
    starting_head: u8,
    starting_sector: u8,
    starting_cylinder: u8,
}

// FIXME: implement Debug for CHS
impl fmt::Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CHS")
         .field("starting_head", &self.starting_head)
         .field("starting_sector", &self.starting_sector)
         .field("starting_cylinder", &self.starting_cylinder)
         .finish()
    }
}

const_assert_size!(CHS, 3);

#[repr(C, packed)]
pub struct PartitionEntry {
    // FIXME: Fill me in.
    boot_indicator: u8,
    chs: CHS,
    pub partition_type: u8,
    ending_head: u8,
    ending_sector: u8,
    ending_cylinder: u8,
    pub relative_sector: u32, // offset from disk to start of the partition
    pub total_sectors_in_partition: u32
}

// FIXME: implement Debug for PartitionEntry
impl fmt::Debug for PartitionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PartitionEntry")
         .field("boot_indicator", &{ self.boot_indicator })
         .field("partition_type", &{ self.partition_type })
         .field("relative_sector", &{ self.relative_sector })
         .field("total_sectors_in_partition", &{ self.total_sectors_in_partition })
         .finish()
    }
}

const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    // FIXME: Fill me in.
    bootstrap: [u8; 436],
    disk_id: [u8; 10],
    pub partition_table: [PartitionEntry; 4],
    magic: [u8; 2],
}

// FIXME: implemente Debug for MaterBootRecord
impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MasterBoot")
         .field("disk_id", &self.disk_id)
         .field("partition_table", &self.partition_table)
         .field("magic", &self.magic)
         .finish()
    }
}

const_assert_size!(MasterBootRecord, 512);

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

const MAGIC: [u8; 2] = [0x55, 0xAA];
const BOOT_INDICATOR: [u8; 2] = [0, 0x80];
const MBR_SECTOR_NUM: u64 = 0;

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        let mut buf: [u8; 512] = [0; 512];
        if let Err(e) = device.read_sector(MBR_SECTOR_NUM, &mut buf) {
            // io error: failed to read the sector
            return Err(Error::Io(e));
        }

        let mbr = unsafe { mem::transmute::<[u8; 512], MasterBootRecord>(buf) };
        if mbr.magic != MAGIC {
            // bad magic signature
            return Err(Error::BadSignature);
        }
        for (i, entry) in mbr.partition_table.iter().enumerate() {
            if !BOOT_INDICATOR.contains(&{ entry.boot_indicator }) {
                // bad boot indicator
                return Err(Error::UnknownBootIndicator(i as u8));
            }
        }
        Ok(mbr)
    }

    /// Return a iterator of partition entry
    pub fn iter(&self) -> Iter<'_, PartitionEntry>{
        self.partition_table.iter()
    }
}
