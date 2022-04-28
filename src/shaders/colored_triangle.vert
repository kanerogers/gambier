#version 450

layout (location = 0) out vec3 outColor;
layout (location = 0) in vec4 inPosition;
layout (location = 1) in vec4 inColour;

void main() {
    gl_Position = inPosition;
    outColor = normalize(inColour.xyz + inPosition.xyz); // just to make sure that vertex colours are working
}