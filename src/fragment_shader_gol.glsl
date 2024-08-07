#version 300 es
precision highp float;

uniform vec2 u_mouse;
uniform sampler2D u_current_state;
uniform vec2 u_resolution;

const float kernel[9] = float[9](1.0/8.0, 1.0/8.0, 1.0/8.0,
                                  1.0/8.0, 0.0, 1.0/8.0,
                                  1.0/8.0, 1.0/8.0, 1.0/8.0);
out vec4 outColor;

float growth(int U) {
    return float(0 + int(U==3) - int(U<2 || U>3));
}

void main() {
    vec2 texCoord = gl_FragCoord.xy / u_resolution;
    vec2 texelSize = vec2(1.0, 1.0) / u_resolution;

    int alive_neighbors = 0;

    for (int i = 0; i < 9; i++) {
            vec2 offset = vec2(float(i % 3 - 1), float(i / 3 - 1)) * texelSize;
            vec4 neighbor = texture(u_current_state, texCoord + offset);
            if (neighbor.r > 0.0) {
                alive_neighbors += 1 * int(kernel[i]*8.0);
            }
    }

    vec4 current = texture(u_current_state, texCoord);
    bool is_alive = current.r > 0.0;


    float new_state = float(is_alive) + growth(alive_neighbors);

    // clamp the new state to 0.0 or 1.0
    new_state = clamp(new_state, 0.0, 1.0);

    outColor = vec4(new_state * texCoord.x, new_state * texCoord.y, new_state, 1.0);    

    float dist = distance(gl_FragCoord.xy, u_mouse);
    vec4 color = mix(outColor, vec4(1.0, 0.0, 0.0, 1.0), smoothstep(5.0,0.0, dist));
    outColor = color;
}
