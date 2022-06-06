#version 460
#include "common.glsl"

#extension GL_EXT_nonuniform_qualifier:enable


// Input
layout(std140, set = 0, binding = 2) readonly buffer MaterialBuffer {
    Material materials[];
} material_buffer;

// Textures
layout(set = 0, binding = 3) uniform sampler2D textures[];

layout (location = 0) in vec2 in_uv;
layout (location = 1) flat in uint in_material_id;

// Output
layout (location = 0) out vec4 out_colour;


void main(void) {
    Material material = material_buffer.materials[in_material_id];
    out_colour = texture(textures[nonuniformEXT(uint(material.base_colour_texture_id))], in_uv);
    out_colour.w = 1;
}