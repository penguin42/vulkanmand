// based on the trival.rs example from the ocl crate

extern crate ocl;

pub struct Bulbocl {
    context: ocl::Context,
    queue: ocl::Queue,
    renderkern: ocl::Kernel,
    mandkern: ocl::Kernel,

    voxelsize: usize, // typically 256 for 256x256x256
    voxelbuf: ocl::Buffer<u8>
}

impl Bulbocl {
    pub fn new() -> Bulbocl {
        let platform = ocl::Platform::default();
        let device = ocl::Device::first(platform);   /* TODO: Should be smarter with selecting GPU */
        let context = ocl::Context::builder().platform(platform).devices(device.clone()).build().unwrap();
        let queue = ocl::Queue::new(&context, device, None).unwrap();

        let voxelsize = 4; // Dummy initial dimension
        let voxelbuf = ocl::Buffer::<u8>::builder().queue(queue.clone()).flags(ocl::flags::MEM_READ_WRITE).dims((4,4,4)).build().unwrap();
        let mandprog = ocl::Program::builder().devices(device).src_file("mandel.ocl").build(&context).unwrap();
        let mandkern = ocl::Kernel::new("mandel", &mandprog).unwrap().arg_buf_named("voxels", None::<ocl::Buffer<u8>>).queue(queue.clone());
        let renderprog = ocl::Program::builder().devices(device).src_file("ray.ocl").build(&context).unwrap();
        let renderkern = ocl::Kernel::new("ray", &renderprog).unwrap().queue(queue.clone());

        Bulbocl { context: context, queue: queue, mandkern: mandkern, renderkern: renderkern,
                  voxelsize: voxelsize, voxelbuf: voxelbuf }
    }

    pub fn calc_bulb(&mut self, size: usize) {
        if self.voxelsize != size {
            // Need to resize the buffer
            self.voxelsize = size;
            self.voxelbuf = ocl::Buffer::<u8>::builder().queue(self.queue.clone()).flags(ocl::flags::MEM_READ_WRITE).dims((size,size,size)).build().unwrap();
        }
        self.mandkern.set_arg_buf_named("voxels", Some(&self.voxelbuf)).unwrap();

        unsafe {
            self.mandkern.cmd().gwo((0,0,0)).gws((size,size,size)).enq().unwrap();
        }
    }
}

