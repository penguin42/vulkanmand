// based on the trival.rs example from the ocl crate

extern crate ocl;

const RENDER_CONFIG_SIZE : usize =  15;

pub struct Bulbocl {
    context: ocl::Context,
    queue: ocl::Queue,
    renderkern: ocl::Kernel,
    mandkern: ocl::Kernel,

    voxelsize: usize, // typically 256 for 256x256x256
    voxelbuf: ocl::Buffer<u8>,

    imagewidth: usize,
    imageheight: usize,
    imagebuf: ocl::Buffer<u8>,
    imageconfigbuf: ocl::Buffer<f32>,
    imagedebugbuf: ocl::Buffer<f32>
}

impl Bulbocl {
    pub fn new() -> Bulbocl {
        let platform = ocl::Platform::default();
        let device = ocl::Device::first(platform);   /* TODO: Should be smarter with selecting GPU */
        let context = ocl::Context::builder().platform(platform).devices(device.clone()).build().unwrap();
        let queue = ocl::Queue::new(&context, device, None).unwrap();

        let voxelsize = 4; // Dummy initial dimension
        let voxelbuf = ocl::Buffer::<u8>::builder().queue(queue.clone()).flags(ocl::flags::MEM_READ_WRITE).dims((4,4,4)).build().unwrap();

        let imagewidth = 4; // Dummy initial dimension
        let imageheight = 4; // Dummy initial dimension
        let imagebuf = ocl::Buffer::<u8>::builder().queue(queue.clone()).flags(ocl::flags::MEM_READ_WRITE).dims((4,4)).build().unwrap();
        let imageconfigbuf = ocl::Buffer::<f32>::builder().queue(queue.clone()).flags(ocl::flags::MEM_READ_WRITE).dims(RENDER_CONFIG_SIZE).build().unwrap();
        let imagedebugbuf = ocl::Buffer::<f32>::builder().queue(queue.clone()).flags(ocl::flags::MEM_READ_WRITE).dims((4,4)).build().unwrap();

        let mandprog = ocl::Program::builder().devices(device).src_file("mandel.ocl").build(&context).unwrap();
        let mandkern = ocl::Kernel::new("mandel", &mandprog).unwrap().arg_buf_named("voxels", None::<ocl::Buffer<u8>>).queue(queue.clone());
        let renderprog = ocl::Program::builder().devices(device).src_file("ray.ocl").build(&context).unwrap();
        let renderkern = ocl::Kernel::new("ray", &renderprog).unwrap().
                             arg_buf_named("image", None::<ocl::Buffer<u8>>).
                             arg_buf_named("voxels", None::<ocl::Buffer<u8>>).
                             arg_buf_named("config", None::<ocl::Buffer<f32>>).
                             arg_buf_named("debug", None::<ocl::Buffer<f32>>).
                             queue(queue.clone());

        Bulbocl { context: context, queue: queue, mandkern: mandkern, renderkern: renderkern,
                  imagewidth: imagewidth, imageheight: imageheight,
                  imagebuf: imagebuf, imageconfigbuf: imageconfigbuf, imagedebugbuf: imagedebugbuf,
                  voxelsize: voxelsize, voxelbuf: voxelbuf }
    }

    pub fn calc_bulb(&mut self, size: usize) {
        if self.voxelsize != size {
            // Need to resize the buffer
            // TODO: wait for the queue to empty
            self.mandkern.set_arg_buf_named("voxels", None::<ocl::Buffer<u8>>).unwrap();
            self.renderkern.set_arg_buf_named("voxels", None::<ocl::Buffer<u8>>).unwrap();
            self.voxelsize = size;
            self.voxelbuf = ocl::Buffer::<u8>::builder().queue(self.queue.clone()).flags(ocl::flags::MEM_READ_WRITE).dims((size,size,size)).build().unwrap();
        }
        self.mandkern.set_arg_buf_named("voxels", Some(&self.voxelbuf)).unwrap();

        unsafe {
            self.mandkern.cmd().gwo((0,0,0)).gws((size,size,size)).enq().unwrap();
        }
    }

    pub fn render_image(&mut self, width: usize, height: usize) {
        if self.imagewidth != width || self.imageheight != height {
            // Need to resize the buffer
            // TODO: wait for the queue to empty
            self.renderkern.set_arg_buf_named("image", None::<ocl::Buffer<u8>>).unwrap();
            self.renderkern.set_arg_buf_named("debug", None::<ocl::Buffer<f32>>).unwrap();
            self.imagewidth = width;
            self.imageheight = height;
            self.imagebuf = ocl::Buffer::<u8>::builder().queue(self.queue.clone()).flags(ocl::flags::MEM_WRITE_ONLY).dims((width, height)).build().unwrap();
            self.imagedebugbuf = ocl::Buffer::<f32>::builder().queue(self.queue.clone()).flags(ocl::flags::MEM_WRITE_ONLY).dims((width, height)).build().unwrap();
        }
        // TODO: Set data in config buffer
        self.renderkern.set_arg_buf_named("voxels", Some(&self.voxelbuf)).unwrap();
        self.renderkern.set_arg_buf_named("image", Some(&self.imagebuf)).unwrap();
        self.renderkern.set_arg_buf_named("config", Some(&self.imageconfigbuf)).unwrap();
        self.renderkern.set_arg_buf_named("debug", Some(&self.imagedebugbuf)).unwrap();
        // TODO: Queue wait for the voxels
        unsafe {
            self.mandkern.cmd().gwo((0,0)).gws((width, height)).enq().unwrap();
        }
        // TODO: Queue wait for the image
        // TODO: Copy the image out
    }
}

