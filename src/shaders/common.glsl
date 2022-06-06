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