use core::fmt::Debug;
use core::marker::PhantomData;

use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Path;

use crate::mbr::MasterBootRecord;
pub use crate::mbr::PartitionEntry;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::SliceExt;
use crate::vfat::{BiosParameterBlock, CachedPartition, Partition};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, Status};

/// A generic trait that handles a critical section as a closure
pub trait VFatHandle: Clone + Debug + Send + Sync {
    fn new(val: VFat<Self>) -> Self;
    fn lock<R>(&self, f: impl FnOnce(&mut VFat<Self>) -> R) -> R;
}

#[derive(Debug)]
pub struct VFat<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    device: CachedPartition,
    bytes_per_sector: u32, // TODO: previously u16
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    rootdir_cluster: Cluster,
}

const FAT32_PARTITION_TYPE: [u8; 2] = [0xB, 0xC];

impl<HANDLE: VFatHandle> VFat<HANDLE> {
    pub fn from<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
        // data in partition_entry
        let mut flag = false;
        let mut partition_start_sector: u64 = 0;
        let mut partition_physical_sectors_num: u64 = 0;
        let mut bios_parameter_block: BiosParameterBlock = Default::default();

        let master_boot_record = MasterBootRecord::from(&mut device)?;
        for partition_entry in master_boot_record.partition_table.iter() {
            // currently only able to handle fat32
            if FAT32_PARTITION_TYPE.contains(&partition_entry.partition_type) {
                flag = true;
                partition_start_sector = partition_entry.relative_sector as u64;
                partition_physical_sectors_num = partition_entry.total_sectors_in_partition as u64;
                bios_parameter_block = BiosParameterBlock::from(&mut device, partition_entry.relative_sector as u64)?;
                break;
            }
        }

        if !flag {
            return Err(Error::Io(newioerr!(NotFound, "failed to find FAT32 format partition")));
        }

        eprintln!("{:#?}", master_boot_record);
        eprintln!("{:#?}", bios_parameter_block);
        let fat_start_sector = bios_parameter_block.reserved_sectors_num as u64;
        let fat_num = bios_parameter_block.fat_num as u64;
        let sectors_per_fat = bios_parameter_block.sectors_per_fat_2;
        let bytes_per_logical_sector = bios_parameter_block.bytes_per_sector as u32;
        let partition = Partition {
            start: partition_start_sector,
            num_sectors: partition_physical_sectors_num * 512 / (bytes_per_logical_sector as u64),
            sector_size: bytes_per_logical_sector as u64,
        };
        Ok(HANDLE::new(VFat {
            phantom: PhantomData,
            device: CachedPartition::new(device, partition),
            bytes_per_sector: bytes_per_logical_sector,
            sectors_per_cluster: bios_parameter_block.sectors_per_cluster,
            sectors_per_fat: sectors_per_fat,
            fat_start_sector: fat_start_sector,
            data_start_sector: fat_start_sector + fat_num * (sectors_per_fat as u64),
            rootdir_cluster: Cluster::from(bios_parameter_block.rootdir_cluster),
        }))
    }

    /// Maps a cluster id to its start sector id
    pub fn cluster_to_sector(&self, cluster: Cluster) -> u64 {
        self.data_start_sector + (self.sectors_per_cluster as u64) * (cluster.cluster_id() - 2)
    }

    /// Maps a cluster and a offset to corresponding sector
    pub fn cluster_by_offset(&mut self, start_cluster: Cluster, offset: usize) -> io::Result<Option<Cluster>> {
        let mut cluster = start_cluster;
        let cnt = offset / (self.bytes_per_cluster() as usize);
        for _ in 0..cnt {
            let fat_entry = self.fat_entry(cluster)?;
            match fat_entry.status() {
                Status::Data(next_cluster) => cluster = next_cluster,
                Status::Eoc(_) => return Ok(None),
                Status::Bad => return ioerr!(InvalidData, "next cluster is bad"),
                Status::Reserved => return ioerr!(InvalidData, "next cluster is reserved"),
                Status::Free => return ioerr!(InvalidData, "next cluster is free"),
            }
        }
        Ok(Some(cluster))
    }

    /// Read from an offset of a cluster into a buffer.
    pub fn read_cluster(&mut self, cluster: Cluster, offset: usize, buf: &mut [u8]) -> io::Result<usize> {
        // get current cluster
        let cluster = self.cluster_by_offset(cluster, offset)?;
        let mut cluster = match cluster {
            Some(c) => c,
            None => return Ok(0),
        };
        // get start sector of current cluster
        let mut cluster_start_sector = self.cluster_to_sector(cluster);
        // calc offset in sector
        let offset_by_cluster = ((offset as u64) % self.bytes_per_cluster()) as usize;
        // get current sector
        let mut sector = cluster_start_sector + (offset_by_cluster as u64) / (self.bytes_per_sector as u64);
        // offset by sector
        let offset = offset_by_cluster % (self.bytes_per_sector as usize);
        // expected read size from first cluster
        let buf_len = buf.len();
        let mut expected_read_size = buf_len;

        eprintln!("vfat::read_cluster cluster: {}, cluster start sector {}, read start sector: {}, offset:{}>>>>>>>>>>>>", cluster.cluster_id(), self.cluster_to_sector(cluster), sector, offset);

        // first sector need special treatment
        let ptr = self.device.get(sector)?;
        let first_sector_max_read_size = (self.bytes_per_sector as usize) - offset;
        if first_sector_max_read_size >= expected_read_size {
            // only need to read part of first sector
            buf[..].clone_from_slice(&ptr[offset..offset + expected_read_size]);
            eprintln!("read_cluster finished, read_size: {}^^^^^^^^;", expected_read_size);
            eprintln!("");
            return Ok(expected_read_size)
        } else {
            buf[..first_sector_max_read_size].clone_from_slice(&ptr[offset..]);
            expected_read_size -= first_sector_max_read_size;
        }

        // feed other parts of buf
        sector += 1;
        let sector_len = self.bytes_per_sector as usize;

        for sector_buf in buf[first_sector_max_read_size..].rchunks_mut(self.bytes_per_sector as usize) {
            if sector - cluster_start_sector >= (self.sectors_per_cluster as u64) {
                // zero sector left in this cluster, forward to next cluster
                let fat_entry = self.fat_entry(cluster)?;
                match &{fat_entry}.status() {
                    Status::Data(next_cluster) => cluster = *next_cluster,
                    Status::Eoc(_) => {
                        // eprintln!("read_cluster finished^^^^^^^, read_size: {};", buf_len - expected_read_size);
                        // eprintln!("");
                        // return Ok(buf_len - expected_read_size);
                        return ioerr!(InvalidData, "read_cluster: reach end of cluster");
                    },
                    Status::Bad => return ioerr!(InvalidData, "read_cluster: next cluster is bad"),
                    Status::Reserved => return ioerr!(InvalidData, "read_cluster: next cluster is reserved"),
                    Status::Free => return ioerr!(InvalidData, "read_cluster: next cluster is free"),
                }
                // set sector index
                sector = self.cluster_to_sector(cluster);
                // set cluster start sector
                cluster_start_sector = sector;
            }
            if expected_read_size >= sector_len {
                // able to read whole sector
                let ptr = self.device.get(sector)?;
                sector_buf[..].clone_from_slice(&ptr[..]);
                expected_read_size -= sector_len;
            } else {
                // left place in buf cannot hold a whole sector
                let ptr = self.device.get(sector)?;
                sector_buf[..].clone_from_slice(&ptr[..expected_read_size]);
                eprintln!("read finished at offset : {}", expected_read_size);
                expected_read_size = 0;
                break;
            }
            // read finish, proceed to next sector 
            sector += 1;
        }
        eprintln!("read_cluster finished, read size: {}, cur sector: {}^^^^^^^;", buf_len - expected_read_size, sector);
        eprintln!("");
        return Ok(buf_len - expected_read_size);
    }

    /// Read all of the clusters chained from a starting cluster
    /// into a vector.
    pub fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut cluster = start;
        let mut read_size = 0;
        let cluster_size = self.bytes_per_cluster();
        loop {
            // read data from current cluster_id
            let start_sector = self.cluster_to_sector(cluster);
            for i in 0..self.sectors_per_cluster as u64 {
                let ptr = self.device.get(start_sector + i)?;
                buf.extend_from_slice(ptr);
            }
            read_size += cluster_size;
            // try fetch next cluster
            let fat_entry = self.fat_entry(cluster)?;
            match fat_entry.status() {
                Status::Data(next_cluster) => cluster = next_cluster,
                Status::Eoc(_) => {
                    return Ok(read_size as usize);
                },
                Status::Bad => return ioerr!(InvalidData, "read_chain: next cluster is bad"),
                Status::Reserved => return ioerr!(InvalidData, "read_chain: next cluster is reserved"),
                Status::Free => return ioerr!(InvalidData, "read_chain: next cluster is free"),
            }
        }
    }

    /// Return a reference to a `FatEntry` for a cluster where the
    /// reference points directly into a cached sector.
    pub fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
        // calc logical sector of the fat entry
        let sector = self.cluster_to_fat_entry_sector(cluster);
        // calc fat_entry index
        let index = self.cluster_to_fat_entry_sector_index(cluster);
        eprintln!("chluster: {} --------------> sector: {}, index: {}", cluster.cluster_id(), sector, index);
        // eprintln!("vfat::read_fat_entry fat start sector {} cluster {} sector {}, index {}", self.fat_start_sector, cluster.cluster_id(), sector, index);
        // eprintln!("vfat::read_fat_entry fat sector num {}", self.sectors_per_fat);

        // read corresponding sector of the fat entry
        let sector_ptr = self.device.get(sector)?;

        // cast &[u8] to &[FatEntry]
        let sector_ptr: &[FatEntry] = unsafe { SliceExt::cast(sector_ptr) };
        Ok(&sector_ptr[index as usize])
    }

    /// Return bytes per cluster
    pub fn bytes_per_cluster(&self) -> u64 {
        self.bytes_per_sector as u64 * self.sectors_per_cluster as u64
    }

    /// Return rootdir_cluster
    pub fn rootdir_cluster(&self) -> Cluster {
        self.rootdir_cluster
    }

    /// Return cluster corresponding fat entry sector
    pub fn cluster_to_fat_entry_sector(&self, cluster: Cluster) -> u64 {
        self.fat_start_sector + (cluster.cluster_id() << 2) / (self.bytes_per_sector as u64)
    }

    /// Return cluster corresponding fat entry index in its sector
    pub fn cluster_to_fat_entry_sector_index(&self, cluster: Cluster) -> u64 {
        cluster.cluster_id() % ((self.bytes_per_sector >> 2) as u64)
    }
}

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = crate::vfat::File<HANDLE>;
    type Dir = crate::vfat::Dir<HANDLE>;
    type Entry = crate::vfat::Entry<HANDLE>;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        let root_dir = Dir::root_dir(self.clone());
        let mut cur_entry = Entry::Dir(root_dir);
        let mut flag = false;

        // check empty and absolute dir first
        let mut components = path.as_ref().components();
        match components.next() {
            Some(first_component) => {
                if first_component != path::Component::RootDir {
                    return ioerr!(InvalidInput, "FileSystem::open: path not absolute");
                }
            },
            None => return ioerr!(InvalidInput, "FileSystem::open: path not absolute"),
        }

        for component in components {
            if flag {
                return ioerr!(InvalidInput, "FileSystem::open: failed open component directory");
            }
            let cur_dir = match cur_entry {
                Entry::File(_) => return ioerr!(InvalidInput, "FileSystem::open: component of file in path"),
                Entry::Dir(ref dir) => dir,
            };

            match component {
                path::Component::ParentDir => {
                    if let Ok(next_entry) = cur_dir.find("..") {
                        cur_entry = next_entry;
                    } else {
                        flag = true;
                    }
                }, 
                path::Component::Normal(name) => {
                    if let Ok(next_entry) = cur_dir.find(name) {
                        cur_entry = next_entry;
                    } else {
                        flag = true;
                    }
                }
                _ => continue,
            }
        }

        if flag {
            return ioerr!(NotFound, "FileSystem::open: failed to find file or directory");
        }
        Ok(cur_entry)
    }
}
