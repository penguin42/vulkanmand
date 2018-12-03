#version 450 

// Full screen quad vertex shader from https://www.saschawillems.de/?page_id=2122
// compile me with glslangValidator -V ray.vert -o ray-vert.spv

// TODO: If we want to make this form a cube then we're going to need to use gl_InstanceID to tell
// which triangle?

layout (location = 0) out vec2 outUV;
void main() 
{
  // Vertex   outUV   pos
  //   0      0,0     -1,-1,0,1
  //   1      2,0      3,-1,0,1
  //   2      0,2     -1, 3,0,1
  // I think it's -1,-1 -> 1,1 that's visible ?
	outUV = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
	gl_Position = vec4(outUV * 2.0f + -1.0f, 0.0f, 1.0f);
}
