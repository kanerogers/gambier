#version 450

layout (push_constant) uniform block { 
    mat4 projection;
    mat4 view;
};
layout (location = 0) out vec3 outColor;
layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec3 inColour;

void main() {
    gl_Position = projection * view * vec4(inPosition, 1.0);
    outColor = normalize(inColour.xyz + inPosition.xyz); // just to make sure that vertex colours are working
}