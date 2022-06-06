#version 460

#extension GL_EXT_shader_16bit_storage:enable
#extension GL_EXT_shader_explicit_arithmetic_types_int16:enable

struct DrawData {
    uint16_t model_id;
    uint16_t material_id;
};

struct Material {
    uint16_t base_colour_texture_id;
};

struct ModelData {
    mat4 transform;
};

layout (push_constant) uniform globals { 
    mat4 projection;
    mat4 view;
};

layout(std140, set = 0, binding = 0) readonly buffer DrawDataBuffer {
    DrawData draw_data[];
} draw_data_buffer;

layout(std140, set = 0, binding = 1) readonly buffer ModelBuffer {
    ModelData models[];
} model_buffer;


layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;

layout (location = 0) out vec2 outUV;
layout (location = 1) out uint out_material_id;

void main() {
    DrawData draw_data = draw_data_buffer.draw_data[gl_DrawID];
    mat4 model = model_buffer.models[uint(draw_data.model_id)].transform;
    gl_Position = projection * view * model * vec4(inPosition, 1.0);
    outUV = inUV;
    out_material_id = uint(draw_data.material_id);
}