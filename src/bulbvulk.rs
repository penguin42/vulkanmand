// based on the trival.rs example from the ocl crate

extern crate vulkano;
extern crate nalgebra as na;
extern crate bincode;
use std::ffi::CStr;
use std::fs::File;
use std::io::*;
use std::sync::Arc;
use self::vulkano::buffer;
use self::vulkano::buffer::BufferAccess;
use self::vulkano::command_buffer;
use self::vulkano::descriptor::descriptor;
use self::vulkano::descriptor::descriptor_set;
use self::vulkano::descriptor::pipeline_layout;
use self::vulkano::device;
use self::vulkano::format;
use self::vulkano::image;
use self::vulkano::image::ImageAccess;
use self::vulkano::instance;
use self::vulkano::pipeline::shader;
use self::vulkano::pipeline::ComputePipeline;
use self::vulkano::sync;
use self::vulkano::sync::GpuFuture;

// I'd like these to be part of Bulbvulk, but that
// gets passed by references in device handlers, but I don't
// really want to force it to be static

lazy_static! {
    static ref VINSTANCE: Arc<instance::Instance> = {
        let vitmp = instance::Instance::new(None,
                                       &instance::InstanceExtensions::none(),
                                       None).unwrap();
        vitmp
    };
    static ref VPHYSDEVICE: instance::PhysicalDevice<'static> = {
        let vpdev = instance::PhysicalDevice::enumerate(&VINSTANCE).next().unwrap();

        vpdev
    };
}

#[derive(Debug, Copy, Clone)]
struct MandLayout(descriptor::ShaderStages);
unsafe impl pipeline_layout::PipelineLayoutDesc for MandLayout {
        // We just have 'voxels' which is binding 0 in set 0
        fn num_sets(&self) -> usize { 1 }
        fn num_bindings_in_set(&self, set: usize) -> Option<usize> {
            match set {
                0 => Some(1),
                _ => None,
            }
        }
        fn descriptor(&self, set: usize, binding: usize) -> Option<descriptor::DescriptorDesc> {
            match (set, binding) {
                (0,0) => Some(descriptor::DescriptorDesc {
                      array_count: 1,
                      stages: descriptor::ShaderStages { compute: true, ..descriptor::ShaderStages::none() },
                      readonly: false,
                      ty: descriptor::DescriptorDescTy::Buffer(descriptor::DescriptorBufferDesc {
                          dynamic: Some(false),  // ?
                          storage: true, // ?
                        }),
                    }),
                _ => None,
            }
        }

        // We have one push constant (power)
        fn num_push_constants_ranges(&self) -> usize { 1 }
        fn push_constants_range(&self, num: usize) -> Option<pipeline_layout::PipelineLayoutDescPcRange> {
            if num != 0 { return None; }
            Some(pipeline_layout::PipelineLayoutDescPcRange { offset: 0,
                                             size: 4,
                                             stages: descriptor::ShaderStages::all() })
        }

}

#[derive(Debug, Copy, Clone)]
struct RayLayout(descriptor::ShaderStages);
unsafe impl pipeline_layout::PipelineLayoutDesc for RayLayout {
        // Voxels, output image, hmm and then push constants - are they, for now I say no
        fn num_sets(&self) -> usize { 1 }
        fn num_bindings_in_set(&self, set: usize) -> Option<usize> {
            match set {
                0 => Some(2),
                _ => None,
            }
        }
        fn descriptor(&self, set: usize, binding: usize) -> Option<descriptor::DescriptorDesc> {
            match (set, binding) {
                // The voxels
                (0,0) => Some(descriptor::DescriptorDesc {
                      array_count: 1,
                      stages: descriptor::ShaderStages { compute: true, ..descriptor::ShaderStages::none() },
                      readonly: true,
                      ty: descriptor::DescriptorDescTy::Buffer(descriptor::DescriptorBufferDesc {
                          dynamic: Some(false),  // ?
                          storage: true, // ?
                        }),
                    }),
                // The output image
                (0,1) => Some(descriptor::DescriptorDesc {
                      array_count: 1,
                      stages: descriptor::ShaderStages { compute: true, ..descriptor::ShaderStages::none() },
                      readonly: false,
                      ty: descriptor::DescriptorDescTy::Image(descriptor::DescriptorImageDesc {
                           sampled: false,
                           multisampled: false,
                           dimensions: descriptor::DescriptorImageDescDimensions::TwoDimensional,
                           array_layers: descriptor::DescriptorImageDescArray::NonArrayed,
                           format: Some(format::Format::B8G8R8A8Uint), // ?
                        }),
                    }),
                _ => None,
            }
        }

        // We've got one push constant layout
        //   Are we supposed to do this as one or as 6 separate vectors??
        fn num_push_constants_ranges(&self) -> usize { 6 * 16 }
        fn push_constants_range(&self, num: usize) -> Option<pipeline_layout::PipelineLayoutDescPcRange> {
            if num != 0 { return None; }
            Some(pipeline_layout::PipelineLayoutDescPcRange { offset: 0,
                                             size: 6 * 16,
                                             stages: descriptor::ShaderStages::all() })
        }

}

pub struct Bulbvulk {
    voxelsize: usize, // typically 256 for 256x256x256

    imagewidth: usize,
    imageheight: usize,

    vdevice: Arc<device::Device>,
    vqueue: Arc<device::Queue>,

    voxelbuf: Arc<buffer::device_local::DeviceLocalBuffer<[u32]>>,
    rayimg: Arc<image::StorageImage<format::B8G8R8A8Uint>>,

    mandpipe: Arc<ComputePipeline<pipeline_layout::PipelineLayout<MandLayout>>>,
    raypipe: Arc<ComputePipeline<pipeline_layout::PipelineLayout<RayLayout>>>,
}

impl Bulbvulk {
    pub fn new() -> Bulbvulk {
        let voxelsize = 4; // Dummy initial dimension

        let imagewidth : usize = 4; // Dummy initial dimension
        let imageheight : usize = 4; // Dummy initial dimension
        let qf = VPHYSDEVICE.queue_families().filter(|q| q.supports_compute() && q.supports_transfers()).next().unwrap();

        let (vdevice, mut vqueueiter) = device::Device::new(*VPHYSDEVICE, &instance::Features::none(), &instance::DeviceExtensions::none(), Some((qf, 1.0))).unwrap();
        // Only using one queue
        let vqueue = vqueueiter.next().unwrap();

        // I want to use an Image, but I can't figure out which image to use here
        let voxelbuf = buffer::device_local::DeviceLocalBuffer::<[u32]>::array(vdevice.clone(), voxelsize*voxelsize*voxelsize,
                                                                              buffer::BufferUsage::all(), vdevice.active_queue_families()).unwrap();

        let mandcs = {
            let mut f = File::open("mandel.spv").unwrap();
            let mut v = vec![];
            f.read_to_end(&mut v).unwrap();
            unsafe { shader::ShaderModule::new(vdevice.clone(), &v) }.unwrap()
        };

        let rayimg = image::StorageImage::with_usage(vdevice.clone(),
                                                     image::Dimensions::Dim2d { width: imagewidth as u32, height: imageheight as u32},
                                                     format::B8G8R8A8Uint,
                                                     image::ImageUsage { storage: true, transfer_source: true,
                                                                         ..image::ImageUsage::none()},
                                                     vdevice.active_queue_families()).unwrap();

        let raycs = {
            let mut f = File::open("ray.spv").unwrap();
            let mut v = vec![];
            f.read_to_end(&mut v).unwrap();
            unsafe { shader::ShaderModule::new(vdevice.clone(), &v) }.unwrap()
        };
        let mandpipe = Arc::new(unsafe {
            ComputePipeline::new(vdevice.clone(),
                                 &mandcs.compute_entry_point(CStr::from_bytes_with_nul_unchecked(b"main\0"),
                                                             MandLayout(descriptor::ShaderStages { compute: true, ..descriptor::ShaderStages::none() })
                                                            ),
                                 &()).unwrap()
        });
        let raypipe = Arc::new(unsafe {
            ComputePipeline::new(vdevice.clone(),
                                 &raycs.compute_entry_point(CStr::from_bytes_with_nul_unchecked(b"main\0"),
                                                             RayLayout(descriptor::ShaderStages { compute: true, ..descriptor::ShaderStages::none() })
                                                            ),
                                 &()).unwrap()
        });
        println!("Vulkan device: {}", VPHYSDEVICE.name());
        Bulbvulk { imagewidth, imageheight, voxelsize,
                   vdevice, vqueue, voxelbuf, rayimg,
                   mandpipe, raypipe }
    }

    pub fn calc_bulb(&mut self, size: usize, power: f32) {
        if self.voxelsize != size {
            // Need to resize the buffer
            self.voxelsize = size;
            self.voxelbuf = buffer::device_local::DeviceLocalBuffer::<[u32]>::array(self.vdevice.clone(), self.voxelsize*self.voxelsize*self.voxelsize,
                                                                              buffer::BufferUsage::all(), self.vdevice.active_queue_families()).unwrap();
        }
        // Do I really want persistent - this is transitory
        let set = Arc::new(descriptor_set::PersistentDescriptorSet::start(self.mandpipe.clone(), 0)
                  .add_buffer(self.voxelbuf.clone()).unwrap()
                  .build().unwrap());
        let vsize32 = self.voxelsize as u32;
        let combuf = command_buffer::AutoCommandBufferBuilder::primary_one_time_submit(self.vdevice.clone(), self.vqueue.family()).unwrap()
                     .dispatch([vsize32, vsize32, vsize32],
                               self.mandpipe.clone(), set.clone(), power).unwrap()
                     .build().unwrap();
        // Engage!
        let future = sync::now(self.vdevice.clone())
                     .then_execute(self.vqueue.clone(), combuf).unwrap()
                     .then_signal_fence_and_flush().unwrap();
        // Wait for it
        future.wait(None).unwrap();
    }

    pub fn render_image(&mut self, result: &mut [u8],
                        width: usize, height: usize,
                        eye: na::Vector3<f32>,
                        vp_mid: na::Vector3<f32>,
                        vp_right: na::Vector3<f32>,
                        vp_down: na::Vector3<f32>,
                        light: na::Vector3<f32>
                        ) {
        #[repr(C)]
        // This MUST match the push_constant binding in the GLSL
        struct PushConstants {
           eyex: f32,
           eyey: f32,
           eyez: f32,
           eyegap: f32,

           vpmidx: f32,
           vpmidy: f32,
           vpmidz: f32,
           vpmidgap: f32,

           vprightx: f32,
           vprighty: f32,
           vprightz: f32,
           vprightgap: f32,

           vpdownx: f32,
           vpdowny: f32,
           vpdownz: f32,
           vpdowngap: f32,

           lightx: f32,
           lighty: f32,
           lightz: f32,
           lightgap: f32,

           voxelsizex: f32,
           voxelsizey: f32,
           voxelsizez: f32,
           voxelsizegap: f32,
        };
        let seye = eye * self.voxelsize as f32;
        let svp_mid = vp_mid * self.voxelsize as f32;
        let svp_right = vp_right * self.voxelsize as f32;
        let svp_down = vp_down * self.voxelsize as f32;
        let slight = light * self.voxelsize as f32;
        let pc = PushConstants { eyex: seye.x, eyey: seye.y, eyez: seye.z, eyegap: -1.0,
                                 vpmidx: svp_mid.x, vpmidy: svp_mid.y, vpmidz: svp_mid.z, vpmidgap: -1.0,
                                 vprightx: svp_right.x, vprighty: svp_right.y, vprightz: svp_right.z, vprightgap: -1.0,
                                 vpdownx: svp_down.x, vpdowny: svp_down.y, vpdownz: svp_down.z, vpdowngap: -1.0,
                                 lightx: slight.x, lighty: slight.y, lightz: slight.z, lightgap: -1.0,
                                 voxelsizex: self.voxelsize as f32, voxelsizey: self.voxelsize as f32, voxelsizez: self.voxelsize as f32, voxelsizegap: -1.0,
                               };

        if self.imagewidth != width || self.imageheight != height {
            // Need to resize the buffer
            self.imagewidth = width;
            self.imageheight = height;
            self.rayimg = image::StorageImage::with_usage(self.vdevice.clone(),
                                                     image::Dimensions::Dim2d { width: self.imagewidth as u32, height: self.imageheight as u32},
                                                     format::B8G8R8A8Uint,
                                                     image::ImageUsage { storage: true, transfer_source: true,
                                                                         ..image::ImageUsage::none()},
                                                     self.vdevice.active_queue_families()).unwrap();
        }
        // Do I really want persistent - this is transitory
        let set = Arc::new(descriptor_set::PersistentDescriptorSet::start(self.raypipe.clone(), 0)
                  .add_buffer(self.voxelbuf.clone()).unwrap()
                  .add_image(self.rayimg.clone()).unwrap()
                  .build().unwrap());
        let combuf = command_buffer::AutoCommandBufferBuilder::primary_one_time_submit(self.vdevice.clone(), self.vqueue.family()).unwrap()
                     .dispatch([self.imagewidth as u32, self.imageheight as u32, 1],
                               self.raypipe.clone(), set.clone(), pc).unwrap()
                     .build().unwrap();
        // Engage!
        let future = sync::now(self.vdevice.clone())
                     .then_execute(self.vqueue.clone(), combuf).unwrap()
                     .then_signal_fence_and_flush().unwrap();
        // Wait for it
        future.wait(None).unwrap();

        // copy it to result - there has to be a better way of doing this!
        let cpubuf = unsafe { buffer::cpu_access::CpuAccessibleBuffer::<[u8]>::uninitialized_array(self.vdevice.clone(),
                                                                                          4*self.imagewidth*self.imageheight,
                                                                                          vulkano::buffer::BufferUsage::all()).unwrap() };
        let combuf2 = command_buffer::AutoCommandBufferBuilder::primary_one_time_submit(self.vdevice.clone(), self.vqueue.family()).unwrap()
                     .copy_image_to_buffer(self.rayimg.clone(), cpubuf.clone()).unwrap()
                     .build().unwrap();
        // Engage!
        let future2 = sync::now(self.vdevice.clone())
                     .then_execute(self.vqueue.clone(), combuf2).unwrap()
                     .then_signal_fence_and_flush().unwrap();
        // Wait for it
        future2.wait(None).unwrap();
        let cpubufread = cpubuf.read().unwrap();
        result.copy_from_slice(&cpubufread.to_owned());
    }

    pub fn save_voxels(&mut self) {
        // We can't read directly from the voxel buffer since it's DeviceLocal, so
        // we copy it into a temporary CPU buffer
        // I'd like to use a CpuBufferPool here but there doesn't seem to be a way to do array
        // allocations
        let cpubuf = unsafe { buffer::cpu_access::CpuAccessibleBuffer::<[u32]>::uninitialized_array(self.vdevice.clone(),
                                                                                          self.voxelsize*self.voxelsize*self.voxelsize,
                                                                                          vulkano::buffer::BufferUsage::all()).unwrap() };

        let combuf = command_buffer::AutoCommandBufferBuilder::primary_one_time_submit(self.vdevice.clone(), self.vqueue.family()).unwrap()
                       .copy_buffer(self.voxelbuf.clone(), cpubuf.clone()).unwrap()
                       .build().unwrap();
        let future = sync::now(self.vdevice.clone())
                     .then_execute(self.vqueue.clone(), combuf).unwrap()
                     .then_signal_fence_and_flush().unwrap();
        future.wait(None).unwrap();

        let cpubufread = cpubuf.read().unwrap();
        let mut file = File::create("voxels.dat").unwrap();
        bincode::serialize_into(&mut file, &cpubufread.to_owned()).unwrap();
    }
}

