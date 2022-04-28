#version 450

layout (location = 0) out vec3 outColor;
layout (location = 0) in vec4 position;

void main() {
    gl_Position = position;
    outColor = gl_Position.xyz;
}