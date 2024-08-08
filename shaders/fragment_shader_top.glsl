#version 300 es
precision lowp float;

uniform sampler2D u_current_state;
uniform vec2 u_resolution;
uniform int u_kernel_id;


out vec4 outColor;
