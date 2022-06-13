#extension GL_EXT_shader_16bit_storage:enable
#extension GL_EXT_shader_explicit_arithmetic_types_int16:enable
#extension GL_EXT_nonuniform_qualifier:enable

struct DrawData {
    uint16_t model_id;
    uint16_t material_id;
};

struct Material {
    vec4 baseColorFactor;
    uint16_t baseColorTextureID;
    uint16_t unlit; // boolean
};

// PERF: It may be beneficial to remove this indirection entirely and just add it all to DrawData.
//       This would involve cloning all the relevant "model" related data mesh.primitives.len() times,
//       but most models don't have THAT many "sets" of primitives, so it seems like a bad idea to
//       essentially optimise for that case. In any event it is probably many times cheaper to do a 
//       clone on the CPU than perform the extra memory fetch in the compute and vertex shader.
struct ModelData {
    mat4 transform;
    vec3 sphereCentre;
    float sphereRadius;
};

struct VkDrawIndexedIndirectCommand
{
	uint indexCount;
	uint instanceCount;
	uint firstIndex;
	int  vertexOffset;
	uint firstInstance;
};

layout (push_constant) uniform globals { 
    mat4 projection;
    mat4 view;
    vec4 cameraPosition;
    vec4 lightPosition;
};

layout(std140, set = 0, binding = 0) readonly buffer DrawDataBuffer {
    DrawData draw_data[];
} draw_data_buffer;

layout(std140, set = 0, binding = 1) readonly buffer ModelBuffer {
    ModelData models[];
} model_buffer;

layout(std140, set = 0, binding = 2) readonly buffer MaterialBuffer {
    Material materials[];
} material_buffer;

// layout(std430, set = 0, binding = 3) readonly buffer DrawCommandsBuffer {
//     VkDrawIndexedIndirectCommand draw_commands[];
// } draw_commands_buffer;