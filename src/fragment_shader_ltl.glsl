#version 300 es
precision lowp float;

uniform vec2 u_mouse;
uniform sampler2D u_current_state;
uniform vec2 u_resolution;

out vec4 outColor;

const int R = 5;
const int b1 = 34;
const int b2 = 45;
const int s1 = 34;
const int s2 = 58;
const int kernel_rows = 2*R+1;
const int kernel_length = kernel_rows*kernel_rows;

void main() {
    vec2 texCoord = gl_FragCoord.xy / u_resolution;
    vec2 texelSize = vec2(1.0) / u_resolution;

    // this is basically a convolution 2D
    int U = 0;
    for (int i = 0; i < kernel_length; i++) {
            vec2 offset = vec2(float(i % kernel_rows - R), float(i / kernel_rows - R)) * texelSize;
            vec4 neighbor = texture(u_current_state, texCoord + offset);
            U += int(neighbor.r>0.0);
    }
    // end of convolution 2D

    vec4 current = texture(u_current_state, texCoord);
    bool is_alive = current.r > 0.0;


    float new_state = float(is_alive) + float(int(U >= b1 && U <= b2) - int(U < s1 || U > s2));

    // clamp the new state to 0.0 or 1.0
    new_state = clamp(new_state, 0.0, 1.0);

    outColor = vec4(new_state * texCoord.x, new_state * texCoord.y, float(U)/121.0, 1.0);    
}
