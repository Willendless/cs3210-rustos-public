use core::iter::Chain;
use core::ops::{Deref, DerefMut};
use core::slice::Iter;

use alloc::boxed::Box;
use alloc::fmt;
use core::alloc::{GlobalAlloc, Layout};

use crate::allocator;
use crate::param::*;
use crate::vm::{PhysicalAddr, VirtualAddr};
use crate::ALLOCATOR;
use crate::console::kprintln;

use aarch64::vmsa::*;
use shim::const_assert_size;

#[repr(C)]
#[derive(Clone)]
pub struct Page([u8; PAGE_SIZE]);
const_assert_size!(Page, PAGE_SIZE);

impl Page {
    pub const SIZE: usize = PAGE_SIZE;
    pub const ALIGN: usize = PAGE_SIZE;

    fn layout() -> Layout {
        unsafe { Layout::from_size_align_unchecked(Self::SIZE, Self::ALIGN) }
    }
}

#[repr(C)]
#[repr(align(65536))]
#[derive(Clone)]
pub struct L2PageTable {
    pub entries: [RawL2Entry; 8192],
}
const_assert_size!(L2PageTable, PAGE_SIZE);

impl L2PageTable {
    /// Returns a new `L2PageTable`
    fn new() -> L2PageTable {
        L2PageTable {
            entries: [RawL2Entry::new(0); 8192],
        }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        (self as *const L2PageTable as *const usize as usize).into()
    }
}

#[derive(Copy, Clone)]
pub struct L3Entry(RawL3Entry);

impl L3Entry {
    /// Returns a new `L3Entry`.
    fn new() -> L3Entry {
        L3Entry(RawL3Entry::new(0))
    }

    /// Returns `true` if the L3Entry is valid and `false` otherwise.
    fn is_valid(&self) -> bool {
        self.0.get_masked(1) == 1
    }

    /// Extracts `ADDR` field of the L3Entry and returns as a `PhysicalAddr`
    /// if valid. Otherwise, return `None`.
    fn get_page_addr(&self) -> Option<PhysicalAddr> {
        if self.is_valid() {
            Some(self.0.get_masked(RawL3Entry::ADDR).into())
        } else {
            None
        }
    }
}

#[repr(C)]
#[repr(align(65536))]
#[derive(Clone)]
pub struct L3PageTable {
    pub entries: [L3Entry; 8192],
}
const_assert_size!(L3PageTable, PAGE_SIZE);

impl L3PageTable {
    /// Returns a new `L3PageTable`.
    fn new() -> L3PageTable {
        L3PageTable {
            entries: [L3Entry::new(); 8192],
        }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        (self as *const L3PageTable as *const usize as usize).into()
    }
}

#[repr(C)]
#[repr(align(65536))]
#[derive(Clone)]
pub struct PageTable {
    pub l2: L2PageTable,
    pub l3: [L3PageTable; 2],
}

impl PageTable {

    const PT_L2_INDEX_MASK: usize = 0x3FF_E000_0000;
    const PT_L3_INDEX_MASK: usize = 0x1FFF_0000;

    /// Returns a new `Box` containing `PageTable`.
    /// Entries in L2PageTable should be initialized properly before return.
    fn new(perm: u64) -> Box<PageTable> {
        let mut pt = unsafe { Box::new(PageTable {
            l2: L2PageTable::new(),
            l3: [L3PageTable::new(), L3PageTable::new()],
        }) };

        // L2 page table have at most three valid entries
        let l2_entry_nums = pt.l3.len();
        for i in 0..l2_entry_nums {
            let entry = &mut pt.l2.entries[i];
            entry.set(pt.l3[i].as_ptr().as_u64());
            entry.set_bit(RawL2Entry::AF);
            entry.set_value(EntrySh::ISh, RawL2Entry::SH);
            entry.set_value(perm, RawL2Entry::AP);
            // NS
            entry.set_value(EntryAttr::Mem, RawL2Entry::ATTR);
            entry.set_value(EntryType::Table, RawL2Entry::TYPE);
            entry.set_value(EntryValid::Valid, RawL2Entry::VALID);
        }
        pt
    }

    /// Returns the (L2index, L3index) extracted from the given virtual address.
    /// L2index should be smaller than the number of L3PageTable.
    ///
    /// # Panics
    ///
    /// Panics if the virtual address is not properly aligned to page size.
    /// Panics if extracted L2index exceeds the number of L3PageTable.
    fn locate(va: VirtualAddr) -> (usize, usize) {
        use crate::console::kprintln;
        if va.as_ptr().align_offset(PAGE_SIZE) > 0 {
            kprintln!("va: {:x}", va.as_usize());
            panic!("virtual address not aligned to page size");
        }
        let index_l2 = (va.as_usize() & Self::PT_L2_INDEX_MASK) >> Self::PT_L2_INDEX_MASK.trailing_zeros();
        let index_l3 = (va.as_usize() & Self::PT_L3_INDEX_MASK) >> Self::PT_L3_INDEX_MASK.trailing_zeros();
        if index_l2 < 3 {
            (index_l2, index_l3)
        } else {
            panic!("level2 index larger than 2")
        }
    }

    fn get_entry_l3(&self, va: VirtualAddr) -> &L3Entry {
        let (l2index, l3index) = Self::locate(va);
        &self.l3[l2index].entries[l3index]
    }

    fn get_entry_l3_mut(&mut self, va: VirtualAddr) -> &mut L3Entry {
        let (l2index, l3index) = Self::locate(va);
        &mut self.l3[l2index].entries[l3index]
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is valid.
    /// Otherwise, `false` is returned.
    pub fn is_valid(&self, va: VirtualAddr) -> bool {
        self.get_entry_l3(va).is_valid()
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is invalid.
    /// Otherwise, `true` is returned.
    pub fn is_invalid(&self, va: VirtualAddr) -> bool {
        !self.get_entry_l3(va).is_valid()
    }

    /// Set the given RawL3Entry `entry` to the L3Entry indicated by the given virtual
    /// address.
    pub fn set_entry(&mut self, va: VirtualAddr, entry: RawL3Entry) -> &mut Self {
        self.get_entry_l3_mut(va).0 = entry;
        self
    }

    /// Returns a base address of the pagetable. The returned `PhysicalAddr` value
    /// will point the start address of the L2PageTable.
    pub fn get_baddr(&self) -> PhysicalAddr {
        self.l2.as_ptr()
    }

    /// Returns va corresponding physical address.
    pub fn get_phyaddr(&self, va: VirtualAddr) -> PhysicalAddr {
        let l3_entry = self.get_entry_l3((va.as_u64() & (!0xFFFF)).into());
        let phyaddr = l3_entry.0.get_masked(RawL3Entry::ADDR);
        (phyaddr | (va.as_u64() & 0xFFFF)).into()
    }
}

// FIXME: Implement `IntoIterator` for `&PageTable`.
impl<'a> IntoIterator for &'a PageTable {
    type Item = &'a L3Entry;
    type IntoIter = Chain<core::slice::Iter<'a, L3Entry>, core::slice::Iter<'a, L3Entry>>;

    fn into_iter(self) -> Self::IntoIter {
        self.l3[0].entries.iter()
                          .chain(self.l3[1].entries.iter())
    }
}

impl<'a> IntoIterator for &'a mut PageTable {
    type Item = &'a mut L3Entry;
    type IntoIter = Chain<core::slice::IterMut<'a, L3Entry>, core::slice::IterMut<'a, L3Entry>>;

    fn into_iter(self) -> Self::IntoIter {
        let (page_0, page_12) = self.l3.split_at_mut(1);
        let (page_1, _) = page_12.split_at_mut(1);
        page_0[0].entries.iter_mut()
                         .chain(page_1[0].entries.iter_mut())
    }
}

pub struct KernPageTable(Box<PageTable>);

impl KernPageTable {
    /// Returns a new `KernPageTable`. `KernPageTable` should have a `Pagetable`
    /// created with `KERN_RW` permission.
    ///
    /// Set L3entry of ARM physical address starting at 0x00000000 for RAM and
    /// physical address range from `IO_BASE` to `IO_BASE_END` for peripherals.
    /// Each L3 entry should have correct value for lower attributes[10:0] as well
    /// as address[47:16]. Refer to the definition of `RawL3Entry` in `vmsa.rs` for
    /// more details.
    pub fn new() -> KernPageTable {
        let mut kpt = PageTable::new(EntryPerm::KERN_RW);
        let mut addr = 0;
        let (_, end) = allocator::memory_map().unwrap();

        // set entry for ram
        for entry in &mut *kpt {
            if addr + PAGE_SIZE > end {
                break;
            }
            entry.0.set(addr as u64);
            entry.0.set_bit(RawL3Entry::AF);
            entry.0.set_value(EntrySh::ISh, RawL3Entry::SH);
            entry.0.set_value(EntryPerm::KERN_RW, RawL3Entry::AP);
            // NS: don't care
            entry.0.set_value(EntryAttr::Mem, RawL3Entry::ATTR);
            entry.0.set_value(PageType::Page, RawL3Entry::TYPE);
            entry.0.set_value(EntryValid::Valid, RawL3Entry::VALID);
            addr += PAGE_SIZE;
        }

        // set entry for peripherals
        addr = GPU_BASE;
        while addr + PAGE_SIZE <= IO_BASE_END - 0x20000000 {
            // for kernel pagetable, virtual addr and physical addr are the same thing
            let vaddr = addr.into();
            let mut entry = RawL3Entry::new(0);
            entry.set(addr as u64);
            entry.set_bit(RawL3Entry::AF);
            entry.set_value(EntrySh::OSh, RawL3Entry::SH);
            entry.set_value(EntryPerm::KERN_RW, RawL3Entry::AP);
            // NS: don't care
            entry.set_value(EntryAttr::Dev, RawL3Entry::ATTR);
            entry.set_value(PageType::Page, RawL3Entry::TYPE);
            entry.set_value(EntryValid::Valid, RawL3Entry::VALID);
            kpt.set_entry(vaddr, entry);
            addr += PAGE_SIZE;
        }

        KernPageTable(kpt)
    }
}

pub enum PagePerm {
    RW,
    RO,
    RWX,
}

pub struct UserPageTable(Box<PageTable>);

impl UserPageTable {
    /// Returns a new `UserPageTable` containing a `PageTable` created with
    /// `USER_RW` permission.
    pub fn new() -> UserPageTable {
        UserPageTable(PageTable::new(EntryPerm::USER_RW))
    }

    /// Allocates a page and set an L3 entry translates given virtual address to the
    /// physical address of the allocated page. Returns the allocated page.
    ///
    /// # Panics
    /// Panics if the virtual address is lower than `USER_IMG_BASE`.
    /// Panics if the virtual address has already been allocated.
    /// Panics if allocator fails to allocate a page.
    ///
    /// TODO. use Result<T> and make it failurable
    /// TODO. use perm properly
    pub fn alloc(&mut self, va: VirtualAddr, _perm: PagePerm) -> &mut [u8] {
        if va.as_usize() < USER_IMG_BASE {
            panic!("virtual address is lower than USER_IMG_BASE");
        }
        // when alloc page, we need simulate page table walking using software
        // to find corresponding page table entry, therefore the first thing is
        // to subtract top bits, however, since we manually include page table
        // number check (level 3 < 2) in the walking code, since T1SZ = 34
        // we need to extend this subtraction from [63:48]->[63:30] 
        let va = va - USER_IMG_BASE.into();
        if self.is_valid(va) {
            panic!("virtual address has already been allocated");
        }
        // allocate a new page
        let physical_addr = unsafe { ALLOCATOR.alloc(Page::layout()) };
        if physical_addr.is_null() {
            panic!("allocator fails to allocate a page");
        }
        let mut entry = RawL3Entry::new(0);
        entry.set(physical_addr as u64);
        entry.set_bit(RawL3Entry::AF);
        entry.set_value(EntrySh::ISh, RawL3Entry::SH);
        entry.set_value(EntryPerm::USER_RW, RawL3Entry::AP);
        // NS: don't care
        entry.set_value(EntryAttr::Mem, RawL3Entry::ATTR);
        entry.set_value(PageType::Page, RawL3Entry::TYPE);
        entry.set_value(EntryValid::Valid, RawL3Entry::VALID);
        self.set_entry(va, entry);
        // TODO: bad design need refactor
        unsafe { 
            core::slice::from_raw_parts_mut(physical_addr, PAGE_SIZE)
        }
    }

    /// Set pagetable from another user process.
    pub fn from(&mut self, old: &UserPageTable) {
        let mut it = (&mut(*self.0)).into_iter();
        for old_entry in (*old.0).into_iter() {
            let new_entry = it.next().unwrap();
            match old_entry.get_page_addr() {
                Some(page_addr) => {
                    let new_addr = unsafe { 
                        // kprintln!("page fork");
                        let addr = ALLOCATOR.alloc(Page::layout());
                        if addr.is_null() {
                            panic!("allocator fails to allocate a page");
                        }
                        core::ptr::copy_nonoverlapping(page_addr.as_ptr(), addr, PAGE_SIZE);
                        addr as u64
                    };
                    *new_entry = *old_entry;
                    new_entry.0.set_masked(new_addr, RawL3Entry::ADDR);
                },
                None => {},
            }
        }
    }

    pub fn get_kaddr(&self, vaddr: VirtualAddr) -> PhysicalAddr {
        kprintln!("0x{:x}", vaddr.as_u64());
        self.0.get_phyaddr((vaddr - USER_IMG_BASE.into()))
    }
}

impl Deref for KernPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for UserPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KernPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for UserPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// FIXME: Implement `Drop` for `UserPageTable`.
impl Drop for UserPageTable {
    fn drop(&mut self) {
        for entry in self.into_iter() {
            if entry.is_valid() {
                // dealloc page
                use crate::console::kprintln;
                kprintln!("dealloc page table");
                let addr = entry.0.get_masked(RawL3Entry::ADDR) as *mut u8;
                unsafe { 
                    ALLOCATOR.dealloc(addr, Page::layout());
                }
            }
        }
    }
}

// FIXME: Implement `fmt::Debug` as you need.
impl fmt::Debug for KernPageTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tmp = self;
        for entry in tmp.into_iter() {
            use crate::console::kprintln;
            kprintln!("{:#?}", entry);
        }
        f.debug_struct("kernalpagetable")
         .finish()
    }
}

impl fmt::Debug for UserPageTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tmp = self;
        for entry in tmp.into_iter() {
            use crate::console::kprintln;
            kprintln!("{:#?}", entry);
        }
        f.debug_struct("UserPageTable")
         .finish()
    }
}

impl fmt::Debug for L3Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("L3Entry")
         .field("", &self.0)
         .finish()
    }
}
