#version 300 es
precision lowp float;

uniform vec2 u_mouse;
uniform sampler2D u_current_state;
uniform vec2 u_resolution;

uniform int u_kernel[9];
uniform float u_states;

out vec4 outColor;

const int R = 1;
const float b1 = 0.20;
const float b2 = 0.25;
const float s1 = 0.18;
const float s2 = 0.33;
const int kernel_rows = 2*R+1;
const int kernel_length = kernel_rows*kernel_rows;

void main() {
    vec2 texCoord = gl_FragCoord.xy / u_resolution;
    vec2 texelSize = vec2(1.0) / u_resolution;

    float kernel_sum = 0.0;
    for (int i = 0; i < kernel_length; i++) {
        kernel_sum += float(u_kernel[i]);
    }
    float kernel_normalized[9];
    for (int i = 0; i < kernel_length; i++) {
        kernel_normalized[i] = float(u_kernel[i]) / kernel_sum;
    }

    // this is basically a convolution 2D
    float U = 0.0;
    for (int i = 0; i < kernel_length; i++) {
            vec2 offset = vec2(float(i % kernel_rows - R), float(i / kernel_rows - R)) * texelSize;
            vec4 neighbor = texture(u_current_state, texCoord + offset);
            U += neighbor.r * kernel_normalized[i];
    }
    // end of convolution 2D

    vec4 current = texture(u_current_state, texCoord) * u_states;


    float new_state = current.r + float(int(U >= b1 && U <= b2) - int(U < s1 || U > s2));

    // clamp the new state to 0.0 or 1.0
    new_state = clamp(new_state, 0.0, u_states)/u_states;

    outColor = vec4(new_state, new_state/2.0, float(U), 1.0);    
}
