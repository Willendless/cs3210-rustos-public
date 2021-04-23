use crate::ALLOCATOR;
use core::alloc::{GlobalAlloc, Layout};
use fat32::util::SliceExt;
use pi::mailbox::{Mailbox, MailBoxChannel};
use kernel_api::{OsError, OsResult};

const REQUEST_CODE: u32 = 0x0;
const RESPONSE_SUCCESS: u32 = 0x80000000;
const RESPONSE_FAIL: u32  = 0x80000001;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum TagID {
    End = 0x0,
    FBAllocate = 0x00040001,
    FBRelease = 0x00048001,
    FBBlankScreen = 0x00040002,
    FBGetPhysicalDim = 0x00040003,
    FBSetPhysicalDim = 0x00048003,
    FBGetVirtualDim = 0x00040004,
    FBSetVirtualDim = 0x00048004,
    FBGetDepth = 0x00040005,
    FBSetDepth = 0x00048005,
    FBGetPixelOrder = 0x00040006,
    FBSetPixelOrder = 0x00048006,
    FBGetPitch = 0x00040008,
    FBGetVirtualOffset = 0x00040009,
    FBSetVirtualOffset = 0x00048009,
}

pub enum TagValueBuffer {
    FBAlign(u32, u32),
    FBPhysicalDim(u32, u32),
    FBVirtualDim(u32, u32),
    FBDepth(u32),
    FBPitch(u32),
    FBPixelOrder(u32),
    FBVirtualOffset(u32, u32),
}

impl TagValueBuffer {
    pub fn as_fb_align(&self) -> Option<(u32, u32)> {
        match self {
            TagValueBuffer::FBAlign(ptr, size) => Some((*ptr, *size)),
            _ => None,
        }
    }

    pub fn as_fb_physical_dim(&self) -> Option<(u32, u32)> {
        match self {
            TagValueBuffer::FBPhysicalDim(w, h) => Some((*w, *h)),
            _ => None,
        }
    }

    pub fn as_fb_virtual_dim(&self) -> Option<(u32, u32)> {
        match self {
            TagValueBuffer::FBVirtualDim(w, h) => Some((*w, *h)),
            _ => None,
        }
    }

    pub fn as_fb_depth(&self) -> Option<u32> {
        match self {
            TagValueBuffer::FBDepth(depth) => Some(*depth),
            _ => None,
        }
    }

    pub fn as_fb_pitch(&self) -> Option<u32> {
        match self {
            TagValueBuffer::FBPitch(pitch) => Some(*pitch),
            _ => None,
        }
    }

    pub fn as_fb_pixel_order(&self) -> Option<u32> {
        match self {
            TagValueBuffer::FBPixelOrder(order) => Some(*order),
            _ => None,
        }
    }

    pub fn as_fb_virtual_offset(&self) -> Option<(u32, u32)> {
        match self {
            TagValueBuffer::FBVirtualOffset(x, y) => Some((*x, *y)),
            _ => None,
        }
    }
}

pub struct Tag {
    pub id: TagID,
    pub value_buffer: TagValueBuffer,
}

impl TagID {
    fn value_buf_len(&self) -> usize {
        match *self {
            TagID::End => 0,
            TagID::FBAllocate => 8,
            TagID::FBRelease => 0,
            TagID::FBBlankScreen => 4,
            TagID::FBGetPhysicalDim => 8,
            TagID::FBSetPhysicalDim => 8,
            TagID::FBGetVirtualDim => 8,
            TagID::FBSetVirtualDim => 8,
            TagID::FBGetDepth => 4,
            TagID::FBSetDepth => 4,
            TagID::FBGetPitch => 4,
            TagID::FBGetPixelOrder => 4,
            TagID::FBSetPixelOrder => 4,
            TagID::FBGetVirtualOffset => 8,
            TagID::FBSetVirtualOffset => 8,
        }
    }
}

pub fn send_messages(tags: &mut [Tag]) -> OsResult<()> {
    let mut buf_size = 0;
    // tags size
    for tag in tags.iter() {
        // | tag id | value buffer size | req/rsp code | value buffer | padding |
        // 3: id, size, req/rsp code
        buf_size += tag.id.value_buf_len() + 3 * core::mem::size_of::<u32>();
    }
    // | buffer size | req/rsp code | tags | end tag | padding |
    // 3: size, req/rsp, end tag
    buf_size += 3 * core::mem::size_of::<u32>();
    // align buffer size to 16 byte
    buf_size += if buf_size % 16 == 0 { 0 } else { 16 - (buf_size % 16) };
    let ptr = unsafe { ALLOCATOR.alloc(Layout::from_size_align_unchecked(buf_size, 16)) };
    let buf = unsafe { SliceExt::cast_mut::<u32>(core::slice::from_raw_parts_mut(ptr, buf_size)) };
    // buffer size
    buf[0] = buf_size as u32;
    // req_code
    buf[1] = REQUEST_CODE;
    // construct tags
    let mut i = 2;
    for tag in tags.iter() {
        // id
        buf[i] = tag.id as u32;
        i += 1;
        // value buffer size
        buf[i] = tag.id.value_buf_len() as u32;
        i += 1;
        // req/rsp code
        buf[i] = REQUEST_CODE;
        i += 1;
        // value buffer
        match tag.value_buffer {
            TagValueBuffer::FBAlign(align, _) => {
                buf[i] = align;
                i += 1;
                buf[i] = 0;
                i += 1;
            }
            TagValueBuffer::FBPhysicalDim(width, height ) => {
                buf[i] = width;
                i += 1;
                buf[i] = height;
                i += 1;
            }
            TagValueBuffer::FBVirtualDim(width, height) => {
                buf[i] = width;
                i += 1;
                buf[i] = height;
                i += 1; 
            }
            TagValueBuffer::FBDepth(depth) => {
                buf[i] = depth;
                i += 1;
            }
            TagValueBuffer::FBPixelOrder(porder) => {
                buf[i] = porder; 
                i += 1;
            }
            TagValueBuffer::FBPitch(_) => {
                buf[i] = 0;
                i += 1;
            }
            TagValueBuffer::FBVirtualOffset(x, y) => {
                buf[i] = x;
                i += 1;
                buf[i] = y;
                i += 1;
            }
            _ => unreachable!(),
        }
    }
    // end tag
    buf[i] = 0;
    // send msg and check response code
    let mut mailbox = Mailbox::new();
    mailbox.write(buf.as_ptr() as u32, MailBoxChannel::PropertyArmToVc);
    assert_eq!(mailbox.read(MailBoxChannel::PropertyArmToVc), buf.as_ptr() as u32);

    // check response code
    let ret = match buf[1] {
        REQUEST_CODE => Err(OsError::MailboxError),
        RESPONSE_FAIL => Err(OsError::MailboxFailed),
        RESPONSE_SUCCESS => Ok(()),
        _ => unreachable!()
    };

    // overwrite response msg
    let mut i = 2;
    for tag in tags.iter_mut() {
        use TagValueBuffer::*;
        let len = tag.id.value_buf_len();
        i += 3;
        tag.value_buffer = match tag.value_buffer {
            FBAlign(_, _) => { i += 2; FBAlign(buf[i-2], buf[i-1]) }
            FBPhysicalDim(_, _) => { i += 2; FBPhysicalDim(buf[i-2], buf[i-1]) }
            FBVirtualDim(_, _) => { i += 2; FBVirtualDim(buf[i-2], buf[i-1]) }
            FBDepth(_) => { i += 1; FBDepth(buf[i-1]) }
            FBPitch(_) => { i += 1; FBPitch(buf[i-1]) }
            FBPixelOrder(_) => { i += 1; FBPixelOrder(buf[i-1]) }
            FBVirtualOffset(_, _) => { i += 2; FBVirtualOffset(buf[i-2], buf[i-1]) }
        }
    }

    // free msg buffer 
    unsafe { ALLOCATOR.dealloc(ptr, Layout::from_size_align_unchecked(buf_size, 16)); }
    ret
}
