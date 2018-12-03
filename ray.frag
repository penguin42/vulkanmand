#version 450

// compile me with glslangValidator -V ray.frag -o ray-frag.spv
// Voxels in from compute
layout(r8ui, binding = 0) uniform uimage3D voxels;

// interpolated coords from vertex shader
layout(location = 0) in vec2 inUV;

// Pixels out to display
layout(location = 0) out vec4 f_color;
void main() {
    f_color = vec4(1.0, inUV[0], inUV[1], 1.0);
}
