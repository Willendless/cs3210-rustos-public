use alloc::string::String;
use alloc::vec::Vec;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::{io, ioerr};

use crate::traits;
use crate::util::VecExt;
use crate::vfat::{Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, File, VFatHandle};

use core::str;
use core::char;

#[derive(Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    // FIXME: Fill me in.
    pub start_cluster: Cluster,
    pub metadata: Metadata,
    pub name: String,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct VFatRegularDirEntry {
    // FIXME: Fill me in.
    pub name: [u8; 8],
    pub extension: [u8; 3],
    pub attributes: u8,
    _1: [u8; 2],
    pub created_time: Time,
    pub created_date: Date,
    pub accessed_date: Date,
    pub cluster_id_hi: u16,
    pub modified_time: Time,
    pub modified_date: Date,
    pub cluster_id_lo: u16,
    pub file_size: u32,
}

const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct VFatLfnDirEntry {
    // FIXME: Fill me in.
    pub sequence_num: u8,
    pub name_1: [u16; 5],
    pub attributes: u8, // use to determine if it is LFN
    _1: [u8; 2],
    pub name_2: [u16; 6],
    _2: [u8; 2],
    pub name_3: [u16; 2],
}

const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    // FIXME: Fill me in.
    pub id: u8,
    _1: [u8; 10],
    pub attributes: u8,
    _2: [u8; 20],
}

const_assert_size!(VFatUnknownDirEntry, 32);

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

#[derive(Debug)]
pub enum VFatWrapEntry {
    Reguler(VFatRegularDirEntry),
    LongFilename(VFatLfnDirEntry),
}

use crate::traits::Dir as DirTrait;
use crate::traits::Entry as EntryTrait;

impl<HANDLE: VFatHandle> Dir<HANDLE> {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry<HANDLE>> {
        let name = name.as_ref();
        if let Some(name) = name.to_str() {
            let entries = self.entries()?;
            for entry in entries {
                if name.eq_ignore_ascii_case(entry.name()) {
                    return Ok(entry);
                }
            }
            ioerr!(NotFound, "Dir::find: failed to find name")
        } else {
            ioerr!(InvalidInput, "Dir::find: input name contains invalid UTF-8 characters")
        }
    }

    pub fn root_dir(vfat: HANDLE) -> Self {
        let root_dir_cluster = vfat.lock(|vfat| vfat.rootdir_cluster());
        Dir {
            vfat,
            start_cluster: root_dir_cluster,
            metadata: Default::default(),
            name: "/".into(),
        }
    }

}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    // FIXME: Implement `trait::Dir` for `Dir`.
    type Entry = Entry<HANDLE>;
    type Iter = DirIter<HANDLE>;

    fn entries(&self) -> io::Result<DirIter<HANDLE>> {
        let mut buf: Vec<u8> = vec![];
        self.vfat.lock(|vfat| {
            vfat.read_chain(self.start_cluster, &mut buf)?;
            let buf: Vec<VFatDirEntry> = unsafe { VecExt::cast(buf) };
            Ok(DirIter {
                vfat: self.vfat.clone(),
                dir_entry_buf: buf,
                expect_index: 0,
            })
        })
    }
}

impl VFatDirEntry {
    const ATTR_LFN_FLAG: u8 = 0x0F;
    const ID_UNUSED_ENTRY: u8 = 0xE5;
    const ID_LAST_ENTRY: u8 = 0;

    fn to_unknown(&self) -> VFatUnknownDirEntry {
        unsafe { self.unknown }
    }

    fn to_wrap_entry(&self) -> VFatWrapEntry {
        match self.to_unknown().attributes {
            Self::ATTR_LFN_FLAG => VFatWrapEntry::LongFilename(unsafe { self.long_filename }),
            _ => VFatWrapEntry::Reguler(unsafe { self.regular }),
        }
    }

    fn is_last_entry(&self) -> bool {
        self.to_unknown().id == Self::ID_LAST_ENTRY
    }

    fn is_unused_entry(&self) -> bool {
        self.to_unknown().id == Self::ID_UNUSED_ENTRY
    }
}

impl VFatRegularDirEntry {
    const ATTR_DIRECTORY_FLAG: u8 = 0x10;

    fn is_directory(&self) -> bool {
        (self.attributes & Self::ATTR_DIRECTORY_FLAG) != 0
    }

    fn metadata(&self) -> Metadata {
        Metadata {
          attributes: self.attributes.into(),
          created_timestamp: Timestamp {
              date: self.created_date.into(),
              time: self.created_time.into(),
          },
          accessed_timestamp: Timestamp {
              date: self.accessed_date.into(),
              ..Default::default()
          },
          modified_timestamp: Timestamp {
              date: self.modified_date.into(),
              time: self.modified_time.into(),
          },
       }
    }
}

impl VFatLfnDirEntry {
    const NAME_END_FLAG1: u16 = 0;
    const NAME_END_FLAG2: u16 = 0xFFFF;
    fn extract_name(&self) -> String {
        let mut u16_vec: Vec<u16> = vec![];
        unsafe {
            u16_vec.extend_from_slice(&self.name_1);
            u16_vec.extend_from_slice(&self.name_2);
            u16_vec.extend_from_slice(&self.name_3);
        }
        for (i, ch) in u16_vec.as_slice().iter().enumerate() {
            if *ch == Self::NAME_END_FLAG1
                || *ch == Self::NAME_END_FLAG2 {
                    return Self::decode_u16(&u16_vec[..i]);
            }
        }
        return Self::decode_u16(&u16_vec);
    }

    fn decode_u16(buf: &[u16]) -> String {
        char::decode_utf16(buf.iter().cloned())
            .map(|r| r.expect("VFatLfnDirEntry::extract_name: failed to decode u16"))
            .collect::<String>()
    }
}

pub struct DirIter<HANDLE: VFatHandle> {
    vfat: HANDLE,
    dir_entry_buf: Vec<VFatDirEntry>,
    expect_index: usize,
}

impl<HANDLE: VFatHandle> Iterator for DirIter<HANDLE> {
    type Item = Entry<HANDLE>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.expect_index >= self.dir_entry_buf.len() {
            return None;
        }

        let mut name: Vec<String> = vec![];
        let mut metadata: Metadata = Default::default();
        let mut start_cluster: Cluster = 0.into();
        let mut is_directory = false;
        let mut is_lfn = false;
        let mut size = 0;
        let mut n = 0;

        for dir_entry in self.dir_entry_buf[self.expect_index..].iter() {
            if dir_entry.is_last_entry() { break; }
            n += 1;
            if dir_entry.is_unused_entry() { continue; }

            match dir_entry.to_wrap_entry() {
                VFatWrapEntry::Reguler(regular_entry) => {
                    if !is_lfn {
                        let extension = parse_str_from_byte(&regular_entry.extension);
                        name.push(parse_str_from_byte(&regular_entry.name));
                        if extension.len() > 0 {
                            name.push(".".into());
                            name.push(extension);
                        }
                    }
                    // entry corresponding start cluster
                    start_cluster = get_u32_from_u16(regular_entry.cluster_id_hi, regular_entry.cluster_id_lo).into();
                    // entry type is directory or file
                    is_directory = regular_entry.is_directory();
                    // entry metadata
                    metadata = regular_entry.metadata();
                    // entry size
                    size = regular_entry.file_size;
                    break;
                },
                VFatWrapEntry::LongFilename(lfn_entry) => {
                    is_lfn = true; // indicate lfn
                    let sequence_num = lfn_entry.sequence_num as usize;
                    let name_segment = lfn_entry.extract_name();
                    if name.len() <= sequence_num {
                        name.resize_with(sequence_num + 1, || "".into());
                    }
                    name[sequence_num] = name_segment;
                },
            }
        }

        self.expect_index += n;

        if name.len() == 0 {
            return None;
        }

        // construct final name
        let name = name.into_iter()
                              .fold(String::new(), |res, cur| res + &cur);
    
        if is_directory {
            Some(Entry::Dir(Dir {
                name,
                metadata,
                start_cluster,
                vfat: self.vfat.clone(),
            }))
        } else {
            Some(Entry::File(File {
                name,
                pos: 0,
                metadata,
                start_cluster,
                size: size as u64,
                vfat: self.vfat.clone(),
            }))
        }
    }
}

/// Return string from the first 8 bytes of the entry.
/// A file name may be terminated early using 0x00 or 0x20 characters.
fn parse_str_from_byte(buf: &[u8]) -> String {
    let mut end = 0;
    for byte in buf.iter() {
        if *byte == 0x0 || *byte == 0x20 {
            break;
        }
        end += 1;
    }
    str::from_utf8(&buf[..end]).expect("parse_str_from_byte: failed to parse utf8").into()
}

#[inline(always)]
fn get_u32_from_u16(hi: u16, lo: u16) -> u32 {
    ((hi as u32) << 16) | (lo as u32)
}
