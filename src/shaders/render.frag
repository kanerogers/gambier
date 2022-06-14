#version 460
#include "common.glsl"

#define AMBIENT vec4(0.1)
#define SPECULAR_STRENGTH 0.5


// Input


// Input 
layout (location = 0) in vec3 inWorldPosition;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;
layout (location = 3) flat in uint inMaterialID;

// Output
layout (location = 0) out vec4 outColor;

vec4 blinnPhong(vec4 baseColor) {
    vec3 lightDir = normalize(lightPosition.xyz - inWorldPosition); 
    vec3 viewDir = normalize(cameraPosition.xyz - inWorldPosition);
    vec3 halfwayDir = normalize(lightDir + viewDir);

    float diffuseLight = max(dot(inNormal, lightDir), 0.0);
    float specularLight = SPECULAR_STRENGTH * pow(max(dot(halfwayDir, inNormal), 0.0), 16);

    return (AMBIENT + diffuseLight + specularLight) * baseColor;
}

void main(void) {
    // 0 - Base Colour
    Material material = material_buffer.materials[inMaterialID];
    vec4 baseColor;
    if (material.baseColorTextureID < 65535) {
        baseColor = texture(textures[nonuniformEXT(uint(material.baseColorTextureID))], inUV) * material.baseColorFactor;
    } else {
        baseColor = material.baseColorFactor;
    }

    // 1 - Normal
    if (material.unlit == 0) {
        outColor = blinnPhong(baseColor);
    } else {
        outColor = baseColor;
    }
    outColor.w = 1;
}