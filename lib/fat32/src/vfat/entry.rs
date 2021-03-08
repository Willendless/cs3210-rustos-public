use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};

// You can change this definition if you want
#[derive(Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    File(File<HANDLE>),
    Dir(Dir<HANDLE>),
}

// TODO: Implement any useful helper methods on `Entry`.
impl<HANDLE: VFatHandle> Entry<HANDLE> {

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
