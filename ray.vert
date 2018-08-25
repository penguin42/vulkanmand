#version 450 

layout (location = 0) out vec2 outUV;

// Full screen quad vertex shader from https://www.saschawillems.de/?page_id=2122
// compile me with glslangValidator -V ray.vert -o ray-vert.spv
void main() 
{
	outUV = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
	gl_Position = vec4(outUV * 2.0f + -1.0f, 0.0f, 1.0f);
}
