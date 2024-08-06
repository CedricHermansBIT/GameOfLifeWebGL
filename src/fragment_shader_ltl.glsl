#version 300 es
precision highp float;

uniform vec2 u_mouse;
uniform sampler2D u_current_state;
uniform vec2 u_resolution;

uniform int u_kernel[9];

out vec4 outColor;

float growth(int U) {
    int b1 = 34;
    int b2 = 45;
    int s1 = 34;
    int s2 = 58;
    return float(0 + int((U>=b1)&&(U<=b2)) - int((U<s1)||(U>s2)));
}

int convolve2D(sampler2D state, vec2 uv, int kernel[121], int kernel_rows, int R) {
    int sum = 0;
    for (int i = 0; i < kernel.length(); i++) {
        vec2 offset = vec2(float(i % kernel_rows - R), float(i / kernel_rows - R));
        vec2 wrapped_uv = mod(uv + offset + 1.0, 1.0); // Wrap around using mod
        vec4 neighbor = texture(state, wrapped_uv);
        if (neighbor.r > 0.0)
            sum += 1 * kernel[i];
    }
    return sum;
}
//pattern["bosco"] = {"name":"Bosco","R":5,"b1":34,"b2":45,"s1":34,"s2":58,
//  "cells":[[0,0,0,0,1,0,0,0,0,0], [0,0,1,1,1,1,1,0,0,0], [0,1,1,0,0,1,1,1,1,0], [1,1,0,0,0,1,1,1,1,1], [1,0,0,0,0,0,1,1,1,1], [1,1,0,0,0,0,1,1,1,1], [1,1,1,0,1,1,1,1,1,1], [0,1,1,1,1,1,1,1,1,0], [0,1,1,1,1,1,1,1,0,0], [0,0,1,1,1,1,1,0,0,0], [0,0,0,1,1,1,0,0,0,0]]
//}

void main() {
    vec2 texCoord = gl_FragCoord.xy / u_resolution;
    vec2 texelSize = vec2(1.0, 1.0) / u_resolution;

    //bosco pattern
    int R = 5;
    int b1 = 34;
    int b2 = 45;
    int s1 = 34;
    int s2 = 58;

    int kernel_rows = 2*R+1;
    int kernel_length = kernel_rows*kernel_rows;
    int kernel[121];
    for (int i = 0; i < (2*R+1)*(2*R+1); i++) {
        kernel[i] = 1;
    }

    // this is basically a convolution 2D
    int U = 0;
    for (int i = 0; i < kernel.length(); i++) {
            vec2 offset = vec2(float(i % kernel_rows - R), float(i / kernel_rows - R)) * texelSize;
            vec4 neighbor = texture(u_current_state, texCoord + offset);
            if (neighbor.r > 0.0) {
                U += 1 * kernel[i];
            }
    }
    // end of convolution 2D

    vec4 current = texture(u_current_state, texCoord);
    bool is_alive = current.r > 0.0;


    float new_state = float(is_alive) + float(0 + int((U>=b1)&&(U<=b2)) - int((U<s1)||(U>s2)));

    // clamp the new state to 0.0 or 1.0
    new_state = clamp(new_state, 0.0, 1.0);

    outColor = vec4(new_state * texCoord.x, new_state * texCoord.y, float(U)/121.0, 1.0);    

    float dist = distance(gl_FragCoord.xy, u_mouse);
    dist=10.0; // disable the mouse effect
    vec4 color = mix(outColor, vec4(1.0, 0.0, 0.0, 1.0), smoothstep(5.0,0.0, dist));
    outColor = color;
}
