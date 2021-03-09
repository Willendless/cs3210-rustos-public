use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};
use core::fmt;

// You can change this definition if you want
#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    File(File<HANDLE>),
    Dir(Dir<HANDLE>),
}

// TODO: Implement any useful helper methods on `Entry`.
impl<HANDLE: VFatHandle> Entry<HANDLE> {
    pub fn is_hidden(&self) -> bool {
        use crate::traits::Entry;
        use crate::traits::Metadata;
        self.metadata().hidden()
    }

    fn size(&self) -> u64 {
        use crate::traits::File;
        match self {
            Entry::File(f) => f.size(),
            Entry::Dir(_) => 0,
        }
    }
}

impl<HANDLE: VFatHandle> fmt::Display for Entry<HANDLE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use crate::traits::Entry;
        use crate::traits::Metadata;
        let metadata = self.metadata();
        let type_flag = if self.is_file() { "f" } else { "d" };
        let rw_flag = if metadata.read_only() { "r" } else { "w" };
        let size = self.size();
        write!(f, "{}{}-- {:<20} {:<8} {:<20}", type_flag, rw_flag, metadata, size, self.name())
    }
}

impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    // FIXME: Implement `traits::Entry` for `Entry`.
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Metadata = Metadata;

    fn name(&self) -> &str {
        match self {
            Entry::File(f) => &f.name,
            Entry::Dir(dir) => &dir.name,
        }
    }

    fn metadata(&self) -> &Metadata {
        match self {
            Entry::File(f) => &f.metadata,
            Entry::Dir(dir) => &dir.metadata,
        }
    }

    fn as_file(&self) -> Option<&File<HANDLE>> {
        match self {
            Entry::File(f) => Some(f),
            Entry::Dir(_) => None,
        }
    }

    fn as_dir(&self) -> Option<&Dir<HANDLE>> {
        match self {
            Entry::File(_) => None,
            Entry::Dir(dir) => Some(dir),
        }
    }

    fn into_file(self) -> Option<File<HANDLE>> {
        match self {
            Entry::File(f) => Some(f),
            Entry::Dir(_) => None,
        }
    }

    fn into_dir(self) -> Option<Dir<HANDLE>> {
        match self {
            Entry::File(_) => None,
            Entry::Dir(dir) => Some(dir),
        }
    }
}
