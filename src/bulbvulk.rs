// based on the trival.rs example from the ocl crate

use glib::translate::ToGlibPtr;
use gdk::WindowExt;
use std;
use std::borrow::Cow;
use std::ffi::CStr;
use std::fs::File;
use std::io::*;
use std::rc::Rc;
use std::sync::Arc;
use vulkano;
use vulkano::buffer;
use vulkano::command_buffer;
use vulkano::framebuffer::{Framebuffer, RenderPassAbstract, Subpass};
use vulkano::descriptor::descriptor;
use vulkano::descriptor::descriptor::ShaderStages;
use vulkano::descriptor::{descriptor_set, PipelineLayoutAbstract, pipeline_layout};
use vulkano::device;
use vulkano::format;
use vulkano::image;
use vulkano::image::SwapchainImage;
use vulkano::instance;
use vulkano::pipeline;
use vulkano::pipeline::shader;
use vulkano::pipeline::shader::{EmptyShaderInterfaceDef, GraphicsShaderType, ShaderInterfaceDef, ShaderInterfaceDefEntry};
use vulkano::pipeline::{viewport, ComputePipeline, GraphicsPipeline};
use vulkano::single_pass_renderpass;
use vulkano::swapchain;
use vulkano::sync;
use vulkano::sync::GpuFuture;

use gtk::*;

static dummy1: usize = 1;

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
                      ty: descriptor::DescriptorDescTy::Image(descriptor::DescriptorImageDesc {
                          sampled: false,
                          multisampled: false,
                          dimensions: descriptor::DescriptorImageDescDimensions::ThreeDimensional,
                          array_layers: descriptor::DescriptorImageDescArray::NonArrayed,
                          format: Some(format::Format::R8Uint),
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
struct RayVertLayout(descriptor::ShaderStages);
unsafe impl pipeline_layout::PipelineLayoutDesc for RayVertLayout {
        // The outputs of a vertex shader don't seem to be a descriptor
        fn num_sets(&self) -> usize { 0 }
        fn num_bindings_in_set(&self, set: usize) -> Option<usize> {
            match set { _ => None, }
        }
        fn descriptor(&self, set: usize, binding: usize) -> Option<descriptor::DescriptorDesc> {
            match (set, binding) { _ => None, }
        }
        fn num_push_constants_ranges(&self) -> usize { 0 }
        fn push_constants_range(&self, num: usize) -> Option<pipeline_layout::PipelineLayoutDescPcRange> {
            if num != 0 { return None; }
            return None;
        }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct RayFragOutput;
unsafe impl ShaderInterfaceDef for RayFragOutput {
    type Iter = RayFragOutputIter;

    fn elements(&self) -> RayFragOutputIter {
        RayFragOutputIter(0)
    }
}
// This structure will tell Vulkan how output entries (those passed to next
// stage) of our vertex shader look like.
#[derive(Debug, Copy, Clone)]
struct RayFragOutputIter(u16);
impl Iterator for RayFragOutputIter {
    type Item = ShaderInterfaceDefEntry;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            self.0 += 1;
            return Some(ShaderInterfaceDefEntry {
                location: 0..1,
                format: format::Format::R32G32B32A32Sfloat,
                name: Some(Cow::Borrowed("f_color"))
            })
        }
        None
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (1 - self.0) as usize;
        (len, Some(len))
    }
}
impl ExactSizeIterator for RayFragOutputIter {
}

#[derive(Debug, Copy, Clone)]
struct RayFragLayout(descriptor::ShaderStages);
unsafe impl pipeline_layout::PipelineLayoutDesc for RayFragLayout {
        // The outputs of a fragment shader don't seem to be a descriptor
        // Voxels: binding 0 in set 0
        fn num_sets(&self) -> usize { 1 }
        fn num_bindings_in_set(&self, set: usize) -> Option<usize> {
            match set {
                0 => Some(1), // Voxels binding 0 set 0
                _ => None,
            }
        }
        fn descriptor(&self, set: usize, binding: usize) -> Option<descriptor::DescriptorDesc> {
            match (set, binding) {
                (0,0) => Some(descriptor::DescriptorDesc {
                      array_count: 1,
                      stages: descriptor::ShaderStages { fragment: true, ..descriptor::ShaderStages::none() },
                      readonly: false,
                      ty: descriptor::DescriptorDescTy::Image(descriptor::DescriptorImageDesc {
                          sampled: false,
                          multisampled: false,
                          dimensions: descriptor::DescriptorImageDescDimensions::ThreeDimensional,
                          array_layers: descriptor::DescriptorImageDescArray::NonArrayed,
                          format: Some(format::Format::R8Uint),
                      }),
                  }),
                _ => None,
            }
        }
        // We've got one push constant layout
        fn num_push_constants_ranges(&self) -> usize { 1 }
        fn push_constants_range(&self, num: usize) -> Option<pipeline_layout::PipelineLayoutDescPcRange> {
            if num != 0 { return None; }
            Some(pipeline_layout::PipelineLayoutDescPcRange {
                     offset: 0,
                     size: 6 * 16,
                     stages: descriptor::ShaderStages { fragment: true, ..descriptor::ShaderStages::none() } })
        }
}

pub struct Bulbvulk {
    voxelsize: usize, // typically 256 for 256x256x256

    imagewidth: usize,
    imageheight: usize,

    vdevice: Arc<device::Device>,
    vqueue: Arc<device::Queue>,

    voxelimg: Arc<image::StorageImage<format::R8Uint>>, 

    swsurface: Arc<swapchain::Surface<usize>>,
    swapc : Arc<swapchain::Swapchain<usize>>,
    swapbuf : std::vec::Vec<std::sync::Arc<SwapchainImage<usize>>>,

    rayimg: Arc<image::StorageImage<format::R8G8B8A8Uint>>,

    mandpipe: Arc<ComputePipeline<pipeline_layout::PipelineLayout<MandLayout>>>,
    raypipe: Arc<GraphicsPipeline<pipeline::vertex::BufferlessDefinition,
                                  std::boxed::Box<PipelineLayoutAbstract + Send + Sync + 'static>,
                                  Arc<RenderPassAbstract + Send + Sync + 'static>
                                 >>,

    raypass: Arc<RenderPassAbstract + Send + Sync>,
}

impl Bulbvulk {
    pub fn new(win: Rc<Widget>) -> Bulbvulk {
        let voxelsize = 4; // Dummy initial dimension

        let imagewidth : usize = 4; // Dummy initial dimension
        let imageheight : usize = 4; // Dummy initial dimension
        let layer = "VK_LAYER_LUNARG_standard_validation";
        let layers = vec![layer];
        let vinstance = instance::Instance::new(None,
                                       &instance::InstanceExtensions {
                                            ext_debug_report: true,
                                            khr_surface: true,
                                            khr_xlib_surface: true,
                                            ..instance::InstanceExtensions::none()
                                       },
                                       layers).unwrap();

        let vpdev = Arc::new(instance::PhysicalDevice::enumerate(&vinstance).next().unwrap());

        // Would it make sense to have multiple queue sets, one with just compute?
        let qf = vpdev.queue_families().filter(|q| q.supports_compute() &&
                                                   q.supports_transfers() &&
                                                   q.supports_graphics()).next().unwrap();

        let (vdevice, mut vqueueiter) = device::Device::new(*vpdev.clone(), &device::Features::none(),
                                                            &device::DeviceExtensions { khr_swapchain: true, ..device::DeviceExtensions::none() },
                                                            Some((qf, 1.0))).unwrap();
        // Only using one queue
        let vqueue = vqueueiter.next().unwrap();

        let voxelimg = image::StorageImage::with_usage(vdevice.clone(),
                                                     image::Dimensions::Dim3d { width: voxelsize as u32, height: voxelsize as u32, depth: voxelsize as u32},
                                                     format::R8Uint,
                                                     image::ImageUsage { storage: true, transfer_source: true,
                                                                         ..image::ImageUsage::none()},
                                                     vdevice.active_queue_families()).unwrap();

        let mandcs = {
            let mut f = File::open("mandel.spv").unwrap();
            let mut v = vec![];
            f.read_to_end(&mut v).unwrap();
            unsafe { shader::ShaderModule::new(vdevice.clone(), &v) }.unwrap()
        };

        // ** TODO: Abstract and make not just X11
        let scr = win.get_screen();
        // a gdk::Window ?
        let gdk_win = win.get_window().unwrap();
        let enres = gdk_win.ensure_native();
        println!("ensure_native said: {}\n", enres);

        // Note! This is a gdk display not a X11 display - *mut gdk_sys::GdkDisplay
        let gdk_display = unsafe { gdk_sys::gdk_window_get_display(gdk_win.to_glib_none().0) };
        extern {
            fn gdk_x11_display_get_xdisplay(gdkdisp: *mut gdk_sys::GdkDisplay) -> *mut x11_dl::xlib::Display;
        }
        let x11_display = unsafe { gdk_x11_display_get_xdisplay(gdk_display) };
        // Don't know if x11_display is right?  Also need xid for the window, might need to
        // ensure_native, and how do we know it's X? (GDK_IS_X11_WINDOW() - how to macro? )
        // something like gdk_x11_drawable_get_xid(gtk_widget_get_window(widget));
        extern {
            fn gdk_x11_window_get_xid(gdkwin: *mut gdk_sys::GdkWindow) -> std::os::raw::c_ulong;
        }
        // The 'xid' I'm getting seems to be the outer window?
        let xid = unsafe { gdk_x11_window_get_xid(gdk_win.to_glib_none().0) };

        // The last param here is just for lifetime?
        let swsurface = unsafe { swapchain::Surface::from_xlib(vinstance.clone(), x11_display, xid, dummy1).unwrap() };

        println!("scr={:?} win_display={:?} x11_display={:?} xid={:?}\n", scr, gdk_display, x11_display, xid);

        let surfcaps = swsurface.capabilities(vdevice.physical_device()).unwrap();
        println!("surface capbilitie={:?}\n", surfcaps);
        let (surfformat, _surfcolourspace) = surfcaps.supported_formats[0];
        let sharing_mode = sync::SharingMode::Exclusive(vqueue.family().id());
        let (swapc, swapbuf) = swapchain::Swapchain::new(
                vdevice.clone(), swsurface.clone(),
                2, // images in the swap chain - was the minimum it allowed
                surfformat,
                surfcaps.min_image_extent, /* Hmm, is this window size? Currently looks like it*/
                1, // layers/image
                image::ImageUsage { color_attachment: true,
                                    transfer_source: true,
                                    transfer_destination: true,
                                    .. image::ImageUsage::none() },
                sharing_mode,
                swapchain::SurfaceTransform::Identity,
                swapchain::CompositeAlpha::Opaque,
                swapchain::PresentMode::Fifo,
                true, // Clip that which isn't visible
                None, // No previous swapchain
            ).unwrap();

        let rayimg = image::StorageImage::with_usage(vdevice.clone(),
                                                     image::Dimensions::Dim2d { width: imagewidth as u32, height: imageheight as u32},
                                                     format::R8G8B8A8Uint,
                                                     image::ImageUsage { storage: true, transfer_source: true,
                                                                         ..image::ImageUsage::none()},
                                                     vdevice.active_queue_families()).unwrap();

        // Simple vertex shader, just gives us a triangle covering the whole window
        let rayvs = {
            let mut f = File::open("ray-vert.spv").unwrap();
            let mut v = vec![];
            f.read_to_end(&mut v).unwrap();
            unsafe { shader::ShaderModule::new(vdevice.clone(), &v) }.unwrap()
        };
        // The ray tracing fragment shader
        let rayfs = {
            let mut f = File::open("ray-frag.spv").unwrap();
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

        // Renderpass from vulkano triangle example
        // TODO: Hmm, do we want this more dynamic? Where do we pass my pc's
        let raypass = Arc::new(single_pass_renderpass!(vdevice.clone(),
            attachments: {
                // `color` is a custom name we give to the first and only attachment.
                color: {
                    // `load: Clear` means that we ask the GPU to clear the content of this
                    // attachment at the start of the drawing.
                    load: Clear,
                    // `store: Store` means that we ask the GPU to store the output of the draw
                    // in the actual image. We could also ask it to discard the result.
                    store: Store,
                    // `format: <ty>` indicates the type of the format of the image. This has to
                    // be one of the types of the `vulkano::format` module (or alternatively one
                    // of your structs that implements the `FormatDesc` trait). Here we use the
                    // generic `vulkano::format::Format` enum because we don't know the format in
                    // advance.
                    format: swapc.format(),
                    // TODO:
                    samples: 1,
                }
            },
            pass: {
                // We use the attachment named `color` as the one and only color attachment.
                color: [color],
                // No depth-stencil attachment is indicated with empty brackets.
                depth_stencil: {}
            }).unwrap()) as Arc<RenderPassAbstract + Send + Sync>;
        let ray_vert_main = unsafe {
            rayvs.graphics_entry_point(CStr::from_bytes_with_nul_unchecked(b"main\0"),
                                                      EmptyShaderInterfaceDef, // No input to our vertex shader
                                                      EmptyShaderInterfaceDef, // No output from our vertex shader other than to gl_Position
                                                      RayVertLayout(ShaderStages { vertex: true, ..ShaderStages::none() }),
                                                      GraphicsShaderType::Vertex
                                                      ) };
        let ray_frag_main = unsafe {
            rayfs.graphics_entry_point(CStr::from_bytes_with_nul_unchecked(b"main\0"),
                                                      EmptyShaderInterfaceDef, // No input to our fragment shader at the moment
                                                      RayFragOutput,
                                                      RayFragLayout(ShaderStages { fragment: true, ..ShaderStages::none() }),
                                                      GraphicsShaderType::Fragment
                                                      ) };
        // Ray pipe from vulkano triangle example crossed with the runtime-shader example
        let raypipe = Arc::new(GraphicsPipeline::start()
            // We need to indicate the layout of the vertices.
            .vertex_input(pipeline::vertex::BufferlessDefinition {})
            .vertex_shader(ray_vert_main, ())
            // The content of the vertex buffer describes a list of triangles.
            .triangle_list()
            .cull_mode_back() // ????
            .front_face_clockwise() // ????
            .viewports_scissors_dynamic(1)
            // See `vertex_shader`.
            .fragment_shader(ray_frag_main, ())
            // We have to indicate which subpass of which render pass this pipeline is going to be used
            // in. The pipeline will only be usable from this particular subpass.
            .render_pass(Subpass::from(raypass.clone(), 0).expect("pipeline/render_pass"))
            // Now that our builder is filled, we call `build()` to obtain an actual pipeline.
            .build(vdevice.clone())
            .expect("raypipe"));

        println!("Vulkan device: {}", vpdev.name());
        Bulbvulk { imagewidth, imageheight, voxelsize,
                   vdevice, vqueue, voxelimg, rayimg,
                   mandpipe, raypass, raypipe,
                   swsurface, swapc, swapbuf }
    }

    pub fn calc_bulb(&mut self, size: usize, power: f32) {
        if self.voxelsize != size {
            // Need to resize the buffer
            self.voxelsize = size;
            self.voxelimg = image::StorageImage::with_usage(self.vdevice.clone(),
                                                     image::Dimensions::Dim3d { width: self.voxelsize as u32, height: self.voxelsize as u32, depth: self.voxelsize as u32},
                                                     format::R8Uint,
                                                     image::ImageUsage { storage: true, transfer_source: true,
                                                                         ..image::ImageUsage::none()},
                                                     self.vdevice.active_queue_families()).unwrap();
        }
        // Do I really want persistent - this is transitory
        let set = Arc::new(descriptor_set::PersistentDescriptorSet::start(self.mandpipe.clone(), 0)
                  .add_image(self.voxelimg.clone()).unwrap()
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

    pub fn render_image(&mut self,
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

        let mut image_num = 0;
        let mut acquire_future_opt = None;

        let mut recreate_swapchain = true;
        while recreate_swapchain {
            let (_image_num, _acquire_future) = match swapchain::acquire_next_image(self.swapc.clone(), None) {
                Ok(r) => r,
                Err(swapchain::AcquireError::OutOfDate) => {
                    println!("render_image OutOfDate!\n");
                    let surfcaps = self.swsurface.capabilities(self.vdevice.physical_device()).unwrap();
                    let surfdims = surfcaps.min_image_extent;

                    println!("recreating with size {:?}\n", surfdims);
                    let (new_swapc, new_swapbuf) = match self.swapc.recreate_with_dimension(surfdims) {
                        Ok(r)=>r,
                        // Manual resize, try again
                        Err(swapchain::SwapchainCreationError::UnsupportedDimensions) => {
                            println!("swapchain recreation failed - trying again");
                            continue;
                        }
                        Err(err) => panic!("{:?}", err)
                    };
                    self.swapc = new_swapc;
                    self.swapbuf = new_swapbuf;
                    // TODO rebuildraypass?
                    continue;
                },
                Err(err) => panic!("{:?}", err)
            };
            recreate_swapchain = false;
            image_num = _image_num;
            acquire_future_opt = Some(_acquire_future);
        }
        let acquire_future =acquire_future_opt.expect("No acquire future");

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
                                                     format::R8G8B8A8Uint,
                                                     image::ImageUsage { storage: true, transfer_source: true,
                                                                         ..image::ImageUsage::none()},
                                                     self.vdevice.active_queue_families()).unwrap();
        }
        let curimage = &self.swapbuf[image_num];

        // TODO: Lifetime of this is just wrong, triangle example keeps it
        let fb = Arc::new(Framebuffer::start(self.raypass.clone()).add(curimage.clone()).unwrap().build().unwrap());

        // Do I really want persistent - this is transitory
        let set = Arc::new(descriptor_set::PersistentDescriptorSet::start(self.raypipe.clone(), 0)
                  .add_image(self.voxelimg.clone()).expect("add voxelimg")
                  .build().expect("pds build"));
        let dynamic_state = command_buffer::DynamicState {
            viewports: Some(vec![viewport::Viewport {
                origin: [0.0, 0.0],
                dimensions: [width as f32,height as f32],
                depth_range: 0.0 .. 1.0,
            }]),
            scissors: Some(vec![viewport::Scissor {
                origin: [0, 0],
                dimensions: [width as u32,height as u32],
            }]),
            .. command_buffer::DynamicState::none()
        };

        let combuf = command_buffer::AutoCommandBufferBuilder::primary_one_time_submit(self.vdevice.clone(), self.vqueue.family()).unwrap()
                     .begin_render_pass(fb, false /* secondary */, vec![[0.0,0.0,1.0,0.0].into()]).expect("one time submit/begin render pass")
                     .draw(self.raypipe.clone(),
                           &dynamic_state,
                           pipeline::vertex::BufferlessVertices { vertices: 3, instances: 1 /* ? */ },
                           
                           set, pc
                           ).expect("draw")
                     .end_render_pass().expect("one time submit/end render pass")
                     .build().expect("one time submit/build");
        // Engage!
        let mut future = sync::now(self.vdevice.clone())
                     .join(acquire_future) // TODO - stuff with previous frame
                     .then_execute(self.vqueue.clone(), combuf).expect("sync/execute")
                     .then_swapchain_present(self.vqueue.clone(), self.swapc.clone(), image_num)
                     .then_signal_fence_and_flush().expect("sync/signal f&f");
        // Wait for it
        future.wait(None).unwrap();
        future.cleanup_finished();
    }

    pub fn save_voxels(&mut self) {
        // We can't read directly from the voxel buffer since it's DeviceLocal, so
        // we copy it into a temporary CPU buffer
        // I'd like to use a CpuBufferPool here but there doesn't seem to be a way to do array
        // allocations
        let cpubuf = unsafe { buffer::cpu_access::CpuAccessibleBuffer::<[u8]>::uninitialized_array(self.vdevice.clone(),
                                                                                          self.voxelsize*self.voxelsize*self.voxelsize,
                                                                                          buffer::BufferUsage::all()).unwrap() };

        let combuf = command_buffer::AutoCommandBufferBuilder::primary_one_time_submit(self.vdevice.clone(), self.vqueue.family()).unwrap()
                       .copy_image_to_buffer(self.voxelimg.clone(), cpubuf.clone()).unwrap()
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

