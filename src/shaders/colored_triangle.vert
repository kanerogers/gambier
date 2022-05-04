#version 450

layout (push_constant) uniform block { 
    mat4 projection;
    mat4 view;
};
layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;

layout (location = 0) out vec3 outColor;
layout (location = 1) out vec2 outUV;

void main() {
    mat4 model = mat4(1); // TODO
    gl_Position = projection * view * vec4(inPosition, 1.0);
    outColor = normalize(inNormal + vec3(0., 0., 2.));
    outUV = inUV;
}