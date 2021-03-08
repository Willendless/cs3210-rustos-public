use alloc::boxed::Box;
use alloc::vec::Vec;
use core::fmt;
use hashbrown::HashMap;
use shim::{io, ioerr};

use crate::traits::BlockDevice;

#[derive(Debug)]
struct CacheEntry {
    data: Vec<u8>,
    dirty: bool,
}

/// Abstract representation of partition to upper layer.
pub struct Partition {
    /// The physical sector where the partition begins.
    pub start: u64,
    /// Number of sectors
    pub num_sectors: u64,
    /// The size, in bytes, of a logical sector in the partition.
    pub sector_size: u64,
}

pub struct CachedPartition {
    device: Box<dyn BlockDevice>,
    cache: HashMap<u64, CacheEntry>,
    partition: Partition,
}

impl CachedPartition {
    /// Creates a new `CachedPartition` that transparently caches sectors from
    /// `device` and maps physical sectors to logical sectors inside of
    /// `partition`. All reads and writes from `CacheDevice` are performed on
    /// in-memory caches.
    ///
    /// The `partition` parameter determines the size of a logical sector and
    /// where logical sectors begin. An access to a sector `0` will be
    /// translated to physical sector `partition.start`. Virtual sectors of
    /// sector number `[0, num_sectors)` are accessible.
    ///
    /// `partition.sector_size` must be an integer multiple of
    /// `device.sector_size()`.
    ///
    /// # Panics
    ///
    /// Panics if the partition's sector size is < the device's sector size.
    pub fn new<T>(device: T, partition: Partition) -> CachedPartition
    where
        T: BlockDevice + 'static,
    {
        assert!(partition.sector_size >= device.sector_size());

        CachedPartition {
            device: Box::new(device),
            cache: HashMap::new(),
            partition: partition,
        }
    }

    /// Returns the number of physical sectors that corresponds to
    /// one logical sector.
    fn factor(&self) -> u64 {
        self.partition.sector_size / self.device.sector_size()
    }

    /// Maps a user's request for a sector `virt` to the physical sector.
    /// Returns `None` if the virtual sector number is out of range.
    fn virtual_to_physical(&self, virt: u64) -> Option<u64> {
        if virt >= self.partition.num_sectors {
            return None;
        }

        let physical_offset = virt * self.factor();
        let physical_sector = self.partition.start + physical_offset;

        Some(physical_sector)
    }

    /// Returns a mutable reference to the cached sector `sector`. If the sector
    /// is not already cached, the sector is first read from the disk.
    ///
    /// The sector is marked dirty as a result of calling this method as it is
    /// presumed that the sector will be written to. If this is not intended,
    /// use `get()` instead.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get_mut(&mut self, sector: u64) -> io::Result<&mut [u8]> {
        if self.cache.contains_key(&sector) {
            let cache_entry = self.cache.get_mut(&sector).unwrap();
            cache_entry.dirty = true;
            Ok(cache_entry.data.as_mut_slice())
        } else {
            let mut buf = vec![0; self.sector_size() as usize];
            self.read_sector(sector, &mut buf[..])?;
            self.cache.insert(sector, CacheEntry {
                data: buf,
                dirty: true,
            });
            let cache_entry = self.cache.get_mut(&sector).unwrap();
            Ok(cache_entry.data.as_mut_slice())
        }
    }

    /// Returns a reference to the cached sector `sector`. If the sector is not
    /// already cached, the sector is first read from the disk.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error reading the sector from the disk.
    pub fn get(&mut self, sector: u64) -> io::Result<&[u8]> {
        if self.cache.contains_key(&sector) {
            let cache_entry = self.cache.get(&sector).unwrap();
            Ok(cache_entry.data.as_slice())
        } else {
            let mut buf = vec![0; self.partition.sector_size as usize];
            self.read_sector(sector, &mut buf[..])?;
            self.cache.insert(sector, CacheEntry {
                data: buf,
                dirty: false,
            });
            let cache_entry = self.cache.get(&sector).unwrap();
            Ok(cache_entry.data.as_slice())
        }
    }
}

// FIXME: Implement `BlockDevice` for `CacheDevice`. The `read_sector` and
// `write_sector` methods should only read/write from/to cached sectors.
impl BlockDevice for CachedPartition {
    fn sector_size(&self) -> u64 {
        self.partition.sector_size
    }

    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> io::Result<usize> {
        eprintln!("read_sector: {}", sector);
        let physical_sec_size = self.device.sector_size(); // max bytes read each time
        let read_size = buf.len(); // expected read size
        if let Some(start_physical_sec) = self.virtual_to_physical(sector) {
            // start physical sector exists
            for (i, chunks) in &mut buf[..read_size]
                .chunks_mut(physical_sec_size as usize).enumerate() {
                    self.device.read_sector(i as u64 + start_physical_sec, chunks)?;
            }
            Ok(read_size)
        } else {
            ioerr!(UnexpectedEof, "sector number is out of range")
        }
    }

    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> io::Result<usize> {
        let physical_sec_size = self.device.sector_size();
        let write_size = buf.len().min(self.sector_size() as usize);
        if let Some(start_physical_sec) = self.virtual_to_physical(sector) {
            // start physical sector exists
            for (i, chunks) in buf[..write_size]
                .chunks(physical_sec_size as usize).enumerate() {
                    self.device.write_sector(i as u64 + start_physical_sec, chunks)?;
            }
            Ok(write_size)
        } else {
            ioerr!(UnexpectedEof, "sector number is out of range")
        }
    }
}

impl fmt::Debug for CachedPartition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CachedPartition")
            .field("device", &"<block device>")
            .field("cache", &self.cache)
            .finish()
    }
}
