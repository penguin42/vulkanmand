#version 450

// compile me with glslangValidator -V ray.frag -o ray-frag.spv
layout(location = 0) out vec4 f_color;
void main() {
    f_color = vec4(1.0, 0.0, 0.0, 1.0);
}