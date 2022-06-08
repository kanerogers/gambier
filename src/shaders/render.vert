#version 460
#include "common.glsl"

layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec3 inNormal;
layout (location = 2) in vec2 inUV;

layout (location = 0) out vec3 outWorldPosition;
layout (location = 1) out vec3 outNormal;
layout (location = 2) out vec2 outUV;
layout (location = 3) out uint outMaterialID;

void main() {
    DrawData draw_data = draw_data_buffer.draw_data[gl_DrawID];
    mat4 model = model_buffer.models[uint(draw_data.model_id)].transform;
    vec4 localPosition = model * vec4(inPosition, 1.0);

    // Set shader output variables
    outNormal = normalize(transpose(inverse(mat3(model))) * inNormal);
    outWorldPosition = localPosition.xyz / localPosition.w;
    outUV = inUV;
    outMaterialID = uint(draw_data.material_id);

    gl_Position = projection * view * localPosition;
}