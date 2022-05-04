#version 460

layout (push_constant) uniform block { 
    mat4 projection;
    mat4 view;
};

struct ModelData {
    mat4 transform;
};

layout(std140, set= 1, binding = 0) readonly buffer ModelBuffer {
    ModelData models[];
} modelBuffer;

layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;

layout (location = 0) out vec2 outUV;

void main() {
    mat4 model = modelBuffer.models[gl_BaseInstance].transform;
    gl_Position = projection * view * model * vec4(inPosition, 1.0);
    outUV = inUV;
}