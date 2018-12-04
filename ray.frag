#version 450

// compile me with glslangValidator -V ray.frag -o ray-frag.spv
// Voxels in from compute
layout(r8ui, binding = 0) uniform uimage3D voxels;

// interpolated coords from vertex shader
layout(location = 0) in vec2 inUV;

// Pixels out to display
layout(location = 0) out vec4 f_color;

layout(std430,push_constant, binding = 0) uniform Pc {
  vec3 eye;
  vec3 vpmid;
  vec3 vpplusx; // half of width
  vec3 vpplusy; // half of height
  vec3 light;
  vec3 voxelsize;
} pc;

void main() {
    f_color = vec4(1.0, inUV[0], inUV[1], 1.0);
}
