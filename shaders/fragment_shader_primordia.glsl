#version 300 es
precision lowp float;

uniform vec2 u_mouse;
uniform sampler2D u_current_state;
uniform vec2 u_resolution;

uniform float u_states;

out vec4 outColor;

const float kernel[9] = float[9](1.0/8.0, 1.0/8.0, 1.0/8.0,
                                  1.0/8.0, 0.0, 1.0/8.0,
                                  1.0/8.0, 1.0/8.0, 1.0/8.0);
const int R = 1;
const float b1 = 0.20;
const float b2 = 0.25;
const float s1 = 0.18;
const float s2 = 0.33;
const int kernel_rows = 2*R+1;
const int kernel_length = kernel_rows*kernel_rows;

float growth(float U) {
    return float(int(U >= b1 && U <= b2) - int(U <= s1 || U >= s2));
}

void main() {
    vec2 texCoord = gl_FragCoord.xy / u_resolution;
    vec2 texelSize = vec2(1.0) / u_resolution;

    // this is basically a convolution 2D
    float U = 0.0;
    for (int i = 0; i < kernel_length; i++) {
            vec2 offset = vec2(float(i % kernel_rows - R), float(i / kernel_rows - R)) * texelSize;
            vec2 wrappedCoord = texCoord + offset;
            wrappedCoord.x = (wrappedCoord.x < 0.0) ? wrappedCoord.x + 1.0 : ((wrappedCoord.x >= 1.0) ? wrappedCoord.x - 1.0 : wrappedCoord.x);
            wrappedCoord.y = (wrappedCoord.y < 0.0) ? wrappedCoord.y + 1.0 : ((wrappedCoord.y >= 1.0) ? wrappedCoord.y - 1.0 : wrappedCoord.y);
            vec4 neighbor = texture(u_current_state, wrappedCoord);
            U += neighbor.r * kernel[i];
    }
    // end of convolution 2D

    vec4 current = texture(u_current_state, texCoord) * u_states;

    float new_state = current.r + growth(U);

    // clamp the new state to 0.0 or 1.0
    new_state = clamp(new_state, 0.0, u_states)/u_states;

    outColor = vec4(new_state, new_state/2.0, U, 1.0);    
}
