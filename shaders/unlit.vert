#version 450

layout(binding = 0) uniform Transform {
    mat4 mvp;
} transform;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inTexCoords;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec4 color;

void main() {
    gl_Position = transform.mvp * vec4(inPosition, 1.0);
    // gl_Position = vec4(inPosition, 1.0);
    color = transform.mvp * vec4(inPosition, 1.0);
    fragTexCoord = inTexCoords.rg;
}
