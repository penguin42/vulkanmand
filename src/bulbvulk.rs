// based on the trival.rs example from the ocl crate

extern crate vulkano;
extern crate nalgebra as na;
extern crate bincode;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use self::vulkano::instance;
use self::vulkano::device;

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

pub struct Bulbvulk {
    voxelsize: usize, // typically 256 for 256x256x256

    imagewidth: usize,
    imageheight: usize,

    vdevice: Arc<device::Device>,
    vqueue: Arc<device::Queue>,
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
        println!("Vulkan device: {}", VPHYSDEVICE.name());
        Bulbvulk { imagewidth, imageheight, voxelsize, vdevice, vqueue }
    }

    pub fn calc_bulb(&mut self, size: usize, power: f32) {
        if self.voxelsize != size {
            // Need to resize the buffer
            self.voxelsize = size;
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

