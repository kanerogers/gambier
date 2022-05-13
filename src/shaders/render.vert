#version 460

layout (push_constant) uniform block { 
    mat4 projection;
    mat4 view;
};

struct ModelData {
    mat4 transform;
};

struct MeshData {
    uint material_id;
};

layout(std140, set= 0, binding = 0) readonly buffer ModelBuffer {
    ModelData models[];
} model_buffer;

layout(std140, set= 0, binding = 1) readonly buffer MeshBuffer {
    MeshData meshes[];
} mesh_buffer;

layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;

layout (location = 0) out vec2 outUV;
layout (location = 1) out uint out_material_id;

void main() {
    mat4 model = model_buffer.models[gl_BaseInstance].transform;
    gl_Position = projection * view * model * vec4(inPosition, 1.0);
    outUV = inUV;
    out_material_id = mesh_buffer.meshes[gl_DrawID].material_id;
}