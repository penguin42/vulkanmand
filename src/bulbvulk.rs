// based on the trival.rs example from the ocl crate

extern crate nalgebra as na;
extern crate bincode;
use std::fs::File;
use std::io::Write;

const RENDER_CONFIG_SIZE : usize =  18;

pub struct Bulbvulk {
    voxelsize: usize, // typically 256 for 256x256x256

    imagewidth: usize,
    imageheight: usize,
}

impl Bulbvulk {
    pub fn new() -> Bulbvulk {
        let voxelsize = 4; // Dummy initial dimension

        let imagewidth = 4; // Dummy initial dimension
        let imageheight = 4; // Dummy initial dimension

        Bulbvulk {  imagewidth, imageheight, voxelsize }
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

