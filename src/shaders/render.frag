#version 460

struct DrawData {
    uint model_id;
    uint material_id;
};

struct Material {
    uint base_colour_texture_id;
};

// Input
layout(std140, set = 0, binding = 0) readonly buffer DrawDataBuffer {
    DrawData draw_data[];
} draw_data_buffer;

// Input
layout(std140, set = 0, binding = 2) readonly buffer MaterialBuffer {
    Material materials[];
} material_buffer;

// Textures
layout(set = 0, binding = 3) uniform sampler2D textures[255];

layout (location = 0) in vec2 inUV;
layout (location = 1) in flat uint in_material_id;

// Output
layout (location = 0) out vec4 outFragColor;


void main() {
    Material material = material_buffer.materials[in_material_id];

    vec4 colour = texture(textures[material.base_colour_texture_id], inUV);
    colour.w = 1;
    outFragColor = colour;
}