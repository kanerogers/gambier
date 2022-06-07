#version 460
#include "common.glsl"

#extension GL_EXT_nonuniform_qualifier:enable


// Input
layout(std140, set = 0, binding = 2) readonly buffer MaterialBuffer {
    Material materials[];
} material_buffer;

// Textures
layout(set = 0, binding = 3) uniform sampler2D textures[];

// Input 
layout (location = 0) in vec3 inWorldPosition;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;
layout (location = 3) flat in uint inMaterialID;

// Output
layout (location = 0) out vec4 outColor;


void main(void) {
    Material material = material_buffer.materials[inMaterialID];
    vec4 baseColor;
    if (material.baseColorTextureID < 65535) {
        baseColor = texture(textures[nonuniformEXT(uint(material.baseColorTextureID))], inUV) * material.baseColorFactor;
    } else {
        baseColor = material.baseColorFactor;
    }

    outColor = baseColor;
    outColor.w = 1;
}