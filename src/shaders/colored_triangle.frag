#version 450

// output write
layout (location = 0) out vec4 outFragColor;

//input read
layout (location = 0) in vec3 inColor;
layout (location = 1) in vec2 inUV;

void main() {
    outFragColor = vec4(inUV, 0., 1.0);
}