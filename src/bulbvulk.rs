// based on the trival.rs example from the ocl crate

extern crate vulkano;
extern crate nalgebra as na;
extern crate bincode;
use std::ffi::CStr;
use std::fs::File;
use std::io::*;
use std::sync::Arc;
use self::vulkano::instance;
use self::vulkano::device;
use self::vulkano::buffer;
use self::vulkano::descriptor::descriptor;
use self::vulkano::descriptor::pipeline_layout;
use self::vulkano::pipeline::shader;
use self::vulkano::pipeline::ComputePipeline;

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

        // We have no push constants
        fn num_push_constants_ranges(&self) -> usize { 0 }
        fn push_constants_range(&self, num: usize) -> Option<pipeline_layout::PipelineLayoutDescPcRange> {
            if num != 0 || 0 == 0 { return None; }
            Some(pipeline_layout::PipelineLayoutDescPcRange { offset: 0,
                                             size: 0,
                                             stages: descriptor::ShaderStages::all() })
        }

}

pub struct Bulbvulk {
    voxelsize: usize, // typically 256 for 256x256x256

    imagewidth: usize,
    imageheight: usize,

    vdevice: Arc<device::Device>,
    vqueue: Arc<device::Queue>,

    voxelbuf: Arc<buffer::device_local::DeviceLocalBuffer<[u8]>>,

    mandcs: Arc<shader::ShaderModule>,
    mandpipe: ComputePipeline<pipeline_layout::PipelineLayout<MandLayout>>,
}

impl Bulbvulk {
    pub fn new() -> Bulbvulk {
        let voxelsize = 4; // Dummy initial dimension

        let imagewidth = 4; // Dummy initial dimension
        let imageheight = 4; // Dummy initial dimension
        let qf = VPHYSDEVICE.queue_families().filter(|q| q.supports_compute() && q.supports_transfers()).next().unwrap();

        let (vdevice, mut vqueueiter) = device::Device::new(*VPHYSDEVICE, &instance::Features::none(), &instance::DeviceExtensions::none(), Some((qf, 1.0))).unwrap();
        // Only using one queue
        let vqueue = vqueueiter.next().unwrap();

        // I want to use an Image, but I can't figure out which image to use here
        let voxelbuf = buffer::device_local::DeviceLocalBuffer::<[u8]>::array(vdevice.clone(), voxelsize*voxelsize*voxelsize,
                                                                              buffer::BufferUsage::all(), vdevice.active_queue_families()).unwrap();

        let mandcs = {
            let mut f = File::open("mandel.spv").unwrap();
            let mut v = vec![];
            f.read_to_end(&mut v).unwrap();
            unsafe { shader::ShaderModule::new(vdevice.clone(), &v) }.unwrap()
        };
        let mandpipe = unsafe { 
            ComputePipeline::new(vdevice.clone(),
                                 &mandcs.compute_entry_point(CStr::from_bytes_with_nul_unchecked(b"main\0"),
                                                             MandLayout(descriptor::ShaderStages { compute: true, ..descriptor::ShaderStages::none() })
                                                            ),
                                 &()).unwrap()
        };
        println!("Vulkan device: {}", VPHYSDEVICE.name());
        Bulbvulk { imagewidth, imageheight, voxelsize,
                   vdevice, vqueue, voxelbuf,
                   mandcs, mandpipe }
    }

    pub fn calc_bulb(&mut self, size: usize, power: f32) {
        if self.voxelsize != size {
            // Need to resize the buffer
            self.voxelsize = size;
            self.voxelbuf = buffer::device_local::DeviceLocalBuffer::<[u8]>::array(self.vdevice.clone(), self.voxelsize*self.voxelsize*self.voxelsize,
                                                                              buffer::BufferUsage::all(), self.vdevice.active_queue_families()).unwrap();
        }

    }

    pub fn render_image(&mut self, result: &mut [u8],
                        width: usize, height: usize,
                        eye: na::Vector3<f32>,
                        vp_mid: na::Vector3<f32>,
                        vp_right: na::Vector3<f32>,
                        vp_down: na::Vector3<f32>,
                        light: na::Vector3<f32>
                        ) {
        if self.imagewidth != width || self.imageheight != height {
            // Need to resize the buffer
            // TODO: wait for the queue to empty
            self.imagewidth = width;
            self.imageheight = height;
        }
    }

    pub fn save_voxels(&mut self) {
        let mut tmpvec = vec![0u8; self.voxelsize*self.voxelsize*self.voxelsize];
        let mut file = File::create("voxels.dat").unwrap();
        file.write_all(tmpvec.as_slice()).unwrap();
    }

    pub fn save_debug(&mut self) {
        let mut tmpvec = vec![0.0f32; self.imagewidth*self.imageheight];
        let mut file = File::create("debug.dat").unwrap();
        bincode::serialize_into(&mut file, &tmpvec).unwrap();
    }

}

