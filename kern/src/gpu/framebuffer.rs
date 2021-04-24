use crate::gpu::msg;
use crate::gpu::msg::{Tag, TagID, TagValueBuffer};
use crate::gpu::Pixel;
use crate::mutex::Mutex;
use crate::console::{kprintln};

pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 320;
const BUFFER_SIZE: usize = (WIDTH * HEIGHT) as usize;

pub struct GlobalFrameBuffer(Mutex<Option<FrameBuffer>>);

impl GlobalFrameBuffer {
    pub const fn uninitialized() -> GlobalFrameBuffer {
        GlobalFrameBuffer(Mutex::new(None))
    }
    
    pub fn is_initialized(&self) -> bool {
        self.0.lock().is_some()
    }

    pub fn critical<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut FrameBuffer) -> R,
    {
        let mut guart = self.0.lock();
        f(guart.as_mut().expect("framebuffer uninitialized"))
    }

    pub fn initialize(&self) {
        let fb = FrameBuffer::new();
        match fb {
            Some(buf) => {
                *self.0.lock() = Some(buf);
                self.critical(|buf| {
                    info!("framebuffer: init");
                    info!("framebuffer: addr: 0x{:x}, size: {}",
                        buf.buffer.as_ptr() as usize, buf.size);
                    info!("framebuffer: width: {}, height: {}, vwidth: {}, vheight: {}, voffset_x: {}, voffset_y: {}, ",
                        buf.width, buf.height,
                        buf.vwidth, buf.vheight,
                        buf.voffset_x, buf.voffset_y,
                    );
                    info!("framebuffer: depth: {}, pitch: {}, porder: {}", buf.depth, buf.pitch, buf.porder);
                    info!("framebuffer: init succeed");
                });
            }
            None => info!("frambuffer: failed")
        }
    }

    pub fn write_pixel(&self, x: u32, y: u32, pixel: Pixel) {
        self.critical(|fb| {
            let pos = (y * fb.pitch + x * fb.depth / 8) as usize;
            fb.buffer[pos] = pixel.blue;
            fb.buffer[pos + 1] = pixel.green;
            fb.buffer[pos + 2] = pixel.red;
        })
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Pixel {
        self.critical(|fb| {
            let pos = (y * fb.pitch + x * fb.depth / 8) as usize;
            Pixel {
                blue: fb.buffer[pos],
                green: fb.buffer[pos + 1],
                red: fb.buffer[pos + 2],
            }
        })
    }

    pub fn set_voffset_x(&self, x: u32) {
        self.critical(|fb| {
            fb.voffset_x = x;
        })
    }

    pub fn set_voffset_y(&self, y: u32) {
        self.critical(|fb| {
            fb.voffset_y = y;
        })
    }

    pub fn get_voffset_x(&self) -> u32 {
        self.critical(|fb| {
            fb.voffset_x
        })
    }

    pub fn get_voffset_y(&self) -> u32 {
        self.critical(|fb| {
            fb.voffset_y
        })
    }

    pub fn print_fb(&self) {
        self.critical(|fb| {
            kprintln!("width: {}, height: {}, vwidth: {}, vheight: {}", fb.width, fb.height, fb.vwidth, fb.vheight);
            kprintln!("depth: {}, pitch: {}, size: {}", fb.depth, fb.pitch, fb.size);
        })
    }

}

pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub vwidth: u32,
    pub vheight: u32,
    pub voffset_x: u32,
    pub voffset_y: u32,
    pub depth: u32,
    pub pitch: u32,
    pub porder: u32,
    pub buffer: &'static mut [u8],
    pub size: u32,
}

impl FrameBuffer {
    pub fn new() -> Option<FrameBuffer> {
        let mut tags: [Tag; 7] = [
            // 0: set physical dim
            Tag {
                id: TagID::FBSetPhysicalDim,
                value_buffer: TagValueBuffer::FBPhysicalDim(WIDTH, HEIGHT)
            },
            // 1: set virtual dim
            Tag {
                id: TagID::FBSetVirtualDim,
                value_buffer: TagValueBuffer::FBVirtualDim(WIDTH, HEIGHT)
            },
            // 2: set depth
            Tag {
                id: TagID::FBSetDepth,
                value_buffer: TagValueBuffer::FBDepth(24),
            },
            // 3: set virtual offset to 0, 0
            Tag {
                id: TagID::FBSetVirtualOffset,
                value_buffer: TagValueBuffer::FBVirtualOffset(0, 0),
            },
            // 4: get pitch
            Tag {
                id: TagID::FBGetPitch,
                value_buffer: TagValueBuffer::FBPitch(0),
            },
            // 5: allocate frame buffer
            Tag {
                id: TagID::FBAllocate,
                value_buffer: TagValueBuffer::FBAlign(16, 0),
            },
            // 6: set pixel order to RGB
            Tag {
                id: TagID::FBSetPixelOrder,
                value_buffer: TagValueBuffer::FBPixelOrder(1),
            },
        ];
        match msg::send_messages(&mut tags[..]) {
            Ok(_) => {}
            Err(_) => unreachable!()
        }
        let (width, height) = tags[0].value_buffer.as_fb_physical_dim().unwrap();
        let (vwidth, vheight) = tags[1].value_buffer.as_fb_virtual_dim().unwrap();
        let depth = tags[2].value_buffer.as_fb_depth().unwrap();
        let (voffset_x, voffset_y) = tags[3].value_buffer.as_fb_virtual_offset().unwrap();
        let pitch = tags[4].value_buffer.as_fb_pitch().unwrap();
        let (mut buffer, size) = tags[5].value_buffer.as_fb_align().unwrap();
        let porder = tags[6].value_buffer.as_fb_pixel_order().unwrap();
        buffer &= 0x3FFF_FFFF;
        Some(FrameBuffer {
            width,
            height,
            vwidth,
            vheight,
            voffset_x,
            voffset_y,
            depth,
            pitch,
            porder,
            buffer: unsafe { core::slice::from_raw_parts_mut(buffer as *mut u8, size as usize) },
            size,
        })
    }
}
