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
 */

#define CL_HPP_ENABLE_EXCEPTIONS
#define CL_HPP_TARGET_OPENCL_VERSION 110

#include <CL/cl.hpp>
#include <iostream>
#include <iomanip>

const char *kern_hello = "__kernel void hello(__global uint* o) { \
  int z = get_global_id(0); \
  int y = get_global_id(1); \
  int x = get_global_id(2); \
  int zr = get_global_size(0); \
  int yr = get_global_size(1); \
  int xr = get_global_size(2); \
  float zf = ((float)z - ((float)zr)/2) / (float)zr; \
  float yf = ((float)y - ((float)yr)/2) / (float)yr; \
  float xf = ((float)x - ((float)xr)/2) / (float)xr; \
\
  o[z*yr*xr + y*xr + x] = ((zf * zf) + (yf * yf) + (xf * xf)) <  0.25; \
}";
#define SIZE 16

static int got_dev(cl::Platform &plat, std::vector<cl::Device> &devices, cl::Device &dev, cl::Context &con)
{
  std::cout << dev.getInfo<CL_DEVICE_NAME>() << std::endl;
  cl_int err = CL_SUCCESS;
  cl_uint* mapped;

  try {
    cl::Program::Sources hello_src(1, std::make_pair(kern_hello,strlen(kern_hello)));
    cl::Program hello_prog = cl::Program(con, hello_src);
    hello_prog.build(devices);

    cl::Kernel hello_kern(hello_prog, "hello", &err);
    cl::Buffer output(con, CL_MEM_WRITE_ONLY,  SIZE * SIZE * SIZE * sizeof(cl_uint));
    hello_kern.setArg(0, output);
    cl::CommandQueue queue(con, dev);
    cl::Event event;
    err = queue.enqueueNDRangeKernel(
        hello_kern,
        cl::NullRange, /* Offsets */
        cl::NDRange(SIZE,SIZE,SIZE), /* Global range */
        cl::NullRange /* Local range */
    );
    std::cerr << __func__ << "NDRangeKernel gave: " << err << std::endl;
    mapped = (cl_uint*)queue.enqueueMapBuffer(output, CL_TRUE /* blocking */, CL_MAP_READ,
                           0 /* offset */, 
                           SIZE * SIZE * SIZE * sizeof(cl_uint) /* size */,
                           NULL /* event list to wait for */,
                           &event,
                           &err);
    std::cerr << __func__ << "enqueueMapBuffer gave: " << err << std::endl;
    event.wait();
  }
  catch (...) {
    std::cerr << __func__ << ": Error: " << err << std::endl;
    return -1;
  }
  /* Just dump our buffer */
  for(int z=0;z<SIZE;z++) {
    std::cerr << __func__ << std::endl << "Z: " << z << std::endl;
    for(int y=0;y<SIZE;y++) {
      for(int x=0;x<SIZE;x++) {
        std::cerr << std::setw(4) << mapped[z*SIZE*SIZE+y*SIZE+x];
      }
      std::cerr << std::endl;
    }
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
