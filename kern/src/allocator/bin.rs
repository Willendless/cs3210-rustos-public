use core::alloc::Layout;
use core::fmt;
use core::ptr;
use core::mem;
use crate::console::{kprintln, kprint};

use crate::allocator::linked_list::LinkedList;
use crate::allocator::util::*;
use crate::allocator::LocalAlloc;

use crate::allocator::bump;

/// A simple allocator that allocates based on size classes.
///   bin 0 (2^3 bytes)    : handles allocations in (0, 2^3]
///   bin 1 (2^4 bytes)    : handles allocations in (2^3, 2^4]
///   ...
///   bin 29 (2^32 bytes): handles allocations in (2^31, 2^32]
///   
///   map_to_bin(size) -> k
///   
pub struct Allocator {
    // FIXME: Add the necessary fields.
    bins: [LinkedList; 27], // bin 26 (2^29 bytes, 500M): handles allocations in (2^28, 2^29]
    allocated: usize,
    total: usize,
}

impl Allocator {
    /// Creates a new bin allocator that will allocate memory from the region
    /// starting at address `start` and ending at address `end`.
    pub fn new(start: usize, end: usize) -> Allocator {
        let mut bins = [LinkedList::new(); 27];
        let mut bump_allocator = bump::Allocator::new(start, end);
        unsafe {
            for i in (0..27).rev() {
                loop {
                    let size = Allocator::bin_size(i);
                    let layout = Layout::from_size_align(size, size).unwrap();
                    let addr = bump_allocator.alloc(layout);
                    if !addr.is_null() {
                        kprintln!("mem_allocator: assign {:#x}B mem at {:?}", size, addr);
                        bins[i].push(addr as *mut usize);
                    } else {
                        break;
                    }
                }
            }
        }
        let allocator = Allocator {
            bins,
            allocated: 0,
            total: end - start,
        };
        allocator
    }

    /// Return bins index based on size
    fn map_to_bin(layout: Layout) -> usize {
        // bit of first 1 in size counting from 0
        let size = layout.size().max(layout.align());
        let nbit = (mem::size_of::<usize>()) * 8 - size.leading_zeros() as usize - 1;
        if size.is_power_of_two() {
            nbit.saturating_sub(3)
        } else {
            (nbit + 1).saturating_sub(3)
        }
    }

    /// Return size based on class index
    fn bin_size(index: usize) -> usize {
        1 << (index + 3)
    }
}

impl LocalAlloc for Allocator {
    /// Allocates memory. Returns a pointer meeting the size and alignment
    /// properties of `layout.size()` and `layout.align()`.
    ///
    /// If this method returns an `Ok(addr)`, `addr` will be non-null address
    /// pointing to a block of storage suitable for holding an instance of
    /// `layout`. In particular, the block will be at least `layout.size()`
    /// bytes large and will be aligned to `layout.align()`. The returned block
    /// of storage may or may not have its contents initialized or zeroed.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure that `layout.size() > 0` and that
    /// `layout.align()` is a power of two. Parameters not meeting these
    /// conditions may result in undefined behavior.
    ///
    /// # Errors
    ///
    /// Returning null pointer (`core::ptr::null_mut`)
    /// indicates that either memory is exhausted
    /// or `layout` does not meet this allocator's
    /// size or alignment constraints.
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 || !layout.align().is_power_of_two() {
                return ptr::null_mut();
        }
        let nth = Allocator::map_to_bin(layout);
        // Iterate list to find matched block
        for (ith, list) in self.bins[nth..].iter_mut().enumerate() {
            if !list.is_empty() {
                // Half cut mem each time
                let addr = list.pop().unwrap();
                for off in (0..ith).rev() {
                    let len = Allocator::bin_size(nth + off);
                    self.bins[nth + off].push((addr as usize + len) as *mut usize);
                }
                self.allocated += Allocator::bin_size(nth);
                return addr as *mut u8;
            }
        }
        // unable to allocate mem
        kprintln!("alloc: failed to allocate mem");
        ptr::null_mut()
    }

    /// Deallocates the memory referenced by `ptr`.
    ///
    /// # Safety
    ///
    /// The _caller_ must ensure the following:
    ///
    ///   * `ptr` must denote a block of memory currently allocated via this
    ///     allocator
    ///   * `layout` must properly represent the original layout used in the
    ///     allocation call that returned `ptr`
    ///
    /// Parameters not meeting these conditions may result in undefined
    /// behavior.
    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        // check addr is power of 2 and insert into list
        if layout.align().is_power_of_two() {
            // ptr can be inserted into the nth list
            let mut ptr = ptr as *mut usize;
            let mut nth = Allocator::map_to_bin(layout);
            loop {
                // find which list to insert
                let cur_class = nth;
                let buddy = ptr as usize ^ Allocator::bin_size(nth);
                for node in self.bins[nth].iter_mut() {
                    // if able to merge, upgrade one level
                    if node.value() as usize == buddy {
                        node.pop();
                        nth += 1;
                        break;
                    }
                }
                if cur_class != nth {
                    ptr = buddy.min(ptr as usize) as *mut usize;
                } else {
                    break;
                }
            }
            self.bins[nth].push(ptr);
            self.allocated -= Allocator::bin_size(Allocator::map_to_bin(layout));
        }
    }
}

// FIXME: Implement `Debug` for `Allocator`.
impl fmt::Debug for Allocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BinAllocator")
         .field("allocated", &self.allocated)
         .field("total", &self.total)
         .finish()
    }
}
