#version 450

// sampler
layout(set = 0, binding = 0) uniform sampler2D textures[255];

// output write
layout (location = 0) out vec4 outFragColor;

//input read
layout (location = 0) in vec2 inUV;

void main() {
    vec3 colour = texture(textures[0], inUV).xyz;
    outFragColor = vec4(colour, 1.0);
}