#version 300 es

in vec4 position;

void main() {
    gl_Position = vec4(position.xy, 0.0, 1.0);
}