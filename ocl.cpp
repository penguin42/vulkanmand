/* OpenCL play thing,
 * Based off Khronos bindings example at http://github.khronos.org/OpenCL-CLHPP/
 * but moving back to the 1.1 world in the headers
 * http://downloads.ti.com/mctools/esd/docs/opencl/execution/kernels-workgroups-workitems.html  helps
 * Dave Gilbert (c) 2017 - dave@treblig.org
 *
 * g++ -O2 ocl.cpp -lOpenCL
 *
 * Note we need to stick to 1.1 since that's all Clover does on Mesa
 *
 * Hmm oclgrind passes - but still gets some wrong data?
 * try putting a barrier in but I don't see why I need it.
 *   - and it doesn't help on the Radeon - but does under oclgrind?
 *
 */

#define CL_HPP_MINIMUM_OPENCL_VERSION 120
#define CL_HPP_TARGET_OPENCL_VERSION 120
//#define CL_HPP_ENABLE_EXCEPTIONS


#include <CL/cl2.hpp>
#include <iostream>
#include <iomanip>
#include <string>
#include <fstream>
#include <streambuf>

#define VOXSIZE 128
#define IMGWIDTH 320
#define IMGHEIGHT 240

static void dump_voxels(cl_uchar *mapped) {
    /* Just dump our buffer */
    for(int z=0;z<VOXSIZE;z++) {
      std::cerr << __func__ << std::endl << "Z: " << z << std::endl;
      for(int y=0;y<VOXSIZE;y++) {
        for(int x=0;x<VOXSIZE;x++) {
          std::cerr << std::setw(2) << (int)mapped[z*VOXSIZE*VOXSIZE+y*VOXSIZE+x];
        }
        std::cerr << std::endl;
      }
    }
}

static int got_dev(cl::Platform &plat, std::vector<cl::Device> &devices, cl::Device &dev, cl::Context &con)
{
  std::cout << dev.getInfo<CL_DEVICE_NAME>() << std::endl;
  cl_int err = CL_SUCCESS;
  cl_uchar* mapped_voxels;

  try {
    std::vector<std::string> programStrings;
    std::ifstream mandkernstrm("mandel.ocl");
    std::string mandkstr((std::istreambuf_iterator<char>(mandkernstrm)), std::istreambuf_iterator<char>());
    programStrings.push_back(mandkstr);
    std::ifstream raykernstrm("ray.ocl");
    std::string raykstr((std::istreambuf_iterator<char>(raykernstrm)), std::istreambuf_iterator<char>());
    programStrings.push_back(raykstr);
    cl::Program prog(con, programStrings);

    cl_int buildErr = CL_SUCCESS;
    try {
      prog.build(devices);
    }
    catch (...) {
      auto buildInfo = prog.getBuildInfo<CL_PROGRAM_BUILD_LOG>(&buildErr);
      for (auto &pair : buildInfo) {
        std::cerr << "build gave " << pair.second << std::endl;
      }
      return err;
    }
    auto buildInfo = prog.getBuildInfo<CL_PROGRAM_BUILD_LOG>(&buildErr);
    for (auto &pair : buildInfo) {
      std::cerr << "build gave " << pair.second << std::endl;
    }

    cl::Kernel mandkern(prog, "mandel", &err);
    cl::Buffer voxels(con, CL_MEM_READ_WRITE,  VOXSIZE * VOXSIZE * VOXSIZE * sizeof(cl_uchar));
    mandkern.setArg(0, voxels);
    cl::CommandQueue queue(con, dev);
    cl::Event mandkernevent;
    std::vector<cl::Event> events;
    err = queue.enqueueNDRangeKernel(
        mandkern,
        cl::NullRange, /* Offsets */
        cl::NDRange(VOXSIZE,VOXSIZE,VOXSIZE), /* Global range */
        cl::NullRange, /* Local range */
        NULL,
        &mandkernevent /* When we're done */
    );
    std::cerr << __func__ << "NDRangeKernel gave: " << err << std::endl;
    if (err) {
        return err;
    }
    events.push_back(mandkernevent);
    queue.enqueueBarrierWithWaitList(&events);
    mandkernevent.wait();
    queue.finish(); // Seem to need this on Mesa on Turks

    //
    //
    //
    //     ^
    //     |y    
    //     |   ^
    //     |  /z
    //     | /
    //     |/
    //     .-----> x
    //
    //     eye . ---> - viewplane - ---> [ voxel array ]
    float config_c[] = {
      // These are all in voxel coordinates - the real voxels run from 0..VOXSIZE
      // in each dimension
      VOXSIZE/2.0, VOXSIZE/2.0, -3.0 * VOXSIZE, // Eye
      VOXSIZE/2.0, VOXSIZE/2.0, -2.0 * VOXSIZE, // View plane mid
      VOXSIZE, 0.0, 0.0, // View plane right
      0.0, VOXSIZE, 0.0, // View plane down
      VOXSIZE, VOXSIZE, VOXSIZE, // Dimensions of voxel array
    };
    cl::Kernel raykern(prog, "ray", &err);
    std::cerr << __func__ << "raykern construct: err=" << err << std::endl;
    cl::Buffer image(con, CL_MEM_WRITE_ONLY, IMGWIDTH*IMGHEIGHT*sizeof(cl_uchar));
    cl::Buffer debugfloat(con, CL_MEM_WRITE_ONLY, IMGWIDTH*IMGHEIGHT*sizeof(cl_float));
    cl::Buffer config(con, CL_MEM_READ_ONLY|CL_MEM_USE_HOST_PTR, sizeof(config_c), config_c);
    raykern.setArg(0, image);
    raykern.setArg(1, voxels);
    raykern.setArg(2, config);
    raykern.setArg(3, debugfloat);

    cl::Event raykernevent;
    std::vector<cl::Event> rayevents;
    err = queue.enqueueNDRangeKernel(
       raykern,
       cl::NullRange,
       cl::NDRange(IMGWIDTH, IMGHEIGHT),
       cl::NullRange,
       NULL,
       &raykernevent);
    std::cerr << __func__ << "(ray)NDRangeKernel gave: " << err << std::endl;
    rayevents.push_back(raykernevent);
    raykernevent.wait();
    cl::Event eventMap;
    cl_uchar *mapped_image = (cl_uchar *)queue.enqueueMapBuffer(image, CL_TRUE /* blocking */, 
                                                                   CL_MAP_READ,
                                                                   0 /* offset */,
                                                                   IMGWIDTH * IMGHEIGHT * sizeof(cl_uchar) /* size */,
                                                                   &rayevents,
                                                                   &eventMap,
                                                                   &err);
    eventMap.wait();
    std::cerr << __func__ << "mapped_image: at" << (void *)mapped_image << " Err=" << err << std::endl;
    cl::Event eventMapF;
    cl_float *mapped_debugfloat = (cl_float *)queue.enqueueMapBuffer(debugfloat, CL_TRUE /* blocking */, 
                                                                   CL_MAP_READ,
                                                                   0 /* offset */,
                                                                   IMGWIDTH * IMGHEIGHT * sizeof(cl_float) /* size */,
                                                                   NULL,
                                                                   &eventMapF,
                                                                   &err);
    eventMapF.wait();
    std::cerr << __func__ << "mapped_debugfloat: at" << (void *)mapped_debugfloat << " Err=" << err << std::endl;
    //std::vector<cl::Event> mapevents;
    //mapevents.push_back(eventMap);
    //cl::Event rayBarrierEvent;
    //queue.enqueueBarrierWithWaitList(&mapevents, &rayBarrierEvent);
    //rayBarrierEvent.wait();
    queue.finish();
    
    for(int iy=0;iy<IMGHEIGHT;iy++) {
      for(int ix=0;ix<IMGWIDTH;ix++) {
        //std::cerr << std::setw(2) << std::hex << (int)mapped_image[iy*IMGWIDTH+ix] << "/" << mapped_debugfloat[iy*IMGWIDTH+ix] << "|";
        //std::cerr << std::setw(2) << std::hex << (int)mapped_image[iy*IMGWIDTH+ix];
        std::cerr << (int)mapped_image[iy*IMGWIDTH+ix] << " ";
      }
      std::cerr << std::endl;
    }
  }
  catch (...) {
    std::cerr << __func__ << ": Error: " << err << std::endl;
    return -1;
  }

  return 0;
}

int main(void)
{
  std::vector<cl::Platform> platforms;
  cl::Platform::get(&platforms);
    
  for(auto &p : platforms) {
    std::cout << p.getInfo<CL_PLATFORM_NAME>() << " by " << p.getInfo<CL_PLATFORM_VENDOR>() << std::endl;

    cl_context_properties cl_prop[] = {
      CL_CONTEXT_PLATFORM, (cl_context_properties)p(), 0
    };
    cl::Context context(CL_DEVICE_TYPE_GPU, cl_prop);
    std::vector<cl::Device> devices = context.getInfo<CL_CONTEXT_DEVICES>();
    for(auto &d : devices) {
      return got_dev(p, devices, d, context);
    }

  }
  std::cerr << "No OpenCL platform/device" << std::endl;
  return -1;
}
