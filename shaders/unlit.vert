#version 450

layout(push_constant) uniform Spatial  {
    mat4 mvp;
} spatial;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inTexCoords;
layout(location = 2) in vec3 inVertexColor;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec3 fragColor;

void main() {
    gl_Position = spatial.mvp * vec4(inPosition, 1.0);
    fragTexCoord = vec2(inTexCoords.x, 1.0 - inTexCoords.y);
    fragColor = inVertexColor;
}
