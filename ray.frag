#version 450

// compile me with glslangValidator -V ray.frag -o ray-frag.spv
// Voxels in from compute
layout(r8ui, binding = 0) uniform uimage3D voxels;

// interpolated coords from vertex shader - runs 0..1,0..1
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

bool hitend(float cur, float dir, float lim) {
  if (dir >= 0) {
     return cur > lim;
  } else {
     return cur <= 0;
  }
}

// Angle between the eye, the voxel (vx/y/z) and the light (lx/y/z)
float lightangle(vec3 eye, vec3 voxel,  vec3 light) {
  // Vector from eye to voxel
  vec3 ev = voxel - eye;
  // Vector from light to voxel
  vec3 lv = voxel - light;

  float magev = length(ev);
  float maglv = length(lv);

  float dotp = dot(ev,lv);

  // cos(angle)
  float cosang = dotp / (magev * maglv);

  float res = pow(cosang,2.0); // About 0.39..0.72
  if (res < 0) res = 0;
  return res;
}

void main() {
  // TODO: Convert to the vertex shader rendering a cube
  // and it doing all the geometry work to tell us the
  // voxel we're aiming for
  ivec3 vsize = ivec3(pc.voxelsize.x, pc.voxelsize.y, pc.voxelsize.z);

  // -1.0 - 1.0 in view plane
  vec2 v1 = 2.0f * (inUV - 0.5f);

  // Pixel in view plane
  vec3 pvp = pc.vpmid + vec3(v1.x * pc.vpplusx.x + v1.y * pc.vpplusy.x,
                             v1.x * pc.vpplusx.y + v1.y * pc.vpplusy.y,
                             v1.x * pc.vpplusx.z + v1.y * pc.vpplusy.z);

  // Ray vector - from the eye through the view plane
  vec3 ray = pvp - pc.eye;

  // We probably should use bresenham - but I'll just scale to make
  // sure that none of rx/ry/rz are greater than a pixel
  ray = ray / length(ray);

  float result = 0.0;
  bool hitx = false;
  bool hity = false;
  bool hitz = false;
  bool hitedge = false;
  float lighting = 0.0;

  while (result <= 255.4 && !hitedge &&
         !(hitx=hitend(pvp.x, ray.x, vsize.x)) &&
         !(hity=hitend(pvp.y, ray.y, vsize.y)) &&
         !(hitz=hitend(pvp.z, ray.z, vsize.z))) {
    if (pvp.x >= 0.0f && pvp.x < vsize.x &&
        pvp.y >= 0.0f && pvp.y < vsize.y &&
        pvp.z >= 0.0f && pvp.z < vsize.z) {
      // OK, we've hit the voxel array
      ivec3 ipvp = ivec3(pvp.x, pvp.y, pvp.z);

      uint value = imageLoad(voxels, ipvp).r;
      if (value > 79) {
        hitedge = true;
        lighting = lightangle(pc.eye, pvp, pc.light);
      }
      result+= float(value/8.0);
    }
    pvp += ray;
  }

  if (result > 255.0) result=255.0;

  result = result / 255.0;

  f_color = vec4( hitedge?result:0,
                 lighting / 4.0,
                 hitedge ? 0.2:0,
                 1.0);
}
