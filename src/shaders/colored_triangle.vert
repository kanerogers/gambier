#version 450

layout (push_constant) uniform block { 
    mat4 projection;
    mat4 view;
    mat4 model;
};
layout (location = 0) out vec3 outColor;
layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec3 inColour;

void main() {
    gl_Position = projection * view * model * vec4(inPosition, 1.0);
    outColor = normalize(inPosition.xyz); // just to make sure that vertex colours are working
}