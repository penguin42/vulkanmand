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

#include <CL/cl2.hpp>
#include <iostream>
#include <iomanip>
#include <string>
#include <fstream>
#include <streambuf>

#define SIZE 256

static void dump_voxels(cl_uchar *mapped) {
    /* Just dump our buffer */
    for(int z=0;z<SIZE;z++) {
      std::cerr << __func__ << std::endl << "Z: " << z << std::endl;
      for(int y=0;y<SIZE;y++) {
        for(int x=0;x<SIZE;x++) {
          std::cerr << std::setw(2) << (int)mapped[z*SIZE*SIZE+y*SIZE+x];
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
    cl::Buffer voxels(con, CL_MEM_WRITE_ONLY,  SIZE * SIZE * SIZE * sizeof(cl_uchar));
    mandkern.setArg(0, voxels);
    cl::CommandQueue queue(con, dev);
    cl::Event event;
    std::vector<cl::Event> events;
    err = queue.enqueueNDRangeKernel(
        mandkern,
        cl::NullRange, /* Offsets */
        cl::NDRange(SIZE,SIZE,SIZE), /* Global range */
        cl::NullRange, /* Local range */
        NULL,
        &event /* When we're done */
    );
    std::cerr << __func__ << "NDRangeKernel gave: " << err << std::endl;
    /* Get the map to wait for the kernel to finish */
    events.push_back(event);
    cl::Event eventMap;
    queue.enqueueBarrierWithWaitList(&events);
    mapped_voxels = (cl_uchar*)queue.enqueueMapBuffer(voxels, CL_TRUE /* blocking */, CL_MAP_READ,
                           0 /* offset */, 
                           SIZE * SIZE * SIZE * sizeof(cl_uchar) /* size */,
                           &events,
                           &eventMap,
                           &err);
    cl::Event eventBarrier2;
    queue.enqueueBarrierWithWaitList(NULL,&eventBarrier2);
    std::cerr << __func__ << "enqueueMapBuffer gave: " << err << std::endl;
    eventMap.wait();
    eventBarrier2.wait();

    dump_voxels(mapped_voxels);
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
