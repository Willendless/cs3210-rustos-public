use core::{fmt, mem};
use shim::const_assert_size;

use crate::traits::BlockDevice;
use crate::vfat::Error;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    // FIXME: Fill me in.
    _1: [u8; 11],
    pub bytes_per_sector: u16, // bytes per logical sector
    pub sectors_per_cluster: u8,
    pub reserved_sectors_num: u16, // offset from partition start to FAT
    pub fat_num: u8,
    _2: [u8; 2],
    sectors_num_1: u16,
    _3: u8,
    pub sectors_per_fat_1: u16,
    _4: [u8; 8],
    sectors_num_2: u32,
    pub sectors_per_fat_2: u32,
    _5: [u8; 4],
    pub rootdir_cluster: u32,
    _6: [u8; 462],
    magic: [u8; 2],
}

const_assert_size!(BiosParameterBlock, 512);


impl Default for BiosParameterBlock {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

const MAGIC: [u8; 2] = [0x55, 0xAA];

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut buf = [0u8; 512];
        device.read_sector(sector, &mut buf)?;
        let ebpb = unsafe { mem::transmute::<[u8; 512], BiosParameterBlock>(buf) };
        if MAGIC != ebpb.magic {
            Err(Error::BadSignature)
        } else {
            Ok(ebpb)
        }
    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BIosParameterBlock")
         .field("bytes per sector", &{ self.bytes_per_sector })
         .field("sectors per cluster", &{ self.sectors_per_cluster })
         .field("reserved sectors number", &{ self.reserved_sectors_num })
         .field("fat num", &{ self.fat_num })
         .field("sectors num", &{ self.sectors_num_2 })
         .field("sectors per fat", &{ self.sectors_per_fat_2 })
         .field("cluster num of root dir", &{ self.rootdir_cluster })
         .finish()
    }
}
