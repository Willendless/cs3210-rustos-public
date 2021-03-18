use alloc::string::String;

use shim::io::{self, SeekFrom};
use shim::ioerr;

use crate::traits;
use crate::vfat::{Cluster, Metadata, VFatHandle};

#[derive(Debug)]
pub struct File<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    // FIXME: Fill me in.
    pub start_cluster: Cluster,
    pub metadata: Metadata,
    pub name: String,
    pub pos: u64,
    pub size: u64,
}

// FIXME: Implement `traits::File` (and its supertraits) for `File`.
impl<HANDLE: VFatHandle> traits::File for File<HANDLE> {
    fn sync(&mut self) -> io::Result<()> {
        panic!("dummy")
    }
    fn size(&self) -> u64 {
        self.size
    }
    fn is_end(&self) -> bool {
        self.size == self.pos
    }
}

impl<HANDLE: VFatHandle> io::Seek for File<HANDLE> {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        match _pos {
            SeekFrom::Start(off) => {
                if off > self.size {
                    return ioerr!(InvalidInput, "seek: seeking before the start of the file");
                }
                self.pos = off;
            },
            SeekFrom::End(off) => {
                if off > 0 {
                    return ioerr!(InvalidInput, "seek: seeking after the end of the file");
                }
                self.pos = self.size;
            },
            SeekFrom::Current(off) => {
                if off < 0 && (self.pos as i64) + off < 0 {
                    return ioerr!(InvalidInput, "seek: seeking before the start of the file");
                } else if off > 0 && self.pos + (off as u64) > self.size {
                    return ioerr!(InvalidInput, "seek: seeking after the end of the file");
                } 
                self.pos = (self.pos as i64 + off) as u64;
            }
        };
        Ok(self.pos as u64)
    }
}

impl<HANDLE: VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        // if self.size == 0 || self.pos >= self.size {
        //     return Ok(0);
        // }
        if self.size == 0 { return Ok(0); }
        // read from current pos of the file
        let read_result = self.vfat.lock(|vfat| {
            let max_read_size = ((self.size - self.pos).min(_buf.len() as u64)) as usize;
            vfat.read_cluster(self.start_cluster, self.pos as usize, &mut _buf[..max_read_size])
        });
        if let Ok(read_size) = read_result {
            self.pos += read_size as u64;
            Ok(read_size)
        } else {
            read_result
        }
    }
}

impl<HANDLE: VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        panic!("dummy")
    }
    fn flush(&mut self) -> io::Result<()> {
        panic!("dummy")
    }
}
